//! Icicle fall: port of pixelfx `src/fx/ice/icicle-fall.js`.
//!
//! The break recipe (a white frame, the ice flung apart as shards with ice dust and glints) is adapted from
//! Sprite Fusion's open-source ice (https://www.spritefusion.com/pixel-art-vfx/ice), MIT License,
//! Copyright (c) 2026 Hugo Duprez.

use core::f64::consts::{FRAC_PI_2, PI};

use pixfx_core::color::{Rgba, WHITE, ramp};
use pixfx_core::math::{
    Rng, TAU, clamp, ease, js_max, js_min, js_round, js_sign, rand, seg, smooth, to_uint32,
};
use pixfx_core::sim::{
    Bit, BitKind, Bits, GRAVITY, Kind, Lights, Particles, ParticlesOpts, Spawn, band, sparkle,
};
use pixfx_core::{Effect, EffectState, Layer, Meta, Options, Painter};

use super::lib::{Pane, PaneEvent, Panes, centroid, end_by, split_poly};

pub struct IcicleFall;

pub static ICICLE_FALL: IcicleFall = IcicleFall;

const META: Meta = Meta {
    id: "ice/icicle-fall",
    name: "Icicle fall",
    desc: "An icicle drips and trembles, snaps off its frosted root, drops, stabs the floor and bursts into skidding shards, splinters and rolling frost.",
    tags: &["ice", "icicle", "fall", "trap", "shatter", "one-shot"],
    size: (152, 120),
    origin: (76, 100),
    duration: 2.6,
    looping: false,
    fps: 30,
    seed: 4,
    state: None,
    options: &[],
};

// meta.options
const PALETTE: [Rgba; 8] = [
    Rgba::hex24(0x151a3d),
    Rgba::hex24(0x212f6b),
    Rgba::hex24(0x2c58aa),
    Rgba::hex24(0x3a8bd8),
    Rgba::hex24(0x62c8f4),
    Rgba::hex24(0xa6ecff),
    Rgba::hex24(0xdcfbff),
    Rgba::hex24(0xffffff),
];

const D: f64 = META.duration;
// where it hangs, how long, how wide at the root
const ROOT: f64 = -80.0;
const LEN: f64 = 36.0;
const W: f64 = 12.0;
const DRIP: f64 = 0.16;
const TREMBLE: f64 = 0.3;
const SNAP: f64 = 0.6;
const FREE: f64 = SNAP + 0.04;
const FLOOR: f64 = 0.0;

/// How long the icicle takes to fall from its root to the floor.
fn fall_t() -> f64 {
    libm::sqrt((2.0 * (-ROOT - LEN)) / GRAVITY)
}

fn impact() -> f64 {
    FREE + fall_t()
}

/// The icicle: a rippled taper hanging from (x, top), lit down its left side with a bright core streak, rings of
/// light at the ripples, deep on the right, a dark rim. `white` flashes it; `glint` (0..1, or -1) is a highlight
/// across it at that height. (pixelfx's `tilt` and `cut` options stay at their defaults here.)
struct Icicle {
    white: bool,
    glint: f64,
}

fn icicle(p: &mut Painter, c: &[Rgba; 8], x: f64, top: f64, len: f64, w: f64, o: &Icicle) {
    let hw = |u: f64| {
        (w / 2.0)
            * libm::pow(1.0 - u, 0.75)
            * (1.0 + 0.14 * libm::sin(u * 13.0 + 0.5))
            * if u < 0.08 { 1.15 } else { 1.0 }
            + 0.3
    };
    p.shade(
        x - w / 2.0 - 4.0,
        top - 1.0,
        x + w / 2.0 + 4.0,
        top + len + 1.0,
        |px, py, out| {
            let u = (py - top) / len;
            let cx = x + 0.7 * libm::sin(u * 6.0 + 1.0);
            let s = px - cx;
            if u < 0.0 || u > 1.0 {
                return None;
            }
            let h = hw(u);
            if libm::fabs(s) > h + 1.0 {
                return None;
            }
            if libm::fabs(s) > h {
                out.c = c[1];
                return None;
            }
            if o.white {
                return Some(WHITE);
            }
            let q = s / h;
            let ring = libm::sin(u * 13.0 + 0.5) > 0.7;
            let mut col = if u > 0.88 {
                WHITE
            } else if q < -0.72 {
                c[6]
            } else if q < -0.2 {
                if ring { c[7] } else { c[5] }
            } else if q < 0.18 {
                if u < 0.72 { c[7] } else { c[6] }
            } else if q > 0.8 {
                c[2]
            } else if libm::fabs(q - 0.45) < 0.1 && u > 0.15 && u < 0.6 {
                c[5]
            } else if q < 0.6 {
                c[4]
            } else {
                c[3]
            };
            if o.glint >= 0.0 && q < 0.25 && libm::fabs(u - o.glint) < 0.06 {
                col = WHITE;
            }
            out.c = col;
            None
        },
    );
}

/// One bit of the rime it hangs from: a frosted crust with stubs of older icicles.
#[derive(Clone, Copy, Debug)]
struct Rime {
    x: f64,
    h: f64,
    stub: f64,
}

impl Effect for IcicleFall {
    fn meta(&self) -> &Meta {
        &META
    }
    fn spawn(&self, opts: &Options, seed: u64) -> Box<dyn EffectState> {
        let c: [Rgba; 8] = ramp(&opts.palette_or(&PALETTE));
        let seed = seed as f64;
        let r = Rng::new(to_uint32(seed));
        let ps = Particles::new(ParticlesOpts {
            ground: FLOOR,
            sparks: vec![c[7], c[6], c[5], c[4]],
            haze: Some(c[5]),
        });
        // the rime it hangs from
        let mut rime = Vec::new();
        let mut x = -18.0;
        while x <= 18.0 {
            let h = rand(&[x, seed, 2.0]);
            let d = libm::fabs(x) / 18.0;
            rime.push(Rime {
                x,
                h: 1.0 + js_round((0.6 + h * 1.8) * (1.0 - d * 0.7)),
                stub: if h < 0.3 && libm::fabs(x) > 7.0 {
                    2.0 + js_round(rand(&[x, 5.0]) * 5.0 * (1.0 - d * 0.5))
                } else {
                    0.0
                },
            });
            x += 1.0;
        }
        Box::new(State {
            c,
            r,
            ps,
            bits: Bits::new(FLOOR),
            lights: Lights::new(),
            panes: Panes::new(FLOOR),
            rime,
            t: 0.0,
            snapped: false,
            landed: false,
            dripped: false,
        })
    }
}

struct State {
    c: [Rgba; 8],
    r: Rng,
    ps: Particles,
    bits: Bits<()>,
    lights: Lights,
    panes: Panes,
    rime: Vec<Rime>,
    t: f64,
    snapped: bool,
    landed: bool,
    dripped: bool,
}

/// A few splinters flung off at (x, y).
fn splinter(r: &mut Rng, bits: &mut Bits<()>, c: &[Rgba; 8], x: f64, y: f64, vx: f64, n: u32) {
    for _ in 0..n {
        let bx = x + r.range(-2.0, 2.0);
        let vx = vx * 0.5 + r.range(-45.0, 45.0);
        let vy = -r.range(30.0, 100.0);
        let life = r.range(0.4, 0.75);
        let a = r.range(0.0, TAU);
        let va = r.range(-18.0, 18.0);
        let s = r.range(1.2, 2.4);
        let col = c[r.int(4.0, 7.0) as usize];
        bits.add(Bit {
            kind: BitKind::Shard,
            x: bx,
            y: y - 1.0,
            vx,
            vy,
            a,
            va,
            s,
            c: col,
            ramp: None,
            g: 0.8,
            drag: 0.6,
            bounce: 0.35,
            life,
        });
    }
}

impl State {
    /// Where the icicle's top is: at the root, then falling.
    fn top(&self) -> f64 {
        ROOT + if self.t < FREE {
            0.0
        } else {
            0.5 * GRAVITY * (self.t - FREE) * (self.t - FREE)
        }
    }

    /// It snaps: a crack flashes across the root, chips and a puff burst off.
    fn snap(&mut self) {
        let (r, c) = (&mut self.r, &self.c);
        self.lights.add(0.0, ROOT + 4.0, 24.0, WHITE, 0.6, 0.1);
        self.lights.add(0.0, ROOT + 4.0, 32.0, c[5], 0.4, 0.3);
        for _ in 0..7 {
            let a = r.range(0.0, TAU);
            let sp = r.range(30.0, 110.0);
            let x = r.range(-4.0, 4.0);
            let life = r.range(0.5, 0.9);
            let ba = r.range(0.0, TAU);
            let va = r.range(-20.0, 20.0);
            let s = r.range(1.0, 2.0);
            let col = c[r.int(5.0, 7.0) as usize];
            self.bits.add(Bit {
                kind: BitKind::Shard,
                x,
                y: ROOT + 2.0,
                vx: libm::cos(a) * sp,
                vy: libm::sin(a) * sp - 20.0,
                a: ba,
                va,
                s,
                c: col,
                ramp: None,
                g: 0.8,
                drag: 0.8,
                bounce: 0.3,
                life,
            });
        }
        for _ in 0..3 {
            let x = r.range(-5.0, 5.0);
            let vx = r.range(-25.0, 25.0);
            let vy = r.range(-5.0, 5.0);
            let life = r.range(0.4, 0.7);
            let size = r.range(1.6, 2.6);
            self.ps.spawn(
                r,
                Spawn {
                    col: Some(c[6]),
                    size,
                    ..Spawn::new(Kind::Vapour, x, ROOT + 2.0, vx, vy, life)
                },
            );
        }
        for _ in 0..5 {
            let x = r.range(-6.0, 6.0);
            let vx = r.range(-30.0, 30.0);
            let vy = r.range(-20.0, 20.0);
            let life = r.range(0.6, 1.2);
            let col = if r.chance(0.5) { WHITE } else { c[6] };
            let size = r.range(0.5, 1.3);
            self.ps.spawn(
                r,
                Spawn {
                    col: Some(col),
                    size,
                    ..Spawn::new(Kind::Flake, x, ROOT + 2.0, vx, vy, life)
                },
            );
        }
    }

    /// It stabs the floor and bursts.
    fn burst(&mut self) {
        let (r, c) = (&mut self.r, &self.c);
        let tp = -LEN;
        self.lights.add(0.0, -16.0, 46.0, WHITE, 0.85, 0.12);
        self.lights.add(0.0, -16.0, 58.0, c[4], 0.5, 0.5);
        // the icicle in pieces: cut across its length, then down the middle
        let mut parts = vec![vec![(-W / 2.0, tp), (W / 2.0, tp), (0.5, 0.0)]];
        for v in [0.3, 0.58, 0.8] {
            let mut next = Vec::new();
            for poly in &parts {
                let y = tp + LEN * v + r.range(-2.0, 2.0);
                let a = r.range(-0.4, 0.4);
                next.extend(split_poly(poly, 0.0, y, a));
            }
            parts = next;
        }
        let mut next = Vec::new();
        for poly in &parts {
            let m = centroid(poly);
            if m.area > 22.0 {
                let a = PI / 2.0 + r.range(-0.5, 0.5);
                next.extend(split_poly(poly, m.x, m.y, a));
            } else {
                next.push(poly.clone());
            }
        }
        for part in next {
            let m = centroid(&part);
            let side = if m.x < 0.5 { -1.0 } else { 1.0 };
            let low = clamp(-m.y / LEN, 0.0, 1.0);
            let mut pane = Pane::new(
                part.iter().map(|&(x, y)| (x - m.x, y - m.y)).collect(),
                m.x,
                m.y,
            );
            pane.vx = side * r.range(30.0, 110.0) + r.range(-15.0, 15.0);
            pane.vy = -r.range(70.0, 200.0) * (0.4 + low);
            pane.va = r.range(-12.0, 12.0);
            pane.f = r.range(-0.7, 0.7);
            pane.vf = r.range(-8.0, 8.0);
            pane.lit = 0.75;
            pane.g = 0.9;
            pane.drag = 0.6;
            pane.bounce = 0.35;
            pane.solid = true;
            pane.ridge = FRAC_PI_2;
            pane.brittle = 2;
            pane.crumble = Some(r.range(0.15, 0.45));
            pane.life = r.range(1.0, 1.4);
            self.panes.add(pane);
        }
        for _ in 0..52 {
            let a = -FRAC_PI_2 + r.range(-1.25, 1.25);
            let sp = r.range(60.0, 230.0);
            let kind = if r.chance(0.15) {
                BitKind::Chunk
            } else {
                BitKind::Shard
            };
            let x = r.range(-3.0, 3.0);
            let y = -r.range(1.0, 10.0);
            let life = r.range(0.6, 1.2);
            let ba = r.range(0.0, TAU);
            let va = r.range(-22.0, 22.0);
            let s = r.range(1.2, 3.0);
            let col = c[r.int(4.0, 7.0) as usize];
            self.bits.add(Bit {
                kind,
                x,
                y,
                vx: libm::cos(a) * sp,
                vy: libm::sin(a) * sp,
                a: ba,
                va,
                s,
                c: col,
                ramp: Some(vec![c[1], c[3], c[4], c[6], c[7]]),
                g: 0.85,
                drag: 0.6,
                bounce: 0.32,
                life,
            });
        }
        for _ in 0..10 {
            let s = r.sign();
            let x = s * r.range(0.0, 8.0);
            let y = -r.range(1.0, 6.0);
            let vx = s * r.range(25.0, 90.0);
            let vy = -r.range(2.0, 14.0);
            let life = r.range(0.6, 1.1);
            let size = r.range(2.2, 3.8);
            self.ps.spawn(
                r,
                Spawn {
                    col: Some(c[6]),
                    size,
                    ..Spawn::new(Kind::Vapour, x, y, vx, vy, life)
                },
            );
        }
        for _ in 0..12 {
            let x = r.range(-6.0, 6.0);
            let y = -r.range(2.0, 16.0);
            let vx = r.range(-70.0, 70.0);
            let vy = -r.range(30.0, 110.0);
            let life = r.range(1.0, 1.5);
            let col = if r.chance(0.6) { WHITE } else { c[6] };
            let size = r.range(0.5, 1.5);
            self.ps.spawn(
                r,
                Spawn {
                    col: Some(col),
                    size,
                    ..Spawn::new(Kind::Flake, x, y, vx, vy, life)
                },
            );
        }
        for _ in 0..6 {
            let x = r.range(-24.0, 24.0);
            let y = -r.range(4.0, 34.0);
            let vx = r.range(-10.0, 10.0);
            let vy = -r.range(4.0, 14.0);
            let life = r.range(0.4, 0.8);
            let col = if r.chance(0.5) { WHITE } else { c[6] };
            let size = r.range(1.0, 2.2);
            self.ps.spawn(
                r,
                Spawn {
                    col: Some(col),
                    size,
                    ..Spawn::new(Kind::Star, x, y, vx, vy, life)
                },
            );
        }
    }

    /// The shock band rolling out along the floor where it struck: its far half (`far`) behind, its near half in
    /// front.
    fn shock(&self, p: &mut Painter, far: bool) {
        let c = &self.c;
        let k = seg(self.t, impact(), impact() + 0.35);
        if k >= 1.0 {
            return;
        }
        band(
            p,
            0.0,
            0.5,
            6.0 + 34.0 * ease::out_cubic(k),
            0.18,
            js_max(1.0, 4.0 * (1.0 - k)),
            1.1 - k,
            &[c[3], c[4], c[5], c[6]],
            |a| (libm::sin(a) < 0.0) == far,
            !far && k < 0.15,
        );
    }
}

impl EffectState for State {
    fn update(&mut self, dt: f64) {
        self.t += dt;
        let t = self.t;
        let impact = impact();
        // a drop of meltwater swells at the tip and falls
        if !self.dripped && t >= DRIP {
            self.dripped = true;
            self.bits.add(Bit {
                vy: 20.0,
                g: 0.9,
                life: 1.2,
                c: self.c[6],
                ..Bit::new(BitKind::Drip, 0.5, ROOT + LEN + 1.0)
            });
        }
        // it trembles: frost sifts down from the root
        if t > TREMBLE && t < SNAP && self.r.chance(0.25) {
            let r = &mut self.r;
            let x = r.range(-6.0, 6.0);
            let y = ROOT + r.range(0.0, 3.0);
            let vx = r.range(-8.0, 8.0);
            let vy = r.range(5.0, 20.0);
            let life = r.range(0.6, 1.1);
            let col = if r.chance(0.5) { WHITE } else { self.c[6] };
            let size = r.range(0.5, 1.1);
            self.ps.spawn(
                r,
                Spawn {
                    col: Some(col),
                    size,
                    ..Spawn::new(Kind::Flake, x, y, vx, vy, life)
                },
            );
        }
        if !self.snapped && t >= SNAP {
            self.snapped = true;
            self.snap();
        }
        // falling: it sheds frost motes that hang in the air behind it
        if t > FREE && t < impact && self.r.chance(0.5) {
            let tp = self.top();
            let r = &mut self.r;
            let x = r.range(-3.0, 3.0);
            let y = tp + r.range(0.0, LEN * 0.6);
            let vx = r.range(-10.0, 10.0);
            let vy = r.range(-40.0, -10.0);
            let life = r.range(0.2, 0.35);
            let col = if r.chance(0.5) { self.c[6] } else { self.c[5] };
            self.ps.spawn(
                r,
                Spawn {
                    col: Some(col),
                    size: 0.6,
                    ..Spawn::new(Kind::Mote, x, y, vx, vy, life)
                },
            );
        }
        if !self.landed && t >= impact {
            self.landed = true;
            self.burst();
        }
        if self.landed && t < impact + 0.9 && self.r.chance(0.08) && self.bits.count() > 0 {
            let r = &mut self.r;
            if let Some(b) = r.pick(self.bits.list()) {
                let (bx, by) = (b.bit.x, b.bit.y);
                let life = r.range(0.2, 0.4);
                let col = if r.chance(0.5) { WHITE } else { self.c[6] };
                let size = r.range(0.6, 1.4);
                self.ps.spawn(
                    r,
                    Spawn {
                        col: Some(col),
                        size,
                        ..Spawn::new(Kind::Star, bx, by - 1.0, 0.0, -3.0, life)
                    },
                );
            }
        }
        self.ps.update(&mut self.r, dt);
        self.bits.update(dt);
        let (r, ps, bits, c) = (&mut self.r, &mut self.ps, &mut self.bits, &self.c);
        self.panes.update(dt, |ev| match ev {
            PaneEvent::Land { pane, x, y, speed } => {
                if speed < 100.0 {
                    return;
                }
                let vx = r.range(-15.0, 15.0);
                let vy = -r.range(2.0, 8.0);
                let life = r.range(0.4, 0.6);
                let size = r.range(1.4, 2.2);
                ps.spawn(
                    r,
                    Spawn {
                        col: Some(c[6]),
                        size,
                        ..Spawn::new(Kind::Vapour, x, y - 1.0, vx, vy, life)
                    },
                );
                splinter(r, bits, c, x, y, pane.vx, 2);
            }
            PaneEvent::Crumble(pane) => splinter(r, bits, c, pane.x, 0.0, pane.vx, 3),
            PaneEvent::Skid { .. } => {}
        });
        self.lights.update(dt);
        end_by(
            self.t,
            D,
            &mut [&mut self.ps, &mut self.bits, &mut self.panes],
        );
    }

    fn draw(&self, p: &mut Painter, _t: f64, layer: Layer) {
        let (t, c) = (self.t, &self.c);
        let impact = impact();
        if layer == Layer::Back {
            // its shadow gathers on the floor as it comes down
            if t > TREMBLE && t < impact + 0.05 {
                let k = if t < FREE {
                    seg(t, TREMBLE, FREE) * 0.35
                } else {
                    0.35 + 0.65 * seg(t, FREE, impact)
                };
                let rx = 3.0 + 7.0 * k;
                let mut x = libm::floor(-rx);
                while x <= rx {
                    let f = 1.0 - (x / rx) * (x / rx);
                    p.blend(x + 0.5, 0.0, c[0], 0.7 * k * f);
                    if libm::fabs(x) < rx * 0.7 {
                        p.blend(x + 0.5, 1.0, c[0], 0.45 * k * f);
                    }
                    x += 1.0;
                }
            }
            // frost splashed across the floor where it struck, melting away
            if t >= impact {
                let g = ease::out_cubic(seg(t, impact, impact + 0.25));
                let out = 1.0 - smooth(impact + 0.8, impact + 1.5, t);
                for xi in -26..=26 {
                    let x = f64::from(xi);
                    let d = libm::fabs(x + 0.5) / 26.0;
                    let h = rand(&[x, 23.0]);
                    let a = out * clamp((g * 1.15 - d) / 0.15, 0.0, 1.0);
                    if a <= 0.0 {
                        continue;
                    }
                    p.dot(
                        x,
                        0.0,
                        if h < 0.35 || d > g - 0.12 {
                            WHITE
                        } else {
                            c[6]
                        },
                        a,
                    );
                    if h < 0.4 {
                        let n = js_round((1.0 + rand(&[x, 3.0]) * 2.5) * (1.0 - d));
                        let mut j = 1.0;
                        while j <= n {
                            let dx = if j > 1.0 { js_sign(x) } else { 0.0 };
                            p.dot(x + dx, -j, if j > 1.0 { WHITE } else { c[5] }, a);
                            j += 1.0;
                        }
                    }
                    let n = js_round((2.0 + rand(&[x, 7.0]) * 3.0) * (1.0 - d * 0.6));
                    let mut j = 1.0;
                    while j <= n {
                        p.dot(x, j, if j < 2.0 { c[5] } else { c[4] }, a * (1.1 - j / 6.0));
                        j += 1.0;
                    }
                }
                self.shock(p, true);
            }
            self.ps.draw(p, Layer::Back);
            return;
        }
        // the rime it hangs from, fading at the end
        let ra = 1.0 - smooth(D - 0.7, D - 0.2, t);
        for r in &self.rime {
            // a lumpy crust: white on top, pale below, a deep underside; little icicles hang off it
            let mut j = 0.0;
            while j < r.h + 1.0 {
                let col = if j == r.h {
                    WHITE
                } else if j == r.h - 1.0 {
                    c[6]
                } else if j == 0.0 {
                    c[3]
                } else {
                    c[5]
                };
                p.dot(r.x, ROOT - 1.0 - j, col, ra);
                j += 1.0;
            }
            let mut j = 0.0;
            while j < r.stub {
                let col = if j == r.stub - 1.0 {
                    WHITE
                } else if j == 0.0 {
                    c[5]
                } else {
                    c[4]
                };
                p.dot(r.x, ROOT + j, col, ra);
                j += 1.0;
            }
            if r.stub > 4.0 {
                p.dot(r.x + 1.0, ROOT, c[3], ra);
            }
        }
        // the snapped root: a jagged stub left behind
        if self.snapped {
            let mut x = -W / 2.0 + 1.0;
            while x <= W / 2.0 - 1.0 {
                let l = 1.0 + js_round(rand(&[x, 77.0]) * 2.5);
                let mut j = 0.0;
                while j < l {
                    let col = if j == l - 1.0 {
                        if t < SNAP + 0.15 { WHITE } else { c[6] }
                    } else if x < 0.0 {
                        c[5]
                    } else {
                        c[3]
                    };
                    p.dot(x, ROOT + j, col, ra);
                    j += 1.0;
                }
                x += 1.0;
            }
        }
        p.glow(0.0, ROOT, 18.0, c[4], 0.3 * ra);
        if t >= impact {
            self.shock(p, false);
            if t < impact + 0.22 {
                sparkle(
                    p,
                    0.5,
                    -3.0,
                    10.0 * ease::out_back(js_min(1.0, (t - impact) / 0.05), ease::BACK_S)
                        * (1.0 - (t - impact) / 0.22),
                    c[6],
                    c[4],
                    0.7,
                );
            }
        }
        if t < impact {
            // the icicle: trembling before it snaps, then falling with a streak of air behind it
            let tp = self.top();
            let jx = if t > TREMBLE && t < SNAP {
                f64::from(libm::floor(t * 36.0) % 2.0 != 0.0)
                    * js_round(seg(t, TREMBLE, SNAP) * 1.4 + 0.4)
            } else {
                0.0
            };
            let falling = t >= FREE;
            let sp = if falling { GRAVITY * (t - FREE) } else { 0.0 };
            if falling {
                for (dx, k) in [(-3.0, 0.7), (0.5, 1.0), (4.0, 0.6)] {
                    let l = js_min(30.0, sp * 0.07) * k;
                    if l > 2.0 {
                        p.line(
                            dx,
                            tp - l,
                            dx,
                            tp + 3.0,
                            |u: f64| {
                                if u > 0.75 {
                                    c[6]
                                } else if u > 0.4 {
                                    c[4]
                                } else {
                                    c[3]
                                }
                            },
                            1.0,
                        );
                    }
                }
            }
            let gl = (t - 0.04) / 0.3;
            icicle(
                p,
                c,
                jx + 0.5,
                tp,
                LEN,
                W,
                &Icicle {
                    white: t >= impact - 0.025 || (SNAP..SNAP + 0.03).contains(&t),
                    glint: if (0.0..=1.0).contains(&gl) { gl } else { -1.0 },
                },
            );
            if self.snapped && t < SNAP + 0.12 {
                p.line(
                    -W / 2.0 - 1.0,
                    ROOT + 1.0,
                    W / 2.0 + 1.0,
                    ROOT + 1.0,
                    WHITE,
                    1.0,
                );
            }
            p.glow(0.5, tp + LEN * 0.5, 20.0, c[4], 0.28);
            // the drop of meltwater swelling at the tip
            if t < DRIP {
                let s = seg(t, 0.0, DRIP);
                p.dot(0.5, ROOT + LEN + 1.0, c[6], s);
                if s > 0.6 {
                    p.px(0.5, ROOT + LEN + 2.0, WHITE);
                }
            }
        }
        self.panes.draw(p, c);
        self.bits.draw(p);
        self.ps.draw(p, Layer::Front);
        self.lights.draw(p);
    }
}
