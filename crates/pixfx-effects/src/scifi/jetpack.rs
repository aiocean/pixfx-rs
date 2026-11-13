//! Jetpack: port of pixelfx `src/fx/scifi/jetpack.js`.
//!
//! The exhaust (flame puffs cooling down the fire ramp into smoke, a warm light over the smoke) follows Sprite
//! Fusion's open-source fire (https://www.spritefusion.com/pixel-art-vfx/fire), MIT License,
//! Copyright (c) 2026 Hugo Duprez.

use core::f64::consts::PI;

use pixfx_core::color::{Rgba, WHITE, ramp};
use pixfx_core::math::{Rng, TAU, js_round, lerp, rand, tile_fbm, to_uint32};
use pixfx_core::sim::{Flags, Kind};
use pixfx_core::{Effect, EffectState, Layer, Meta, Options, Painter};

use super::lib::{Birth, Ghosts};

pub struct Jetpack;

pub static JETPACK: Jetpack = Jetpack;

const META: Meta = Meta {
    id: "scifi/jetpack",
    name: "Jetpack",
    desc: "Twin thrusters fire straight down: white-hot cores with shock diamonds inside ragged flames, puffs that cool into smoke billowing below, lit by the fire.",
    tags: &[
        "scifi", "jetpack", "thrust", "rocket", "fire", "smoke", "flight", "loop",
    ],
    size: (96, 116),
    origin: (48, 30),
    duration: 2.0,
    looping: true,
    fps: 30,
    seed: 30,
};

// meta.options
const PALETTE: [Rgba; 8] = [
    Rgba::hex24(0x2a0e12),
    Rgba::hex24(0x5c1818),
    Rgba::hex24(0xa02818),
    Rgba::hex24(0xe0501a),
    Rgba::hex24(0xff8a2a),
    Rgba::hex24(0xffc23d),
    Rgba::hex24(0xfff3a6),
    Rgba::hex24(0xffffff),
];
const METAL: [Rgba; 6] = [
    Rgba::hex24(0x0e0d14),
    Rgba::hex24(0x1e1c27),
    Rgba::hex24(0x312e3e),
    Rgba::hex24(0x4b4759),
    Rgba::hex24(0x6f6a82),
    Rgba::hex24(0x9d98b0),
];
const LENGTH: f64 = 26.0;

const D: f64 = META.duration;
const GAP: f64 = 8.0;
const FR: f64 = 30.0;

impl Effect for Jetpack {
    fn meta(&self) -> &Meta {
        &META
    }
    fn spawn(&self, opts: &Options, seed: u64) -> Box<dyn EffectState> {
        let f: [Rgba; 8] = ramp(&opts.palette_or(&PALETTE));
        let length = opts.num_or("length", LENGTH);
        let s = seed as f64;
        Box::new(State {
            f,
            h: ramp(&opts.ramp_or("metal", &METAL)),
            length,
            s,
            ghosts: ghosts(&f, length, s),
        })
    }
}

struct State {
    f: [Rgba; 8],
    h: [Rgba; 6],
    length: f64,
    s: f64,
    ghosts: Ghosts,
}

impl EffectState for State {
    fn draw(&self, p: &mut Painter, t: f64, layer: Layer) {
        if layer != Layer::Back {
            draw(p, t, self);
        }
    }
}

fn ghosts(f: &[Rgba; 8], length: f64, seed: f64) -> Ghosts {
    let mut rn = Rng::new(to_uint32(seed));
    let l = length;
    let mut g = Ghosts::new(D, vec![WHITE, f[6], f[5], f[4]], [-46.0, -28.0, 46.0, 84.0]);
    for s in [-1.0, 1.0] {
        // flame shed off the tip, cooling down the ramp into smoke that's driven down, slows and billows
        for _ in 0..26 {
            let born = rn.range(0.0, D);
            let x = s * GAP + rn.range(-1.5, 1.5);
            let y = l * rn.range(0.55, 0.85);
            let vx = rn.range(-10.0, 10.0);
            let vy = rn.range(70.0, 130.0);
            let life = rn.range(0.22, 0.4);
            let size = rn.range(2.2, 3.4);
            let big = if rn.chance(0.5) {
                Flags::BIG
            } else {
                Flags::NONE
            };
            g.add(
                born,
                Birth {
                    col: Rgba::NONE,
                    size,
                    flags: Flags::SMOKES | big,
                    ..Birth::new(Kind::Fire, x, y, vx, vy, life)
                },
            );
        }
        for _ in 0..10 {
            let born = rn.range(0.0, D);
            let x = s * GAP + rn.range(-3.0, 3.0);
            let y = l + rn.range(2.0, 8.0);
            let vx = s * rn.range(0.0, 14.0);
            let vy = rn.range(40.0, 80.0);
            let life = rn.range(1.1, 1.6);
            let size = rn.range(3.0, 5.0);
            g.add(
                born,
                Birth {
                    col: Rgba::NONE,
                    size,
                    flags: Flags::BIG,
                    ..Birth::new(Kind::Smoke, x, y, vx, vy, life)
                },
            );
        }
        // sparks spat out of the exhaust, embers drifting off it
        for _ in 0..10 {
            let born = rn.range(0.0, D);
            let x = s * GAP + rn.range(-1.0, 1.0);
            let y = l * rn.range(0.3, 0.7);
            let vx = s * rn.range(-40.0, 60.0);
            let vy = rn.range(80.0, 200.0);
            let life = rn.range(0.2, 0.45);
            g.add(born, Birth::new(Kind::Spark, x, y, vx, vy, life));
        }
        for _ in 0..5 {
            let born = rn.range(0.0, D);
            let x = s * GAP + rn.range(-2.0, 2.0);
            let y = l * rn.range(0.6, 1.0);
            let vx = s * rn.range(10.0, 40.0);
            let vy = rn.range(20.0, 60.0);
            let life = rn.range(0.6, 1.1);
            let size = rn.range(0.8, 1.6);
            g.add(
                born,
                Birth {
                    col: f[4],
                    size,
                    ..Birth::new(Kind::Ember, x, y, vx, vy, life)
                },
            );
        }
    }
    g
}

/// The pack: twin tanks with rounded caps, a spine between them, belled nozzles glowing hot inside.
fn pack(p: &mut Painter, h: &[Rgba; 6], f: &[Rgba; 8], dy: f64) {
    for s in [-1.0, 1.0] {
        let cx = s * GAP;
        p.shade(cx - 5.0, -24.0 + dy, cx + 5.0, -3.0 + dy, |x, y, out| {
            let u = (x - cx) / 4.5;
            let top = -19.0 + dy;
            let v = if y < top { (y - top) / 4.5 } else { 0.0 };
            if u * u + v * v > 1.0 {
                return None;
            }
            let l = -u * 0.8 - v * 0.6;
            out.c = if u * u + v * v > 0.78 {
                h[0]
            } else if libm::fabs(y - (-12.0 + dy)) < 0.6 {
                h[1]
            } else if l > 0.55 {
                h[5]
            } else if l > 0.15 {
                h[4]
            } else if l > -0.35 {
                h[3]
            } else {
                h[2]
            };
            None
        });
        // the nozzle bell, wider at its mouth, its inside lit by the flame
        for yi in -3..=0 {
            let y = f64::from(yi);
            let w = 2.0 + (y + 3.0) * 0.7;
            let mut x = libm::floor(cx - w);
            while x <= libm::ceil(cx + w) {
                let inner = libm::fabs(x + 0.5 - cx) < w - 1.2 && y >= -1.0;
                let c = if inner {
                    if y == 0.0 { f[6] } else { f[4] }
                } else if libm::fabs(x + 0.5 - cx) > w - 0.6 {
                    h[0]
                } else if x < cx {
                    h[3]
                } else {
                    h[1]
                };
                p.px(x, y + dy, c);
                x += 1.0;
            }
        }
    }
    for yi in -20..=-6 {
        let y = f64::from(yi) + dy;
        p.px(-1.0, y, h[1]);
        p.px(0.0, y, h[2]);
        p.px(1.0, y, h[0]);
    }
    // a status light on the spine
    p.px(0.0, -16.0 + dy, f[5]);
}

fn draw(p: &mut Painter, t: f64, st: &State) {
    let (f, h, s) = (&st.f, &st.h, st.s);
    let ph = t / D;
    let fr = libm::floor(t * FR) % js_round(D * FR);
    let dy = js_round(libm::sin(TAU * 12.0 * ph) * 0.6);
    // smoke and spent flame behind, then the plumes
    let g = st.ghosts.at(t);
    g.draw(p, Layer::Back);
    for sd in [-1.0, 1.0] {
        let cx = sd * GAP
            + if rand(&[s, fr, sd]) < 0.15 {
                sd * 0.5
            } else {
                0.0
            };
        let l = st.length
            * (0.92
                + 0.08 * libm::sin(TAU * 7.0 * ph + sd)
                + 0.06 * (rand(&[s, fr, sd, 1.0]) - 0.5));
        // the flame: a cone of heat torn ragged by noise streaming down it, white at its heart, red at its edges
        p.shade(cx - 7.0, dy, cx + 7.0, dy + l + 4.0, |x, y, out| {
            let v = (y - dy) / l;
            let w = lerp(3.4, 0.6, v) + 1.4 * libm::sin(v * PI) * if v < 1.0 { 1.0 } else { 0.0 };
            let n = tile_fbm(
                (x - cx) / 3.0 + sd * 5.0,
                (y - dy) / 4.0 - ph * 20.0,
                0.0,
                10.0,
                s + if sd > 0.0 { 9.0 } else { 0.0 },
                2,
            );
            let q = libm::fabs(x - cx) / w + n * 0.45;
            if v > 1.15 || q > 1.2 {
                return None;
            }
            let heat = (1.0 - v) * 1.15 - q * 0.9 + n * 0.25;
            // a dark red rim round the flame, so it reads as a shape and not a smear
            if q > 0.95 {
                out.c = if heat > 0.2 { f[3] } else { f[2] };
                if heat < -0.3 {
                    out.a = 0.5;
                }
                return None;
            }
            out.c = if heat > 0.82 {
                WHITE
            } else if heat > 0.62 {
                f[6]
            } else if heat > 0.42 {
                f[5]
            } else if heat > 0.22 {
                f[4]
            } else if heat > 0.02 {
                f[3]
            } else {
                f[2]
            };
            None
        });
        // shock diamonds: bright knots strung down the core, breathing
        for i in 0..3 {
            let fi = f64::from(i);
            let y = js_round(dy + 5.0 + fi * 6.5 + libm::sin(TAU * 4.0 * ph + fi) * 0.5);
            let r = js_round(2.2 - fi * 0.45 + 0.4 * libm::sin(TAU * 8.0 * ph + fi * 2.0));
            if y > dy + l * 0.75 || r < 1.0 {
                break;
            }
            let mut yy = -r;
            while yy <= r {
                let mut xx = -r;
                while xx <= r {
                    let d = libm::fabs(xx) + libm::fabs(yy);
                    if d <= r {
                        let c = if d < r {
                            WHITE
                        } else if i == 0 {
                            f[6]
                        } else {
                            f[5]
                        };
                        p.px(cx + xx, y + yy, c);
                    }
                    xx += 1.0;
                }
                yy += 1.0;
            }
            p.glow(cx, y, 6.0, f[6], 0.35);
        }
        // light: hot at the nozzle, warm down the plume
        p.glow(cx, dy + 2.0, 9.0, WHITE, 0.45);
        p.glow(cx, dy + l * 0.45, l * 0.8, f[4], 0.35);
    }
    pack(p, h, f, dy);
    g.draw(p, Layer::Front);
    // the fire lights the smoke below it, and its own surroundings
    p.glow(0.0, st.length + 14.0, 34.0, f[4], 0.16);
    p.glow(0.0, st.length * 0.4, 30.0, f[5], 0.18);
}
