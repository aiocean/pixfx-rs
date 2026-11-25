//! Spark arc: port of pixelfx `src/fx/lightning/spark-arc.js`.
//!
//! The arc's re-struck bowed shape, its halo-body-core inking, its flicker and the nodes where it roots follow
//! Sprite Fusion's open-source lightning (https://www.spritefusion.com/pixel-art-vfx/lightning),
//! MIT License, Copyright (c) 2026 Hugo Duprez.

use pixfx_core::{Effect, EffectState, Layer, Meta, Options, RampOpt};
use pixfx_draw::Painter;
use pixfx_draw::color::{Rgba, WHITE, mix, ramp};
use pixfx_draw::math::{Rng, clamp, js_round, rng};

use super::lib::{
    PLASMA, StrokeOpts, VOLT, bowed, crackle, flight, heat, puff, spark, stroke, zigzag,
};

pub struct SparkArc;

pub static SPARK_ARC: SparkArc = SparkArc;

const META: Meta = Meta {
    id: "lightning/spark-arc",
    name: "Spark arc",
    desc: "A climbing arc between two rods: it strikes white at their narrow foot, climbs on rising heat as it stretches and bows, sputters and snaps at the top, then strikes again.",
    tags: &[
        "lightning",
        "electric",
        "arc",
        "jacobs-ladder",
        "machine",
        "prop",
        "loop",
    ],
    size: (124, 120),
    origin: (62, 108),
    duration: 2.0,
    looping: true,
    fps: 30,
    seed: 2,
    state: None,
    options: &[SPARKS.spec()],
};

const SPARKS: RampOpt = RampOpt::new("sparks", &PLASMA);

const FPS: f64 = 30.0;
const N: f64 = 60.0;
const P: f64 = 2.0;
const SNAP: f64 = 1.42;
const H: f64 = 58.0;
const STEEL: [Rgba; 6] = [
    Rgba::hex24(0x151823),
    Rgba::hex24(0x262c3b),
    Rgba::hex24(0x3d4558),
    Rgba::hex24(0x5f6a82),
    Rgba::hex24(0x8f9bb2),
    Rgba::hex24(0xd2daea),
];
const CERAMIC: [Rgba; 4] = [
    Rgba::hex24(0x4b4458),
    Rgba::hex24(0x857d96),
    Rgba::hex24(0xc9c1d6),
    Rgba::hex24(0xefe9f6),
];
const ASH: Rgba = Rgba::hex24(0x6a6478);
const UP: f64 = -core::f64::consts::FRAC_PI_2;

impl Effect for SparkArc {
    fn meta(&self) -> &Meta {
        &META
    }
    fn spawn(&self, opts: &Options, seed: u64) -> Box<dyn EffectState> {
        Box::new(State {
            c: ramp(&opts.palette_or(&VOLT)),
            s: opts.ramp(&SPARKS),
            seed: seed as f64,
        })
    }
}

struct State {
    c: [Rgba; 8],
    s: Vec<Rgba>,
    seed: f64,
}

/// The rods: a V from the insulators up; where the arc roots at height `h` above the gap.
fn rod_at(s: f64, h: f64) -> (f64, f64) {
    (s * (4.5 + (h / 60.0) * 19.5), -9.0 - h)
}

fn height(t: f64) -> f64 {
    1.5 + H * libm::pow(clamp(t / SNAP, 0.0, 1.0), 1.35)
}

/// Where a burst's spark starts: x, y, heading, spread, slowest and fastest speed.
type From = (f64, f64, f64, f64, f64, f64);

impl State {
    /// Sparks thrown at `t0` each loop: `n` of them, each from `from(q, j)`.
    fn burst(
        &self,
        p: &mut Painter,
        t: f64,
        t0: f64,
        n: u32,
        salt: f64,
        from: impl Fn(&mut Rng, u32) -> From,
    ) {
        let age = (((t - t0) % P) + P) % P;
        for j in 0..n {
            let mut q = rng(&[self.seed, salt, f64::from(j)]);
            let life = q.range(0.25, 0.6);
            if age >= life {
                continue;
            }
            let (x0, y0, a0, spread, sp0, sp1) = from(&mut q, j);
            let ang = a0 + q.range(-spread, spread);
            let sp = q.range(sp0, sp1);
            let (x, y, vx, vy) = flight(x0, y0, libm::cos(ang) * sp, libm::sin(ang) * sp, age, 0.0);
            spark(p, x, y, vx, vy, 1.0 - age / life, &self.s);
        }
    }
}

impl EffectState for State {
    fn draw(&self, p: &mut Painter, t: f64, layer: Layer) {
        if layer == Layer::Back {
            return;
        }
        let (c, seed) = (&self.c, self.seed);
        let f = libm::floor(t * FPS + 1e-6) % N;
        let mut r = rng(&[seed, 1.0, f]);
        let climbing = t < SNAP;
        let h = height(t);
        let u = clamp(t / SNAP, 0.0, 1.0);
        let a = t - SNAP;
        // the floor and the stand: a pool of cold light while it burns, a plate, two ribbed insulators
        p.glow(
            0.0,
            -14.0,
            50.0,
            c[3],
            if climbing {
                0.2 + 0.15 * (1.0 - u)
            } else {
                0.06
            },
        );
        p.rect(-18.0, -3.0, 36.0, 3.0, STEEL[1], 1.0);
        p.line(-18.0, -3.0, 17.0, -3.0, STEEL[3], 1.0);
        for s in [-1i32, 1] {
            for y in -9i32..-3 {
                let rib = (y & 1) == 0;
                for x in 2i32..=6 {
                    let col = if rib {
                        CERAMIC[if x < 4 {
                            3
                        } else if x < 6 {
                            2
                        } else {
                            1
                        }]
                    } else {
                        CERAMIC[if x < 5 { 1 } else { 0 }]
                    };
                    p.px(f64::from(s * x - i32::from(s < 0)), f64::from(y), col);
                }
            }
        }
        // the rods: steel, a lit edge, glowing where the arc has passed and cooling behind it
        for s in [-1.0, 1.0] {
            let mut hh = 0.0;
            while hh <= 60.0 {
                // when the arc passed this point; not yet this time round, it is still cooling from the last
                let (x, y) = rod_at(s, hh);
                let tp = SNAP * libm::pow(clamp((hh - 1.5) / H, 0.0, 1.0), 1.0 / 1.35);
                let age = if t >= tp { t - tp } else { t - tp + P };
                let hot = if hh > H + 2.0 {
                    0.0
                } else {
                    libm::exp(-age / 0.26)
                };
                let hc = heat(hot * 1.05 - 0.12);
                p.px(
                    x,
                    y,
                    hc.unwrap_or(if s < 0.0 { STEEL[4] } else { STEEL[3] }),
                );
                let edge = if hc.is_some() && hot > 0.35 {
                    heat(hot * 0.6).unwrap_or(STEEL[2])
                } else if s < 0.0 {
                    STEEL[2]
                } else {
                    STEEL[1]
                };
                p.px(x + s, y, edge);
                if hot > 0.3 && (js_round(hh * 2.0) as i64) & 3 == 0 {
                    p.glow(x, y, 5.0, hc.unwrap_or(c[4]), 0.25 * hot);
                }
                hh += 0.5;
            }
            let (tx, ty) = rod_at(s, 60.0);
            p.circle(tx + s * 0.5, ty, 1.6, STEEL[4], 1.0);
            p.px(tx + s * 0.5 - 0.5, ty - 1.0, STEEL[5]);
        }
        // the arc: struck white at the foot, flickering as it climbs and stretches, sputtering at the top
        if climbing {
            let (xl, yl) = rod_at(-1.0, h);
            let (xr, yr) = rod_at(1.0, h);
            let first = t < 1.0 / FPS;
            let k0 = if first { 1.0 } else { 1.0 - 0.22 * u };
            let sput = if u > 0.88 && r.next() < 0.25 {
                0.4
            } else {
                1.0
            };
            let k = k0 * sput * (0.82 + 0.18 * r.next());
            let bow = -(1.5 + 13.0 * u * u) * (0.8 + 0.4 * r.next());
            let bolt = bowed(
                &mut r,
                xl,
                yl,
                xr,
                yr,
                bow,
                0.18 + 0.12 * u,
                0.12 + 0.25 * u,
            );
            stroke(
                p,
                &bolt,
                k,
                c,
                if first { 0.95 } else { 0.8 },
                StrokeOpts {
                    hero: first,
                    light: 0.7,
                    ..StrokeOpts::default()
                },
            );
            p.glow_line(xl, yl + bow * 0.5, xr, yr + bow * 0.5, 12.0, c[4], 0.22 * k);
            for (x, y) in [(xl, yl), (xr, yr)] {
                p.circle(x, y, 1.8 + 0.6 * k, c[5], 1.0);
                p.px(x, y, WHITE);
                p.glow(x, y, 12.0, c[5], 0.55 * k);
            }
            if t < 0.1 {
                p.glow(0.0, -11.0, 26.0, c[6], 0.45 * (1.0 - t / 0.1));
            }
        } else if a < 2.0 / FPS {
            // the snap: two stubs left clinging to the rods, a flash where it broke
            let (xl, yl) = rod_at(-1.0, H + 1.5);
            let (xr, yr) = rod_at(1.0, H + 1.5);
            for (x0, y0, s) in [(xl, yl, 1.0), (xr, yr, -1.0)] {
                let x1 = x0 + s * r.range(5.0, 9.0);
                let y1 = y0 - r.range(2.0, 6.0);
                let bolt = zigzag(&mut r, x0, y0, x1, y1, 0.4, 0.3);
                crackle(p, &bolt, 0.7, c, 0.4, 1.0);
            }
            p.glow(0.0, yl - 8.0, 30.0, c[4], 0.5);
        } else if t > P - 0.16 {
            // anticipation: little sparks jump the narrow gap before it strikes again
            if r.next() < 0.7 {
                let y0 = -10.5 - r.range(0.0, 2.0);
                let y1 = -10.5 - r.range(0.0, 2.0);
                let bolt = zigzag(&mut r, -4.5, y0, 4.5, y1, 0.5, 0.0);
                let k = 0.4 + 0.3 * r.next();
                crackle(p, &bolt, k, c, 0.35, 1.0);
            }
        }
        // ozone and smoke rise off the arc while it climbs, and from where it snapped
        for j in 0..3 {
            let fj = f64::from(j);
            let born = ((fj + 1.0) / 3.2) * SNAP;
            let age = (((t - born) % P) + P) % P;
            let life = 1.0;
            if age >= life {
                continue;
            }
            let y0 = -9.0 - height(born) - 6.0;
            puff(
                p,
                (if j % 2 != 0 { 3.0 } else { -3.0 }) + libm::sin(age * 3.0 + fj) * 2.0,
                y0 - age * 20.0,
                1.8 + fj * 0.3,
                age / life,
                mix(ASH, c[5], 0.3),
            );
        }
        // sparks: thrown from the roots as it strikes, sprayed up where it snaps, molten drips off the rods
        let side = |j: u32| if j.is_multiple_of(2) { -1.0 } else { 1.0 };
        self.burst(p, t, 0.0, 12, 11.0, |_, j| {
            let (x, y) = rod_at(side(j), 1.5);
            (
                x,
                y,
                UP + if j % 2 != 0 { 0.6 } else { -0.6 },
                1.0,
                60.0,
                170.0,
            )
        });
        self.burst(p, t, SNAP, 14, 12.0, |q, _| {
            (
                rod_at(1.0, H).0 * q.range(-0.8, 0.8),
                -9.0 - H - 6.0,
                UP,
                1.3,
                50.0,
                160.0,
            )
        });
        for (k, tt) in [(0.0, 0.45), (1.0, 0.8), (2.0, 1.15)] {
            self.burst(p, t, tt, 2, 20.0 + k, |_, j| {
                let (x, y) = rod_at(side(j), height(tt));
                (x, y, core::f64::consts::FRAC_PI_2, 0.5, 5.0, 30.0)
            });
        }
        if climbing || a < 0.25 {
            p.glow(
                0.0,
                -9.0 - h - 3.0,
                26.0,
                c[4],
                if climbing {
                    0.25
                } else {
                    0.4 * (1.0 - a / 0.25)
                },
            );
        }
    }
}
