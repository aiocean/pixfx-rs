//! Port of the parts of pixelfx `src/core/sim.js` that the ported effects use.
//!
//! Simulated particles, debris and lights, for effects that want real physics: sparks that streak and bounce
//! off the floor, shards that tumble and settle, vapour and smoke that swell and thin, lights that flare and die.
//! Adapted from Sprite Fusion's open-source pixel art VFX (https://www.spritefusion.com/pixel-art-vfx),
//! MIT License, Copyright (c) 2026 Hugo Duprez. Units here are art pixels and seconds; draw through a Painter,
//! so effects stay in local space and mirror or rotate like any other.

use crate::color::{Rgba, WHITE, mix, scale};
use crate::effect::Layer;
use crate::math::{Rng, clamp, js_max, js_min, js_round};
use crate::painter::Painter;
use core::ops::BitOr;

/// Downward acceleration of falling particles, px/s².
pub const GRAVITY: f64 = 950.0;
const CROSS: [(f64, f64); 4] = [(1.0, 0.0), (-1.0, 0.0), (0.0, 1.0), (0.0, -1.0)];
const DIAG: [(f64, f64); 4] = [(1.0, 1.0), (1.0, -1.0), (-1.0, 1.0), (-1.0, -1.0)];
const HOT: [Rgba; 4] = [
    Rgba::hex24(0xfff3a6),
    Rgba::hex24(0xffe26b),
    Rgba::hex24(0xff8a2a),
    Rgba::hex24(0xf2561d),
];

/// Fire from white-hot to smouldering: the ramp FIRE puffs walk down.
pub const FIRE_RAMP: [Rgba; 9] = [
    Rgba::hex24(0xffffff),
    Rgba::hex24(0xfff3a6),
    Rgba::hex24(0xffe26b),
    Rgba::hex24(0xffc23d),
    Rgba::hex24(0xff8a2a),
    Rgba::hex24(0xf2561d),
    Rgba::hex24(0xc9281f),
    Rgba::hex24(0x8a1f17),
    Rgba::hex24(0x4a1712),
];

/// Smoke from fresh to thin: the ramp smoke puffs walk down
/// (pixelfx `["#7b6f72", "#5d5660", "#433d48", "#2b2630", "#1a171e"].map(hex)`).
pub const SMOKE_RAMP: [Rgba; 5] = [
    Rgba(0x7b, 0x6f, 0x72, 255),
    Rgba(0x5d, 0x56, 0x60, 255),
    Rgba(0x43, 0x3d, 0x48, 255),
    Rgba(0x2b, 0x26, 0x30, 255),
    Rgba(0x1a, 0x17, 0x1e, 255),
];

/// Step `i` of a list, `Rgba::NONE` past its end or for a NaN or negative index (JS `undefined`).
fn step_at(list: &[Rgba], i: f64) -> Rgba {
    if i >= 0.0 {
        list.get(i as usize).copied().unwrap_or(Rgba::NONE)
    } else {
        Rgba::NONE
    }
}

/// pixelfx `rampAt`: the ramp step for t in 0..1.
fn ramp_at(r: &[Rgba], t: f64) -> Rgba {
    let n = r.len() as f64;
    step_at(r, js_min(n - 1.0, js_max(0.0, libm::floor(t * n))))
}

/// Particle kinds (pixelfx `P`, in its numeric order).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Spark,
    Mote,
    Streak,
    Flake,
    Vapour,
    Dust,
    Star,
    Ember,
    Smoke,
    Fire,
    Brand,
    Grit,
}

impl Kind {
    /// The soft kinds, drawn on the back layer (pixelfx `SOFT`).
    fn soft(self) -> bool {
        matches!(self, Kind::Vapour | Kind::Dust | Kind::Smoke)
    }
}

/// Particle flags (pixelfx `PF`): SMOKES (a FIRE puff leaves smoke when it dies), BIG (bigger, slower smoke),
/// BACK (draw behind).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Flags(pub u8);

impl Flags {
    pub const NONE: Flags = Flags(0);
    pub const SMOKES: Flags = Flags(1);
    pub const BIG: Flags = Flags(2);
    pub const BACK: Flags = Flags(4);

    pub fn contains(self, other: Flags) -> bool {
        self.0 & other.0 != 0
    }
}

impl BitOr for Flags {
    type Output = Flags;
    fn bitor(self, rhs: Flags) -> Flags {
        Flags(self.0 | rhs.0)
    }
}

/// The arguments of pixelfx `spawn(k, x, y, vx, vy, life, col, size, delay, flag)`. `col: None` is JS `col` 0
/// (FIRE and SMOKE use their own ramps); `delay` is seconds before the particle appears.
///
/// Rust evaluates a struct expression's `..base` last: when an extra field draws from the rng, write the full
/// literal in JS argument order instead of `..Spawn::new(..)`, or the draws reorder.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Spawn {
    pub kind: Kind,
    pub x: f64,
    pub y: f64,
    pub vx: f64,
    pub vy: f64,
    pub life: f64,
    pub col: Option<Rgba>,
    pub size: f64,
    pub delay: f64,
    pub flags: Flags,
}

impl Spawn {
    /// A spawn with pixelfx's defaults: white, size 1, no delay, no flags.
    pub fn new(kind: Kind, x: f64, y: f64, vx: f64, vy: f64, life: f64) -> Self {
        Self {
            kind,
            x,
            y,
            vx,
            vy,
            life,
            col: Some(WHITE),
            size: 1.0,
            delay: 0.0,
            flags: Flags::NONE,
        }
    }
}

/// Options of a particle system (pixelfx `opts` minus `rng` and `onLand`, which the update calls take).
#[derive(Clone, Debug, PartialEq)]
pub struct ParticlesOpts {
    /// The floor y in local px: sparks bounce on it, grit settles on it, brands land on it.
    pub ground: f64,
    /// SPARK and EMBER colours, head to tail.
    pub sparks: Vec<Rgba>,
    /// The colour vapour glows with.
    pub haze: Option<Rgba>,
}

impl Default for ParticlesOpts {
    fn default() -> Self {
        Self {
            ground: f64::INFINITY,
            sparks: HOT.to_vec(),
            haze: None,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
struct Particle {
    kind: Kind,
    x: f64,
    y: f64,
    vx: f64,
    vy: f64,
    life: f64,
    max: f64,
    /// `Rgba::NONE` for JS `col` 0.
    col: Rgba,
    size: f64,
    seed: f64,
    delay: f64,
    flags: Flags,
}

/// What one update step did to a particle.
enum Fate {
    Lives,
    Dies,
    /// A BRAND hit the floor at (x, ground).
    Lands(f64, f64),
}

/// One update step of `p` at time `t`: its fate and the particle it sheds (the smoke a dying FIRE leaves,
/// a BRAND's flame trail), whose rng draws for the arguments are already made.
fn advance(p: &mut Particle, rng: &mut Rng, t: f64, dt: f64, ground: f64) -> (Fate, Option<Spawn>) {
    if p.delay > 0.0 {
        p.delay -= dt;
        return (Fate::Lives, None);
    }
    p.life -= dt;
    if p.life <= 0.0 {
        let mut smoke = None;
        if p.kind == Kind::Fire && p.flags.contains(Flags::SMOKES) {
            let big = p.flags.contains(Flags::BIG);
            smoke = Some(Spawn {
                kind: Kind::Smoke,
                x: p.x,
                y: p.y,
                vx: p.vx * 0.6,
                vy: p.vy * 0.6 - 6.0,
                life: if big {
                    rng.range(1.3, 2.3)
                } else {
                    rng.range(0.8, 1.4)
                },
                col: None,
                size: p.size * if big { 0.85 } else { 0.7 },
                delay: 0.0,
                flags: if big { Flags::BIG } else { Flags::NONE },
            });
        }
        return (Fate::Dies, smoke);
    }
    match p.kind {
        Kind::Fire => {
            p.vy -= 95.0 * dt;
            p.vx += libm::sin(t * 17.0 + p.seed) * 30.0 * dt;
            p.vx -= p.vx * 5.0 * dt;
            p.vy -= p.vy * 3.2 * dt;
        }
        Kind::Grit => {
            p.vy += GRAVITY * dt;
            p.vx -= p.vx * 0.8 * dt;
            let ny = p.y + p.vy * dt;
            p.x += p.vx * dt;
            if p.vy > 0.0 && p.y <= ground && ny >= ground {
                p.y = ground - 0.25;
                if p.vy > 130.0 && rng.chance(0.4) {
                    p.vy = -p.vy * 0.25;
                    p.vx = p.vx * 0.5 + rng.range(-10.0, 10.0);
                } else {
                    p.vx = 0.0;
                    p.vy = 0.0;
                    p.life = js_min(p.life, 0.5);
                }
            } else {
                p.y = ny;
            }
            return (Fate::Lives, None);
        }
        Kind::Brand => {
            // a burning chunk: arcs, trails flame, and starts a little fire where it lands
            p.vy += GRAVITY * 0.8 * dt;
            p.vx -= p.vx * 0.9 * dt;
            let mut trail = None;
            if rng.chance(0.9 * js_min(1.0, dt * 60.0)) {
                trail = Some(Spawn {
                    kind: Kind::Fire,
                    x: p.x,
                    y: p.y,
                    vx: p.vx * 0.1 + rng.range(-7.0, 7.0),
                    vy: p.vy * 0.1 - 10.0,
                    life: rng.range(0.16, 0.32),
                    col: None,
                    size: rng.range(0.9, 1.7) * (0.5 + (p.life / p.max) * 0.6),
                    delay: 0.0,
                    flags: if rng.chance(0.4) {
                        Flags::SMOKES
                    } else {
                        Flags::NONE
                    },
                });
            }
            let ny = p.y + p.vy * dt;
            p.x += p.vx * dt;
            if p.vy > 0.0 && p.y <= ground && ny >= ground {
                return (Fate::Lands(p.x, ground), trail);
            }
            p.y = ny;
            return (Fate::Lives, trail);
        }
        Kind::Spark => {
            p.vy += GRAVITY * 0.45 * dt;
            p.vx -= p.vx * 2.2 * dt;
            p.vy -= p.vy * 2.2 * dt;
            let ny = p.y + p.vy * dt;
            p.x += p.vx * dt;
            if p.vy > 0.0 && p.y <= ground && ny >= ground {
                p.vy = -p.vy * 0.45;
                p.vx *= 0.7;
                p.y = ground - 0.25;
            } else {
                p.y = ny;
            }
            return (Fate::Lives, None);
        }
        Kind::Streak => {
            p.vx += p.vx * 3.0 * dt;
            p.vy += p.vy * 3.0 * dt;
        }
        Kind::Flake => {
            p.vx -= p.vx * 2.2 * dt;
            p.vy -= p.vy * 2.2 * dt - 35.0 * dt;
            p.x += libm::sin(t * 2.5 + p.seed) * 7.0 * dt;
        }
        Kind::Ember => {
            p.vy += (-30.0 + libm::sin(t * 3.0 + p.seed) * 20.0) * dt;
            p.vx += libm::cos(t * 2.3 + p.seed) * 45.0 * dt;
            p.vx -= p.vx * 1.6 * dt;
            p.vy -= p.vy * 1.6 * dt;
        }
        Kind::Smoke => {
            let big = p.flags.contains(Flags::BIG);
            p.vy -= if big { 23.0 } else { 16.0 } * dt;
            p.vx += libm::sin(t * 1.3 + p.seed) * 8.0 * dt;
            p.vx -= p.vx * 1.6 * dt;
            p.vy -= p.vy * 1.2 * dt;
        }
        Kind::Mote => {
            p.vx -= p.vx * 0.6 * dt;
            p.vy -= p.vy * 0.6 * dt;
        }
        Kind::Vapour | Kind::Dust | Kind::Star => {
            let drag = if p.kind.soft() { 4.0 } else { 1.6 };
            p.vx -= p.vx * drag * dt;
            p.vy -= p.vy * drag * dt + 5.0 * dt;
        }
    }
    p.x += p.vx * dt;
    p.y += p.vy * dt;
    (Fate::Lives, None)
}

/// A particle system (pixelfx `Particles`). The rng is the caller's: pixelfx effects hand theirs in as
/// `opts.rng` and keep drawing from it, so every call that draws takes it.
#[derive(Clone, Debug, PartialEq)]
pub struct Particles {
    list: Vec<Particle>,
    t: f64,
    ground: f64,
    sparks: Vec<Rgba>,
    haze: Option<Rgba>,
}

impl Particles {
    pub fn new(opts: ParticlesOpts) -> Self {
        Self {
            list: Vec::new(),
            t: 0.0,
            ground: opts.ground,
            sparks: opts.sparks,
            haze: opts.haze,
        }
    }

    /// Add a particle; draws its seed after the caller's argument draws, as pixelfx.
    pub fn spawn(&mut self, rng: &mut Rng, s: Spawn) {
        self.list.push(Particle {
            kind: s.kind,
            x: s.x,
            y: s.y,
            vx: s.vx,
            vy: s.vy,
            life: s.life,
            max: s.life,
            col: s.col.unwrap_or(Rgba::NONE),
            size: s.size,
            seed: rng.next() * 100.0,
            delay: s.delay,
            flags: s.flags,
        });
    }

    pub fn count(&self) -> usize {
        self.list.len()
    }

    /// Advance every particle by `dt` seconds.
    pub fn update(&mut self, rng: &mut Rng, dt: f64) {
        self.update_with(rng, dt, |_, _, _| {});
    }

    /// [`Particles::update`], calling `on_land(rng, x, y)` the moment a BRAND hits the floor (pixelfx `onLand`).
    pub fn update_with(
        &mut self,
        rng: &mut Rng,
        dt: f64,
        mut on_land: impl FnMut(&mut Rng, f64, f64),
    ) {
        self.t += dt;
        // walk backwards; particles shed during the walk go on the end and wait for the next update
        for i in (0..self.list.len()).rev() {
            let Some(p) = self.list.get_mut(i) else {
                continue;
            };
            let (fate, shed) = advance(p, rng, self.t, dt, self.ground);
            if let Some(s) = shed {
                self.spawn(rng, s);
            }
            match fate {
                Fate::Lives => {}
                Fate::Dies => {
                    self.list.remove(i);
                }
                Fate::Lands(x, y) => {
                    on_land(rng, x, y);
                    self.list.remove(i);
                }
            }
        }
    }

    /// Draw the particles of `layer`: [`Layer::Back`] the soft kinds (vapour, dust, smoke) and those flagged
    /// BACK, [`Layer::Front`] the rest, [`Layer::Both`] all of them.
    pub fn draw(&self, p: &mut Painter, layer: Layer) {
        for q in &self.list {
            if q.delay > 0.0 {
                continue;
            }
            let back = q.kind.soft() || q.flags.contains(Flags::BACK);
            match layer {
                Layer::Back if !back => continue,
                Layer::Front if back => continue,
                _ => {}
            }
            self.draw_one(p, q);
        }
    }

    fn draw_one(&self, p: &mut Painter, q: &Particle) {
        let (k, x, y) = (q.life / q.max, q.x, q.y);
        let h = libm::hypot(q.vx, q.vy);
        let sp = if h > 0.0 { h } else { 1.0 };
        let (ux, uy) = (q.vx / sp, q.vy / sp);
        let spark = |i: usize| self.sparks.get(i).copied().unwrap_or(Rgba::NONE);
        let spark_tail = self.sparks.last().copied().unwrap_or(Rgba::NONE);
        match q.kind {
            Kind::Spark => {
                let head = if k > 0.6 {
                    spark(0)
                } else if k > 0.3 {
                    spark(1)
                } else {
                    spark(2)
                };
                let len = js_min(9.0, 1.0 + sp * 0.022);
                let c = if k > 0.5 { head } else { spark_tail };
                p.line(
                    x - ux * len,
                    y - uy * len,
                    x - ux * len * 0.4,
                    y - uy * len * 0.4,
                    c,
                    1.0,
                );
                p.line(x - ux * len * 0.45, y - uy * len * 0.45, x, y, head, 1.0);
                p.glow(x, y, 3.0, head, 0.45 * k);
            }
            Kind::Mote => {
                let len = js_min(16.0, 0.5 + sp * 0.026) * q.size;
                let a = if k < 0.35 { k / 0.35 } else { 1.0 };
                p.line(
                    x - ux * len,
                    y - uy * len,
                    x - ux * len * 0.35,
                    y - uy * len * 0.35,
                    scale(q.col, 0.55),
                    a,
                );
                p.line(x - ux * len * 0.4, y - uy * len * 0.4, x, y, q.col, a);
                p.glow(x, y, 2.5 + q.size, q.col, 0.4 * k);
            }
            Kind::Streak => {
                let l = clamp(sp / 30.0, 1.0, 7.0);
                let mut d = l;
                while d >= 0.0 {
                    let c = if d < 1.0 {
                        WHITE
                    } else if d < l * 0.5 {
                        q.col
                    } else {
                        scale(q.col, 0.6)
                    };
                    p.dot(
                        x - ux * d,
                        y - uy * d,
                        c,
                        if d < l * 0.5 { 1.0 } else { 1.2 - d / l },
                    );
                    d -= 0.5;
                }
                p.glow(x, y, 4.0, q.col, 0.4);
            }
            Kind::Flake => {
                let a = js_min(1.0, k * 3.0);
                p.dot(x, y, q.col, a);
                if q.size > 1.3 {
                    for (dx, dy) in CROSS {
                        p.dot(x + dx, y + dy, scale(q.col, 0.8), a * 0.8);
                    }
                }
            }
            Kind::Vapour => {
                let r = q.size * (0.6 + (1.0 - k) * 0.9);
                p.disc(x, y, r, q.col, libm::pow(k, 1.3) * 0.45);
                if let Some(haze) = self.haze {
                    p.glow(x, y, r * 1.8, haze, 0.16 * k);
                }
            }
            Kind::Dust => {
                p.disc(
                    x,
                    y,
                    q.size * (0.6 + (1.0 - k) * 0.9),
                    q.col,
                    libm::pow(k, 1.3) * 0.7,
                );
            }
            Kind::Smoke => {
                // three soft discs: a body, a denser core sitting low, a lit cap while it's young
                let (age, big) = (1.0 - k, q.flags.contains(Flags::BIG));
                let r = q.size
                    * (0.65 + js_min(1.0, age * 2.5) * 0.45 + age * if big { 0.5 } else { 0.4 });
                let a =
                    js_min(1.0, age * 12.0) * js_min(1.0, k * 2.4) * if big { 0.7 } else { 0.58 };
                let c = if q.col != Rgba::NONE {
                    mix(q.col, ramp_at(&SMOKE_RAMP, age), age)
                } else {
                    ramp_at(&SMOKE_RAMP, age * 1.05)
                };
                p.disc(x, y, r, c, a * 0.45);
                p.disc(x + r * 0.05, y + r * 0.06, r * 0.74, c, a * 0.62);
                if age < 0.5 {
                    p.disc(
                        x - r * 0.22,
                        y - r * 0.3,
                        r * 0.5,
                        mix(c, WHITE, 0.12),
                        a * 0.4 * (1.0 - age * 2.0),
                    );
                }
            }
            Kind::Fire => {
                // a ball of flame that cools down the ramp as it rises: a darker rim, a lit core, a tongue on top
                let age = 1.0 - k;
                let heat = js_min(0.999, libm::pow(age, 0.62) + (q.seed % 1.0) * 0.1);
                let n = FIRE_RAMP.len() as f64;
                let idx = js_min(n - 2.0, libm::floor(heat * (n - 1.0)));
                let r = q.size * (0.55 + 0.45 * js_min(1.0, age * 7.0)) * (1.0 - age * 0.55);
                let c = step_at(&FIRE_RAMP, idx);
                if idx >= n - 2.0 {
                    p.disc(x, y, r, c, js_min(1.0, k * 3.0) * 0.85);
                } else {
                    p.circle(
                        x,
                        y,
                        r,
                        step_at(&FIRE_RAMP, js_min(n - 1.0, idx + 2.0)),
                        1.0,
                    );
                    p.circle(x - r * 0.16, y - r * 0.22, r * 0.78, c, 1.0);
                    if r > 1.6 {
                        p.circle(x, y - r * 0.9, r * 0.42, c, 1.0);
                    }
                    if idx <= 3.0 && r > 1.8 {
                        let tip = step_at(&FIRE_RAMP, js_max(0.0, idx - 1.0));
                        p.circle(x - r * 0.28, y - r * 0.34, r * 0.36, tip, 1.0);
                    }
                    if idx < 6.0 {
                        p.glow(x, y, r * 2.4 + 2.0, c, 0.18 * k);
                    }
                }
            }
            Kind::Brand => {
                p.circle(x, y, 1.6, HOT[3], 1.0);
                p.px(
                    x,
                    y,
                    if libm::sin(self.t * 40.0 + q.seed) > 0.0 {
                        HOT[0]
                    } else {
                        HOT[1]
                    },
                );
                p.px(x + 1.0, y, HOT[1]);
                p.glow(x, y, 7.0, HOT[2], 0.45);
            }
            Kind::Grit => {
                if k > 0.25 || k * 4.0 > 0.5 {
                    p.px(x, y, q.col);
                    if q.size >= 2.0 {
                        p.px(x + 1.0, y, q.col);
                        p.px(x, y + 1.0, scale(q.col, 0.8));
                        p.px(x + 1.0, y + 1.0, scale(q.col, 0.7));
                    }
                }
            }
            Kind::Ember => {
                let c = if k > 0.6 {
                    spark(0)
                } else if k > 0.3 {
                    spark(1)
                } else {
                    spark(2)
                };
                if k < 0.25 && libm::sin(self.t * 40.0 + q.seed) < 0.0 {
                    return;
                }
                p.px(x, y, c);
                if q.size > 1.5 {
                    p.px(x, y + 1.0, spark_tail);
                }
                p.glow(x, y, 3.0 + q.size, c, 0.35 * k);
            }
            Kind::Star => {
                // a twinkling cross, arms set by size
                let tw = 0.6 + 0.4 * libm::sin(self.t * (8.0 + (q.seed % 5.0)) + q.seed);
                if q.size < 1.0 {
                    if tw * k > 0.25 {
                        p.px(x, y, if tw > 0.8 { WHITE } else { q.col });
                    }
                    p.glow(x, y, 3.0, q.col, 0.4 * k * tw);
                } else {
                    sparkle(
                        p,
                        x,
                        y,
                        q.size * tw * js_min(1.0, k * 3.0),
                        q.col,
                        scale(q.col, 0.75),
                        0.5 * k * tw,
                    );
                }
            }
        }
    }
}

/// A star glint: arms of length `arm` in colour `c` with tips in `tip`, a white heart, light `k` (pixelfx
/// `sparkle`; its defaults are `tip = c`, `k = 0.6`).
pub fn sparkle(p: &mut Painter, x: f64, y: f64, arm: f64, c: Rgba, tip: Rgba, k: f64) {
    let n = js_round(arm);
    if n < 1.0 {
        return;
    }
    let (cx, cy) = (libm::floor(x), libm::floor(y));
    let mut d = 1.0;
    while d <= n {
        let cc = if d > n * 0.6 { tip } else { c };
        for (dx, dy) in CROSS {
            p.px(cx + dx * d, cy + dy * d, cc);
        }
        d += 1.0;
    }
    if n >= 3.0 {
        for (dx, dy) in DIAG {
            p.px(cx + dx, cy + dy, c);
        }
    }
    if n >= 6.0 {
        for (dx, dy) in DIAG {
            p.px(cx + dx * 2.0, cy + dy * 2.0, tip);
        }
    }
    p.px(cx, cy, WHITE);
    p.glow(x, y, 4.0 + n * 2.0, c, k);
}
