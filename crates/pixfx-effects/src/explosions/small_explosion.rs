//! Explosion: port of pixelfx `src/fx/explosions/small-explosion.js`.
//!
//! Beyond pixelfx: the `size` option is held to 0..=16, so a huge number cannot hang the particle loops.
//!
//! The particle recipe (fire puffs that cool into smoke, sparks, embers, burning brands, grit, rocks, scorch) is
//! adapted from Sprite Fusion's open-source explosion (https://www.spritefusion.com/pixel-art-vfx/explosion),
//! MIT License, Copyright (c) 2026 Hugo Duprez.

use core::f64::consts::FRAC_PI_2;

use pixfx_core::color::{Rgba, WHITE, mix, ramp, rgb, scale};
use pixfx_core::math::{Rng, TAU, clamp, ease, js_max, js_min, js_round, rand, smooth, to_uint32};
use pixfx_core::sim::{
    Bit, BitKind, Bits, Blazes, Flags, Kind, Lights, Particles, ParticlesOpts, Spawn, band,
};
use pixfx_core::{Effect, EffectState, Layer, Meta, Options, Painter};

use super::lib::{OUTLINE, Stone};

pub struct SmallExplosion;

pub static SMALL_EXPLOSION: SmallExplosion = SmallExplosion;

const META: Meta = Meta {
    id: "explosions/small-explosion",
    name: "Explosion",
    desc: "A ground blast: a white flash and shock rings, fire that cools into a rolling smoke column, sparks, burning brands that land and keep burning, tumbling rocks and a glowing scorch.",
    tags: &["explosion", "fire", "smoke", "debris", "impact", "one-shot"],
    size: (300, 240),
    origin: (150, 196),
    duration: 5.4,
    looping: false,
    fps: 30,
    seed: 7,
    state: None,
};

// meta.options
const PALETTE: [Rgba; 8] = [
    Rgba::hex24(0x4a1712),
    Rgba::hex24(0xc9281f),
    Rgba::hex24(0xf2561d),
    Rgba::hex24(0xff8a2a),
    Rgba::hex24(0xffc23d),
    Rgba::hex24(0xffe26b),
    Rgba::hex24(0xfff3a6),
    Rgba::hex24(0xffffff),
];
const STONE: Rgba = Rgba::hex24(0x4a3f69);
const SIZE: f64 = 1.0;
/// The largest explosion drawn (beyond pixelfx, which draws any size; see the module doc).
const MAX_SIZE: f64 = 16.0;

const D: f64 = META.duration;
const SOOT: Rgba = Rgba::hex24(0x0d0a12);
const SOOT2: Rgba = Rgba::hex24(0x17121d);
const WARM: Rgba = Rgba::hex24(0xffb060);

/// Stone colours from one body colour: a lit face, the body, a shaded face, and the dust it makes.
fn stone(c: Rgba) -> (Stone, Rgba) {
    let st = Stone {
        light: mix(scale(c, 1.35), WHITE, 0.08),
        body: c,
        shade: scale(c, 0.64),
    };
    (st, mix(scale(c, 1.2), rgb(150.0, 140.0, 150.0), 0.3))
}

// the floor under the blast: white-hot, cooling through the fire ramp to soot
fn scorch(p: &mut Painter, f: &[Rgba; 8], t: f64, rx0: f64, cool: f64, fade: f64) {
    if fade <= 0.0 {
        return;
    }
    let rx = rx0 * js_min(1.0, t / 0.08);
    let ry = 4.0;
    let mut heat = *f;
    heat.reverse();
    let n = heat.len() as f64;
    for y in 0..=2 {
        let y = f64::from(y);
        let mut x = libm::floor(-rx);
        while x <= libm::ceil(rx) {
            let (dx, dy) = ((x + 0.5) / rx, (y + 0.5) / ry);
            let d = libm::sqrt(dx * dx + dy * dy);
            if d <= 1.0 {
                let k = rand(&[x * 31.0 + y, 3.0]);
                let tt = libm::pow(clamp(1.0 - t / cool, 0.0, 1.0), 1.4) * (1.25 - d) - k * 0.35;
                if tt > 0.06 {
                    let i = js_min(n - 1.0, libm::floor((1.0 - js_min(1.0, tt)) * (n - 1.0)));
                    let c = heat.get(i as usize).copied().unwrap_or(Rgba::NONE);
                    p.dot(x, y, c, fade);
                    if (x as i64) & 3 == 0 {
                        p.glow(x + 0.5, y, 6.0, c, 0.18 * tt * fade);
                    }
                } else {
                    let a = if d < 0.95 - k * 0.3 { 1.0 } else { 1.0 - d };
                    p.dot(x, y, if d < 0.5 { SOOT } else { SOOT2 }, a * fade);
                }
            }
            x += 1.0;
        }
    }
}

// a rock: a lit side, a shaded side, an outline, spinning
fn rock(p: &mut Painter, b: &Bit, st: &Stone) {
    let r = 0.6 + b.s * 0.75;
    let fade = js_min(1.0, b.life / 0.5);
    let lx = libm::cos(b.a) * r * 0.4;
    let ly = libm::sin(b.a) * r * 0.4;
    p.shade(
        b.x - r - 1.0,
        b.y - r - 1.0,
        b.x + r + 1.0,
        b.y + r + 1.0,
        |x, y, out| {
            let (dx, dy) = (x - b.x, y - b.y);
            let d = libm::hypot(dx * 1.15, dy);
            if d > r + 0.9 {
                return None;
            }
            let lit = dx * lx + dy * ly;
            out.c = if d > r {
                OUTLINE
            } else if lit < -0.4 {
                st.light
            } else if lit > 0.6 {
                st.shade
            } else {
                st.body
            };
            out.a = fade;
            None
        },
    );
}

/// The white flash at the heart of the blast.
struct Flash {
    r: f64,
    life: f64,
    max: f64,
}

/// A shock ring racing out from `r0` to `r1`.
struct Ring {
    r0: f64,
    r1: f64,
    life: f64,
    max: f64,
    thick: f64,
    c: Rgba,
}

/// What the effect keeps on each rock: whether its landing already kicked up dust.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct Rock {
    dusted: bool,
}

impl Effect for SmallExplosion {
    fn meta(&self) -> &Meta {
        &META
    }
    fn spawn(&self, opts: &Options, seed: u64) -> Box<dyn EffectState> {
        let f: [Rgba; 8] = ramp(&opts.palette_or(&PALETTE));
        let s = opts.num_in("size", SIZE, 0.0, MAX_SIZE);
        let rad = 18.0 * s;
        let power = 0.9 * libm::pow(s, 0.8);
        let earth = 1.6;
        let (st, dust) = stone(opts.color_or("stone", STONE));
        let mut r = Rng::new(to_uint32(seed as f64));
        let mut ps = Particles::new(ParticlesOpts {
            ground: 0.0,
            sparks: vec![f[6], f[5], f[3], f[2]],
            ..ParticlesOpts::default()
        });
        let mut rocks: Bits<Rock> = Bits::new(0.0);
        let mut lights = Lights::new();
        let y = -rad * 0.3;

        // the fireball
        let flashes = vec![Flash {
            r: rad * 1.05,
            life: 0.06,
            max: 0.06,
        }];
        let rings = vec![
            Ring {
                r0: rad * 0.45,
                r1: rad * 2.7,
                life: 0.34,
                max: 0.34,
                thick: 3.0,
                c: f[5],
            },
            Ring {
                r0: rad * 0.3,
                r1: rad * 1.6,
                life: 0.22,
                max: 0.22,
                thick: 1.8,
                c: f[3],
            },
        ];
        lights.add(0.0, y, rad * 3.4, WARM, 0.6 * power, 0.7);
        lights.add(0.0, y, rad * 1.6, WHITE, 0.6, 0.08);
        for _ in 0..5 {
            let x = r.range(-2.5, 2.5);
            let py = y + r.range(-2.5, 2.5);
            let vx = r.range(-20.0, 20.0);
            let vy = r.range(-40.0, -10.0);
            let life = r.range(0.18, 0.3);
            let size = r.range(rad * 0.4, rad * 0.55);
            ps.spawn(
                &mut r,
                Spawn {
                    kind: Kind::Fire,
                    x,
                    y: py,
                    vx,
                    vy,
                    life,
                    col: None,
                    size,
                    delay: 0.0,
                    flags: Flags::NONE,
                },
            );
        }
        let mut i = 0.0;
        while i < js_round(28.0 * power) {
            let a = r.range(0.0, TAU);
            let d = r.range(0.0, rad * 0.55);
            let sp = r.range(40.0, 210.0) * power;
            let life = r.range(0.55, 1.15);
            let size = r.range(rad * 0.22, rad * 0.46);
            let flags = if r.chance(0.45) {
                Flags::SMOKES | Flags::BIG
            } else {
                Flags::NONE
            };
            ps.spawn(
                &mut r,
                Spawn {
                    kind: Kind::Fire,
                    x: libm::cos(a) * d,
                    y: y + libm::sin(a) * d,
                    vx: libm::cos(a) * sp,
                    vy: libm::sin(a) * sp * 0.8 - 40.0,
                    life,
                    col: None,
                    size,
                    delay: 0.0,
                    flags,
                },
            );
            i += 1.0;
        }
        for _ in 0..5 {
            let a = r.range(0.0, TAU);
            let d = r.range(rad * 0.1, rad * 0.5);
            let life = r.range(0.9, 1.5);
            let size = r.range(rad * 0.15, rad * 0.26);
            let delay = r.range(0.06, 0.2);
            ps.spawn(
                &mut r,
                Spawn {
                    kind: Kind::Smoke,
                    x: libm::cos(a) * d,
                    y: y + libm::sin(a) * d,
                    vx: libm::cos(a) * 30.0,
                    vy: libm::sin(a) * 20.0 - 30.0,
                    life,
                    col: Some(rgb(34.0, 29.0, 38.0)),
                    size,
                    delay,
                    flags: Flags::BIG,
                },
            );
        }
        for _ in 0..5 {
            let x = r.range(-rad * 0.25, rad * 0.25);
            let vx = r.range(-12.0, 12.0);
            let vy = r.range(-180.0, -100.0);
            let life = r.range(0.8, 1.3);
            let size = r.range(rad * 0.24, rad * 0.36);
            let delay = r.range(0.0, 0.1);
            let flags = if r.chance(0.6) {
                Flags::SMOKES | Flags::BIG
            } else {
                Flags::NONE
            };
            ps.spawn(
                &mut r,
                Spawn {
                    kind: Kind::Fire,
                    x,
                    y,
                    vx,
                    vy,
                    life,
                    col: None,
                    size,
                    delay,
                    flags,
                },
            );
        }
        let mut i = 0.0;
        while i < js_round(10.0 * power) {
            let a = r.range(0.0, TAU);
            let d = r.range(0.0, rad * 0.7);
            let sp = r.range(15.0, 100.0);
            let life = r.range(1.8, 2.8);
            let size = r.range(rad * 0.26, rad * 0.44);
            let delay = r.range(0.12, 0.45);
            ps.spawn(
                &mut r,
                Spawn {
                    kind: Kind::Smoke,
                    x: libm::cos(a) * d,
                    y: y + libm::sin(a) * d,
                    vx: libm::cos(a) * sp,
                    vy: libm::sin(a) * sp * 0.6 - 35.0,
                    life,
                    col: None,
                    size,
                    delay,
                    flags: Flags::BIG,
                },
            );
            i += 1.0;
        }
        let mut i = 0.0;
        while i < js_round(80.0 * power) {
            let a = r.range(0.0, TAU);
            let sp = r.range(140.0, 380.0) * power;
            let life = r.range(0.18, 0.6);
            ps.spawn(
                &mut r,
                Spawn::new(
                    Kind::Spark,
                    0.0,
                    y,
                    libm::cos(a) * sp,
                    libm::sin(a) * sp - 70.0,
                    life,
                ),
            );
            i += 1.0;
        }
        let mut i = 0.0;
        while i < js_round(40.0 * power) {
            let a = r.range(0.0, TAU);
            let sp = r.range(30.0, 150.0);
            let life = r.range(1.2, 2.6);
            ps.spawn(
                &mut r,
                Spawn {
                    col: Some(f[3]),
                    ..Spawn::new(
                        Kind::Ember,
                        libm::cos(a) * rad * 0.4,
                        y + libm::sin(a) * rad * 0.4,
                        libm::cos(a) * sp,
                        libm::sin(a) * sp - 75.0,
                        life,
                    )
                },
            );
            i += 1.0;
        }
        // JS re-evaluates the loop bound, drawing a new count before every brand
        let mut i = 0.0;
        while i < js_round(r.range(9.0, 14.0) * js_max(0.0, power - 0.55)) {
            let a = -FRAC_PI_2 + r.range(-1.35, 1.35);
            let sp = r.range(170.0, 330.0);
            let life = r.range(0.7, 1.3);
            ps.spawn(
                &mut r,
                Spawn::new(
                    Kind::Brand,
                    0.0,
                    y,
                    libm::cos(a) * sp,
                    libm::sin(a) * sp,
                    life,
                ),
            );
            i += 1.0;
        }
        // the ground's part: dust rolling out along the floor, rocks and grit thrown up, a dust column
        let mut i = 0.0;
        while i < js_round(rad * 0.5) {
            let s = r.sign();
            let x = s * r.range(0.0, rad * 0.6);
            let py = -r.range(1.0, 5.0 + rad * 0.04);
            let vx = s * r.range(3.0, 9.5) * rad;
            let vy = -r.range(2.0, 15.0);
            let life = r.range(1.1, 2.0);
            let size = r.range(4.0, 7.0) + rad / 6.0;
            let delay = r.range(0.0, 0.06);
            ps.spawn(
                &mut r,
                Spawn {
                    kind: Kind::Smoke,
                    x,
                    y: py,
                    vx,
                    vy,
                    life,
                    col: Some(dust),
                    size,
                    delay,
                    flags: Flags::BIG,
                },
            );
            i += 1.0;
        }
        let mut i = 0.0;
        while i < 26.0 * js_min(1.5, s) {
            let x = r.range(-0.4, 0.4) * rad;
            let vx = r.range(-9.0, 9.0) * rad * 0.25;
            let vy = -r.range(7.0, 16.0) * rad * 0.9;
            let life = r.range(1.8, 2.6);
            let a = r.range(0.0, TAU);
            let va = r.range(-16.0, 16.0);
            let size = r.int(1.0, 3.0);
            rocks.add(Bit {
                vx,
                vy,
                g: 1.0,
                drag: 0.4,
                bounce: 0.3,
                life,
                a,
                va,
                s: size,
                ..Bit::new(BitKind::Chunk, x, -3.0)
            });
            i += 1.0;
        }
        let mut i = 0.0;
        while i < js_round(earth * rad * 4.0) {
            let u = r.range(-1.0, 1.0);
            let a = -FRAC_PI_2 + u * 0.5;
            let sp = r.range(5.0, 15.0) * rad * (1.0 - 0.45 * libm::fabs(u));
            let life = r.range(1.2, 2.2);
            let col = r.pick(&[st.light, st.body, st.body, st.shade]).copied();
            let size = if r.chance(0.45) { 2.0 } else { 1.0 };
            ps.spawn(
                &mut r,
                Spawn {
                    col,
                    size,
                    ..Spawn::new(
                        Kind::Grit,
                        u * 0.25 * rad,
                        -1.0,
                        libm::cos(a) * sp,
                        libm::sin(a) * sp,
                        life,
                    )
                },
            );
            i += 1.0;
        }
        let mut i = 0.0;
        while i < js_round(earth * 12.0) {
            let x = r.range(-0.3, 0.3) * rad;
            let py = -r.range(2.0, rad * 0.6);
            let vx = r.range(-15.0, 15.0);
            let vy = -r.range(60.0, 190.0) * earth * 0.6;
            let life = r.range(1.6, 2.6);
            let size = r.range(5.0, 9.0) * libm::sqrt(s) + rad / 7.0;
            let delay = r.range(0.0, 0.15);
            ps.spawn(
                &mut r,
                Spawn {
                    kind: Kind::Smoke,
                    x,
                    y: py,
                    vx,
                    vy,
                    life,
                    col: Some(dust),
                    size,
                    delay,
                    flags: Flags::BIG,
                },
            );
            i += 1.0;
        }

        Box::new(State {
            f,
            s,
            rad,
            y,
            st,
            dust,
            r,
            ps,
            blazes: Blazes::new(),
            rocks,
            lights,
            flashes,
            rings,
            t: 0.0,
        })
    }
}

struct State {
    f: [Rgba; 8],
    s: f64,
    rad: f64,
    y: f64,
    st: Stone,
    dust: Rgba,
    r: Rng,
    ps: Particles,
    blazes: Blazes,
    rocks: Bits<Rock>,
    lights: Lights,
    flashes: Vec<Flash>,
    rings: Vec<Ring>,
    t: f64,
}

impl State {
    /// The shock band rolling out along the floor: its far half (`far`) behind, its near half in front.
    fn shock(&self, p: &mut Painter, far: bool) {
        let k = self.t / 0.5;
        if k < 1.0 {
            band(
                p,
                0.0,
                0.5,
                80.0 * self.s * ease::out_cubic(k),
                0.16,
                js_max(1.0, (2.0 + self.rad / 9.0) * (1.0 - k)),
                1.15 - k,
                &[self.st.body, self.st.light, self.f[3], self.f[6]],
                |g| (libm::sin(g) < 0.0) == far,
                false,
            );
        }
    }
}

impl EffectState for State {
    fn update(&mut self, dt: f64) {
        self.t += dt;
        for i in (0..self.flashes.len()).rev() {
            if let Some(f) = self.flashes.get_mut(i) {
                f.life -= dt;
                if f.life <= 0.0 {
                    self.flashes.remove(i);
                }
            }
        }
        for i in (0..self.rings.len()).rev() {
            if let Some(g) = self.rings.get_mut(i) {
                g.life -= dt;
                if g.life <= 0.0 {
                    self.rings.remove(i);
                }
            }
        }
        self.rocks.update(dt);
        // rocks that hit hard kick up dust
        let (r, ps, dust) = (&mut self.r, &mut self.ps, self.dust);
        for b in self.rocks.list_mut() {
            if b.rest && !b.data.dusted {
                b.data.dusted = true;
                for _ in 0..3 {
                    let x = b.bit.x + r.range(-1.5, 1.5);
                    let vx = r.sign() * r.range(10.0, 55.0) * 0.6;
                    let vy = -r.range(2.0, 22.0);
                    let life = r.range(0.3, 0.65);
                    let size = r.range(1.3, 2.8);
                    ps.spawn(
                        r,
                        Spawn {
                            col: Some(dust),
                            size,
                            ..Spawn::new(Kind::Dust, x, -0.5, vx, vy, life)
                        },
                    );
                }
            }
        }
        self.blazes.update(ps, r, dt);
        // a brand that lands starts a little fire where it fell
        let blazes = &mut self.blazes;
        ps.update_with(r, dt, |r, x, y| {
            let life = r.range(0.9, 1.5);
            let power = r.range(0.55, 0.85);
            blazes.add(r, x, y - 0.25, life, power);
        });
        self.lights.update(dt);
    }

    fn draw(&self, p: &mut Painter, _t: f64, layer: Layer) {
        let (t, f, y) = (self.t, &self.f, self.y);
        if layer == Layer::Back {
            scorch(
                p,
                f,
                t,
                22.0 * self.s,
                1.2 + self.rad / 14.0,
                1.0 - smooth(D - 1.2, D - 0.2, t),
            );
            self.ps.draw(p, Layer::Back);
            self.blazes.draw_back(p);
            self.shock(p, true);
            return;
        }
        self.shock(p, false);
        for b in self.rocks.list() {
            rock(p, &b.bit, &self.st);
        }
        self.ps.draw(p, Layer::Front);
        for fl in &self.flashes {
            let k2 = fl.life / fl.max;
            let r = fl.r * (0.8 + 0.2 * k2);
            p.circle(0.0, y, r + 1.2, f[2], 1.0);
            p.circle(0.0, y, r, if k2 > 0.45 { f[6] } else { f[5] }, 1.0);
            p.circle(0.0, y, r * 0.6, if k2 > 0.45 { WHITE } else { f[6] }, 1.0);
        }
        for g in &self.rings {
            let k2 = g.life / g.max;
            let e = 1.0 - libm::pow(k2, 3.0);
            p.ring(
                0.0,
                y,
                g.r0 + (g.r1 - g.r0) * e,
                js_max(1.0, g.thick * k2 + 0.4),
                g.c,
                js_min(1.0, k2 * 1.6),
            );
        }
        self.lights.draw(p);
    }
}
