//! Thunderclap: port of pixelfx `src/fx/lightning/thunderclap.js`.
//!
//! Beyond pixelfx: it fires an impact cue ([`pixfx_core::cue::impact`]) at the clap for the game to answer; the
//! `feel` option scales it, 0 for none. The acted-out caster and target (`act`) are gallery staging and are left
//! out. Wraps its actor ([`Effect::wraps_actor`]): the caster's body flashes white at the clap; with no sprite
//! handed to it, it reads pixelfx's stock 2x actor ([`super::lib::stand_in`]), as the pixelfx render does.
//!
//! The flash and flare, the bolts and their flicker, the plasma spray, the scorch where bolts land and the dust
//! thrown along the floor follow Sprite Fusion's open-source lightning and explosion
//! (https://www.spritefusion.com/pixel-art-vfx/lightning), MIT License, Copyright (c) 2026 Hugo Duprez.

use std::sync::Arc;

use pixfx_core::cue::{FEEL, TimedCue, impact};
use pixfx_core::sim::{Flags, Kind, Lights, Particles, ParticlesOpts, Spawn, band, flare, sparkle};
use pixfx_core::{Actor, Effect, EffectState, Layer, Meta, NumOpt, Options, RampOpt};
use pixfx_draw::Painter;
use pixfx_draw::color::{Rgba, WHITE, mix, ramp};
use pixfx_draw::math::{Rng, TAU, clamp, ease, js_max, rng, smooth, to_uint32};

use super::lib::{PLASMA, StrokeOpts, VOLT, crackle, scorch, stand_in, stroke, zigzag};

pub struct Thunderclap;

pub static THUNDERCLAP: Thunderclap = Thunderclap;

const META: Meta = Meta {
    id: "lightning/thunderclap",
    name: "Thunderclap",
    desc: "A thunderclap round the caster: a white flash, forked bolts burst out and stab the floor, a heavy shock ring tears across the ground raising dust and grit, sparks streak away.",
    tags: &[
        "lightning",
        "electric",
        "nova",
        "aoe",
        "spell",
        "shockwave",
        "one-shot",
    ],
    size: (288, 156),
    origin: (144, 124),
    duration: 1.9,
    looping: false,
    fps: 30,
    seed: 6,
    state: None,
    options: &[SPARKS.spec(), REACH.spec(), FEEL.spec()],
};

const SPARKS: RampOpt = RampOpt::new("sparks", &PLASMA);
/// How far the shock ring runs along the floor, px.
const REACH: NumOpt = NumOpt::new("reach", 104.0);

const PI: f64 = core::f64::consts::PI;
const CLAP: f64 = 0.12;
const CY: f64 = -22.0;
const BK: [f64; 6] = [1.0, 0.7, 0.95, 0.5, 0.8, 0.3];
const DUST: Rgba = Rgba::hex24(0x6e6382);
const STONE: [Rgba; 3] = [
    Rgba::hex24(0x655b7a),
    Rgba::hex24(0x4a4263),
    Rgba::hex24(0x342d45),
];
const ASH: Rgba = Rgba::hex24(0x5b5566);

impl Effect for Thunderclap {
    fn meta(&self) -> &Meta {
        &META
    }
    fn wraps_actor(&self) -> bool {
        true
    }
    /// Beyond pixelfx: the clap asks the game for a heavy impact's answer.
    fn cues(&self, opts: &Options) -> Vec<TimedCue> {
        impact(CLAP, 0.9, 0.0, CY, opts)
    }
    fn spawn(&self, opts: &Options, seed: u64) -> Box<dyn EffectState> {
        Box::new(State::new(opts, seed))
    }
}

/// pixelfx `spawn(k, x, y, vx, vy, life, col, size, delay, flag)`, its arguments taken in order.
#[expect(clippy::too_many_arguments, reason = "pixelfx spawn signature")]
fn sp(
    kind: Kind,
    x: f64,
    y: f64,
    vx: f64,
    vy: f64,
    life: f64,
    col: Option<Rgba>,
    size: f64,
    delay: f64,
    flags: Flags,
) -> Spawn {
    Spawn {
        kind,
        x,
        y,
        vx,
        vy,
        life,
        col,
        size,
        delay,
        flags,
    }
}

/// One of the eight bolts: the lower ones stab the floor, the upper ones tear into the air.
struct Bolt {
    down: bool,
    x1: f64,
    y1: f64,
    t0: f64,
}

struct State {
    c: [Rgba; 8],
    seed: f64,
    reach: f64,
    r: Rng,
    ps: Particles,
    hot: Particles,
    lights: Lights,
    bolts: Vec<Bolt>,
    landed: [bool; 8],
    t: f64,
    clapped: bool,
    actor: Option<Arc<Actor>>,
}

impl State {
    fn new(opts: &Options, seed: u64) -> Self {
        let c: [Rgba; 8] = ramp(&opts.palette_or(&VOLT));
        let sd = seed as f64;
        let mut r = Rng::new(to_uint32(sd));
        let ps = Particles::new(ParticlesOpts {
            ground: 0.0,
            sparks: opts.ramp(&SPARKS),
            ..ParticlesOpts::default()
        });
        let hot = Particles::new(ParticlesOpts {
            ground: 0.0,
            ..ParticlesOpts::default()
        });
        let bolts = (0..8)
            .map(|i| {
                let down = i % 2 == 0;
                let side = if i % 4 < 2 { -1.0 } else { 1.0 };
                let a = if down {
                    if side > 0.0 {
                        r.range(0.35, 0.75)
                    } else {
                        PI - r.range(0.35, 0.75)
                    }
                } else if side > 0.0 {
                    -r.range(0.15, 1.2)
                } else {
                    PI + r.range(0.15, 1.2)
                };
                let l = if down {
                    -CY / libm::sin(a)
                } else {
                    r.range(40.0, 62.0)
                };
                let x1 = libm::cos(a) * l;
                let y1 = if down { 0.0 } else { CY + libm::sin(a) * l };
                Bolt {
                    down,
                    x1,
                    y1,
                    t0: CLAP + r.range(0.0, 0.06),
                }
            })
            .collect();
        Self {
            c,
            seed: sd,
            reach: opts.num(&REACH),
            r,
            ps,
            hot,
            lights: Lights::new(),
            bolts,
            landed: [false; 8],
            t: 0.0,
            clapped: false,
            actor: stand_in(),
        }
    }

    fn front(&self, t: f64) -> f64 {
        if t < CLAP {
            0.0
        } else {
            8.0 + (self.reach - 8.0) * ease::out_cubic(clamp((t - CLAP) / 0.5, 0.0, 1.0))
        }
    }

    fn draw_back(&self, p: &mut Painter) {
        let (c, t) = (&self.c, self.t);
        // scorch where the bolts struck, and the far half of the shock ring
        for b in &self.bolts {
            if b.down && t >= b.t0 {
                scorch(
                    p,
                    b.x1,
                    0.0,
                    9.0,
                    libm::pow(clamp(1.0 - (t - b.t0) / 1.1, 0.0, 1.0), 1.3),
                    1.0 - smooth(1.4, 1.85, t),
                );
            }
        }
        let u = (t - CLAP) / 0.5;
        if u > 0.0 && u < 1.0 {
            band(
                p,
                0.0,
                0.5,
                self.front(t),
                0.18,
                js_max(2.0, 16.0 * (1.0 - u)),
                1.25 - u,
                &[c[1], c[2], c[4], c[6]],
                |g| libm::sin(g) < 0.0,
                u < 0.15,
            );
        }
        self.ps.draw(p, Layer::Back);
        self.hot.draw(p, Layer::Back);
    }

    fn draw_front(&self, p: &mut Painter) {
        let (c, t) = (&self.c, self.t);
        let f = libm::floor(t * 30.0 + 1e-4);
        let u = (t - CLAP) / 0.5;
        // the flash: the caster lit white for a frame, a flare, a core
        let a = t - CLAP;
        if (0.0..0.07).contains(&a)
            && let Some(actor) = &self.actor
        {
            p.mask(
                &**actor,
                f64::from(actor.x()),
                f64::from(actor.y()),
                if a < 0.035 { WHITE } else { c[6] },
                1.0,
            );
        }
        if (0.0..0.2).contains(&a) {
            let q = a / 0.2;
            flare(p, 0.0, CY, 44.0 * libm::pow(1.0 - q, 1.4), 1.0 - q, c);
            if q < 0.35 {
                p.circle(0.0, CY, 10.0 * (1.0 - q * 2.0) + 2.0, WHITE, 1.0);
            }
            p.glow(0.0, CY, 44.0, c[5], 0.32 * (1.0 - q));
        }
        // the bolts: struck, flickering, re-struck every other frame
        for (i, b) in self.bolts.iter().enumerate() {
            let ba = libm::floor((t - b.t0) * 30.0 + 1e-4);
            if t < b.t0 || ba >= BK.len() as f64 {
                continue;
            }
            let k = BK[ba as usize];
            let mut r = rng(&[self.seed, 3.0, i as f64, libm::floor(ba / 2.0)]);
            let bolt = zigzag(
                &mut r,
                0.0,
                CY,
                b.x1,
                b.y1,
                0.25,
                if b.down { 0.5 } else { 0.8 },
            );
            stroke(
                p,
                &bolt,
                k,
                c,
                if ba == 0.0 { 1.0 } else { 0.8 },
                StrokeOpts {
                    hero: ba == 0.0,
                    ..StrokeOpts::default()
                },
            );
            if b.down {
                p.circle(b.x1, -1.0, 3.0 * k + 1.0, c[5], 1.0);
                p.px(b.x1, -1.0, WHITE);
                p.glow(b.x1, -3.0, 20.0, c[5], 0.5 * k);
            } else if ba < 2.0 {
                sparkle(p, b.x1, b.y1, 4.0, c[6], c[4], 0.4);
            }
        }
        // the shock: the near half of the floor ring, and a ring in the air that bursts out from the clap
        if u > 0.0 && u < 1.0 {
            let fr = self.front(t);
            band(
                p,
                0.0,
                0.5,
                fr,
                0.18,
                js_max(2.0, 16.0 * (1.0 - u)),
                1.25 - u,
                &[c[1], c[2], c[4], c[6]],
                |g| libm::sin(g) >= 0.0,
                u < 0.15,
            );
            for s in [-1.0, 1.0] {
                p.glow(s * fr, -3.0, 28.0, c[4], 0.45 * (1.0 - u));
            }
            let ua = (t - CLAP) / 0.26;
            let ra = 8.0 + 60.0 * ease::out_cubic(ua);
            if ua < 1.0 {
                let col = if ua < 0.25 {
                    c[5]
                } else if ua < 0.6 {
                    c[4]
                } else {
                    c[3]
                };
                p.ring(
                    0.0,
                    CY,
                    ra,
                    js_max(1.0, 2.0 * (1.0 - ua)),
                    col,
                    |_v: f64, g: f64| {
                        if CY + libm::sin(g) * ra > -1.0 {
                            0.0
                        } else {
                            0.85 - 0.85 * ua
                        }
                    },
                );
            }
        }
        // the hum after: small arcs crackle round the caster as the charge bleeds away
        if a > 0.2 && a < 0.9 {
            let mut r = rng(&[self.seed, 9.0, f]);
            let k = 1.0 - (a - 0.2) / 0.7;
            if r.next() < 0.8 {
                let x0 = r.range(-14.0, 14.0);
                let y0 = r.range(-34.0, -4.0);
                let b = r.range(0.0, TAU);
                let l = r.range(6.0, 13.0);
                let bolt = zigzag(
                    &mut r,
                    x0,
                    y0,
                    x0 + libm::cos(b) * l,
                    y0 + libm::sin(b) * l,
                    0.4,
                    0.3,
                );
                crackle(p, &bolt, 0.85 * k + 0.1, c, 0.35, 1.0);
            }
        }
        self.ps.draw(p, Layer::Front);
        self.hot.draw(p, Layer::Front);
        self.lights.draw(p);
    }
}

impl EffectState for State {
    fn update(&mut self, dt: f64) {
        self.t += dt;
        let t = self.t;
        let c = self.c;
        let r = &mut self.r;
        // the breath before: sparks rush in to the caster's chest
        if t < CLAP && r.chance(60.0 * dt) {
            let a = r.range(0.0, TAU);
            let d = r.range(16.0, 30.0);
            let tt = r.range(0.07, 0.1);
            let v = (d * 3.0) / (libm::exp(3.0 * tt) - 1.0);
            let col = if r.chance(0.5) { c[6] } else { c[5] };
            let s = sp(
                Kind::Streak,
                libm::cos(a) * d,
                CY + libm::sin(a) * d * 0.8,
                -libm::cos(a) * v,
                -libm::sin(a) * v * 0.8,
                tt,
                Some(col),
                1.0,
                0.0,
                Flags::NONE,
            );
            self.ps.spawn(r, s);
        }
        if t >= CLAP && !self.clapped {
            self.clapped = true;
            self.lights.add(0.0, CY - 10.0, 100.0, c[4], 0.65, 0.45);
            self.lights.add(0.0, CY, 26.0, WHITE, 0.5, 0.07);
            for _ in 0..64 {
                let a = r.range(0.0, TAU);
                let v = r.range(170.0, 380.0) * if libm::sin(a) > 0.4 { 0.5 } else { 1.0 };
                let life = r.range(0.2, 0.55);
                let s = Spawn::new(
                    Kind::Spark,
                    libm::cos(a) * 6.0,
                    CY + libm::sin(a) * 6.0,
                    libm::cos(a) * v,
                    libm::sin(a) * v - 40.0,
                    life,
                );
                self.ps.spawn(r, s);
            }
            for _ in 0..8 {
                let x = r.range(-8.0, 8.0);
                let y = CY + r.range(-6.0, 6.0);
                let vx = r.range(-25.0, 25.0);
                let vy = -r.range(10.0, 30.0);
                let life = r.range(0.8, 1.3);
                let size = r.range(4.0, 7.0);
                let delay = r.range(0.05, 0.2);
                let s = sp(
                    Kind::Smoke,
                    x,
                    y,
                    vx,
                    vy,
                    life,
                    Some(mix(ASH, c[4], 0.3)),
                    size,
                    delay,
                    Flags::NONE,
                );
                self.ps.spawn(r, s);
            }
        }
        for (i, b) in self.bolts.iter().enumerate() {
            if b.down && !self.landed[i] && t >= b.t0 {
                self.landed[i] = true;
                self.lights.add(b.x1, -4.0, 46.0, c[4], 0.7, 0.25);
                for _ in 0..12 {
                    let a = -PI / 2.0 + r.range(-1.2, 1.2);
                    let v = r.range(70.0, 240.0);
                    let life = r.range(0.2, 0.5);
                    let s = Spawn::new(
                        Kind::Spark,
                        b.x1,
                        -1.0,
                        libm::cos(a) * v,
                        libm::sin(a) * v,
                        life,
                    );
                    self.ps.spawn(r, s);
                }
                for _ in 0..5 {
                    let x = b.x1 + r.range(-2.0, 2.0);
                    let vx = r.range(-80.0, 80.0);
                    let vy = -r.range(60.0, 180.0);
                    let life = r.range(0.4, 0.8);
                    self.hot
                        .spawn(r, Spawn::new(Kind::Spark, x, -1.0, vx, vy, life));
                }
            }
        }
        // the ring tears the floor up as it passes: dust rolling out, grit tossed
        let fr = if t < CLAP {
            0.0
        } else {
            8.0 + (self.reach - 8.0) * ease::out_cubic(clamp((t - CLAP) / 0.5, 0.0, 1.0))
        };
        if t > CLAP && t < CLAP + 0.5 {
            for s in [-1.0, 1.0] {
                if r.chance(70.0 * dt) {
                    let x = s * fr + r.range(-3.0, 3.0);
                    let y = -r.range(1.0, 4.0);
                    let vx = s * r.range(10.0, 40.0);
                    let vy = -r.range(5.0, 20.0);
                    let life = r.range(0.6, 1.1);
                    let size = r.range(3.0, 6.0) * (1.0 - 0.4 * (fr / self.reach));
                    let q = sp(
                        Kind::Smoke,
                        x,
                        y,
                        vx,
                        vy,
                        life,
                        Some(DUST),
                        size,
                        0.0,
                        Flags::BIG,
                    );
                    self.ps.spawn(r, q);
                }
                if r.chance(50.0 * dt) {
                    let vx = s * r.range(10.0, 60.0);
                    let vy = -r.range(60.0, 180.0);
                    let life = r.range(0.8, 1.3);
                    let col = r.pick(&STONE).copied();
                    let size = if r.chance(0.3) { 2.0 } else { 1.0 };
                    let q = sp(
                        Kind::Grit,
                        s * fr,
                        -1.0,
                        vx,
                        vy,
                        life,
                        col,
                        size,
                        0.0,
                        Flags::NONE,
                    );
                    self.ps.spawn(r, q);
                }
            }
        }
        self.ps.update(r, dt);
        self.hot.update(r, dt);
        self.lights.update(dt);
    }

    fn draw(&self, p: &mut Painter, _t: f64, layer: Layer) {
        match layer {
            Layer::Back => self.draw_back(p),
            Layer::Front => self.draw_front(p),
            Layer::Both => {
                self.draw_back(p);
                self.draw_front(p);
            }
        }
    }

    fn set_actor(&mut self, actor: Option<Arc<Actor>>) {
        self.actor = actor.or_else(stand_in);
    }
}
