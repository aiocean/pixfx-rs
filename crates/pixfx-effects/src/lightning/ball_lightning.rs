//! Ball lightning: port of pixelfx `src/fx/lightning/ball-lightning.js`.
//!
//! The layered ball, its tapering wake and the zap to the floor with stars where it lands follow Sprite Fusion's
//! open-source energy orb and bolt (https://www.spritefusion.com/pixel-art-vfx/energy),
//! MIT License, Copyright (c) 2026 Hugo Duprez.

use pixfx_core::particles::cycle;
use pixfx_core::{Effect, EffectState, Layer, Meta, Options, RampOpt};
use pixfx_draw::Painter;
use pixfx_draw::color::{Rgba, WHITE, ramp};
use pixfx_draw::math::{TAU, clamp, rng};

use super::lib::{PLASMA, StrokeOpts, VOLT, crackle, flight, spark, stroke, zigzag};

pub struct BallLightning;

pub static BALL_LIGHTNING: BallLightning = BallLightning;

const META: Meta = Meta {
    id: "lightning/ball-lightning",
    name: "Ball lightning",
    desc: "A crackling ball of plasma drifts on a wandering loop, spitting arcs off its skin and leaving a wake of fading motes, and zaps the floor when it dips low.",
    tags: &[
        "lightning",
        "electric",
        "plasma",
        "orb",
        "wisp",
        "ambient",
        "loop",
    ],
    size: (136, 104),
    origin: (68, 92),
    duration: 4.0,
    looping: true,
    fps: 30,
    seed: 9,
    state: None,
    options: &[SPARKS.spec()],
};

const SPARKS: RampOpt = RampOpt::new("sparks", &PLASMA);

const FPS: f64 = 30.0;
const N: f64 = 120.0;
const P: f64 = 4.0;
const ZAP: f64 = 62.0;

impl Effect for BallLightning {
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

/// Where the ball is: a closed, wandering path that dips toward the floor once a loop.
fn at(t: f64) -> (f64, f64) {
    let a = (t / P) * TAU;
    (
        38.0 * libm::sin(a) + 9.0 * libm::sin(2.0 * a + 1.0),
        -42.0 + 13.0 * libm::sin(2.0 * a + 0.4) + 6.0 * libm::sin(3.0 * a + 0.5),
    )
}

impl EffectState for State {
    fn draw(&self, p: &mut Painter, t: f64, layer: Layer) {
        if layer == Layer::Back {
            return;
        }
        let (c, seed) = (&self.c, self.seed);
        let f = libm::floor(t * FPS + 1e-6) % N;
        let mut r = rng(&[seed, 1.0, f]);
        let (x, y) = at(t);
        // light under the ball on the floor, stronger as it comes low
        p.glow(
            x,
            -2.0,
            30.0,
            c[3],
            0.12 + 0.3 * clamp((y + 60.0) / 40.0, 0.0, 1.0),
        );
        // the wake: where it has been, shrinking and cooling, and motes shed along it, drifting and dying
        for i in (1..=14).rev() {
            let (wx, wy) = at(t - f64::from(i) * 0.034);
            let u = f64::from(i) / 14.0;
            let rr = 3.0 * (1.0 - u) + 0.3;
            if u > 0.6 && (f64::from(i) + f) % 2.0 != 0.0 {
                continue;
            }
            p.circle(wx, wy, rr + 0.8, if u < 0.35 { c[3] } else { c[2] }, 1.0);
            let body = if u < 0.35 {
                c[6]
            } else if u < 0.7 {
                c[5]
            } else {
                c[4]
            };
            p.circle(wx, wy, rr, body, 1.0);
            p.glow(wx, wy, 7.0 * (1.0 - u) + 3.0, c[4], 0.25 * (1.0 - u));
        }
        cycle(
            t,
            P,
            40,
            seed + 5.0,
            |g| g.range(0.5, 1.0),
            |age, life, q, _| {
                let born = t - age;
                let (bx, by) = at(born);
                let k = 1.0 - age / life;
                let a = q.range(0.0, TAU);
                let sp = q.range(4.0, 14.0);
                let mx = bx + libm::cos(a) * sp * age + q.range(-3.0, 3.0);
                let my = by + libm::sin(a) * sp * age + 6.0 * age + q.range(-3.0, 3.0);
                if k > 0.25 || f % 2.0 != 0.0 {
                    let col = if k > 0.6 {
                        c[6]
                    } else if k > 0.3 {
                        c[4]
                    } else {
                        c[3]
                    };
                    p.px(mx, my, col);
                }
                p.glow(mx, my, 3.0, c[4], 0.3 * k);
            },
        );
        // the ball: a churning skin with hot spots rolling round it, a white heart, corona bristling off it
        let r0 = 5.2 + 0.6 * libm::sin(t * 9.0) + 0.4 * r.range(-1.0, 1.0);
        p.circle(x, y, r0 + 1.3, c[2], 1.0);
        p.circle(x, y, r0, c[4], 1.0);
        p.circle(x - 0.4, y - 0.4, r0 * 0.72, c[5], 1.0);
        for i in 0..3 {
            let fi = f64::from(i);
            let a = t * (5.0 + fi * 2.3) + fi * 2.1;
            let d = r0 * 0.6;
            p.circle(
                x + libm::cos(a) * d,
                y + libm::sin(a) * d * 0.8,
                1.2,
                if i != 0 { c[6] } else { WHITE },
                1.0,
            );
        }
        p.circle(x - 0.6, y - 0.6, r0 * 0.32, WHITE, 1.0);
        for i in 0..8 {
            if r.next() < 0.35 {
                continue;
            }
            let a = (f64::from(i) / 8.0) * TAU + t * 2.0 + r.range(-0.2, 0.2);
            let l = r.range(1.5, 4.0);
            p.line(
                x + libm::cos(a) * (r0 + 1.0),
                y + libm::sin(a) * (r0 + 1.0),
                x + libm::cos(a) * (r0 + 1.0 + l),
                y + libm::sin(a) * (r0 + 1.0 + l),
                if i % 2 != 0 { c[5] } else { c[4] },
                1.0,
            );
        }
        p.glow(x, y, 30.0, c[4], 0.3);
        p.glow(x, y, 9.0, c[6], 0.1);
        // arcs spat off its skin, re-struck every frame (the JS loop bound draws again on every test)
        let mut i = 0;
        while i < 2 + i32::from(r.next() < 0.4) {
            let a = r.range(0.0, TAU);
            let l = r.range(5.0, 11.0);
            let x0 = x + libm::cos(a) * (r0 + 1.0);
            let y0 = y + libm::sin(a) * (r0 + 1.0);
            let b = a + r.range(-0.7, 0.7);
            let bolt = zigzag(
                &mut r,
                x0,
                y0,
                x0 + libm::cos(b) * l,
                y0 + libm::sin(b) * l,
                0.4,
                0.3,
            );
            let k = r.range(0.4, 0.7);
            crackle(p, &bolt, k, c, 0.25, 1.0);
            i += 1;
        }
        // once a loop, low over the floor, it zaps down: a bolt, the floor lit, sparks kicked up
        let za = f - ZAP;
        let (zx, _) = at(ZAP / FPS);
        let gx = zx + rng(&[seed, 3.0]).range(-10.0, 10.0);
        if (0.0..4.0).contains(&za) {
            let k = [1.0, 0.6, 0.9, 0.4][za as usize];
            let bolt = zigzag(&mut rng(&[seed, 4.0, f]), x, y + r0, gx, 0.0, 0.28, 0.4);
            stroke(
                p,
                &bolt,
                k,
                c,
                0.7,
                StrokeOpts {
                    hero: za == 0.0,
                    ..StrokeOpts::default()
                },
            );
            p.circle(gx, -1.0, 2.5 * k + 1.0, c[5], 1.0);
            p.px(gx, -1.0, WHITE);
            p.glow(gx, -4.0, 34.0, c[4], 0.6 * k);
        }
        let sa = (((t - ZAP / FPS) % P) + P) % P;
        for j in 0..14 {
            let mut q = rng(&[seed, 7.0, f64::from(j)]);
            let life = q.range(0.25, 0.55);
            if sa >= life {
                continue;
            }
            let a = -core::f64::consts::FRAC_PI_2 + q.range(-1.3, 1.3);
            let sp = q.range(50.0, 170.0);
            let (sx, sy, vx, vy) = flight(gx, -1.0, libm::cos(a) * sp, libm::sin(a) * sp, sa, 0.0);
            spark(p, sx, sy, vx, vy, 1.0 - sa / life, &self.s);
        }
    }
}
