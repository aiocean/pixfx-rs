//! Whirlpool: port of pixelfx `src/fx/water/whirlpool.js`.
//!
//! The swirl's texture (streaks laid along a differentially wound flow, light on the brightest pixels) is adapted
//! from Sprite Fusion's open-source black hole (https://www.spritefusion.com/pixel-art-vfx/black-hole), MIT
//! License, Copyright (c) 2026 Hugo Duprez.

use core::f64::consts::PI;

use pixfx_core::particles::cycle;
use pixfx_core::{Effect, EffectState, Layer, Meta, Options};
use pixfx_draw::Painter;
use pixfx_draw::color::{WHITE, ramp};
use pixfx_draw::math::{TAU, js_max, js_min, smooth};

use super::lib::{CROSS, Ramp, WATER, js_hypot, shade_with};

pub struct Whirlpool;

pub static WHIRLPOOL: Whirlpool = Whirlpool;

const META: Meta = Meta {
    id: "water/whirlpool",
    name: "Whirlpool",
    desc: "Seen from above, dark water winds into a vortex: foam arms spiral in, darkening as they're drawn down into a deep eye with a lit far rim, flecks of foam circling in after them.",
    tags: &[
        "water",
        "whirlpool",
        "vortex",
        "maelstrom",
        "swirl",
        "top-down",
        "loop",
    ],
    size: (128, 84),
    origin: (64, 42),
    duration: 3.0,
    looping: true,
    fps: 30,
    seed: 5,
    state: None,
    options: &[],
};

const D: f64 = 3.0;
const R: f64 = 60.0;
const SY: f64 = 0.6;
const K: f64 = 2.4;
const EYE: f64 = 0.16;

impl Effect for Whirlpool {
    fn meta(&self) -> &Meta {
        &META
    }
    fn spawn(&self, opts: &Options, seed: u64) -> Box<dyn EffectState> {
        Box::new(State {
            c: ramp(&opts.palette_or(&WATER)),
            seed: seed as f64,
        })
    }
}

struct State {
    c: Ramp,
    seed: f64,
}

impl EffectState for State {
    fn draw(&self, p: &mut Painter, t: f64, layer: Layer) {
        if layer != Layer::Back {
            draw(p, t, &self.c, self.seed);
        }
    }
}

// the flow at radius rho (0 eye, 1 rim), angle th: thin streaks of foam laid along a log spiral that turns once a
// loop (a turning log spiral reads as water drawn inward), dashed and brightest where two sets of streaks cross
fn foam(rho: f64, th: f64, ph: f64) -> f64 {
    let s = libm::log(js_max(rho, 0.02));
    let a = th + K * s + ph * TAU;
    let b = th + K * 1.35 * s + ph * TAU * 2.0;
    let one = libm::pow(
        0.5 + 0.5 * libm::cos(7.0 * a + 0.9 * libm::sin(3.0 * a + s * 4.0)),
        26.0,
    );
    let two = libm::pow(0.5 + 0.5 * libm::cos(11.0 * b + 1.3), 34.0) * 0.7;
    // finer streaks crowd in near the eye, where the water runs fastest
    let three = libm::pow(
        0.5 + 0.5 * libm::cos(17.0 * (th + K * 0.8 * s + ph * TAU * 3.0)),
        30.0,
    ) * 0.8
        * js_max(0.0, 1.0 - (rho - EYE) * 2.2);
    let dash = 0.55 + 0.45 * libm::sin(s * 22.0 - ph * TAU * 4.0 + a * 2.0);
    (one + two + three) * dash
}

fn draw(p: &mut Painter, t: f64, c: &Ramp, seed: f64) {
    let ph = t / D;
    let ry = R * SY;
    shade_with(p, -R, -ry, R, ry, |p, x, y, out| {
        let (dx, dy) = (x / R, y / ry);
        let rho = js_hypot(dx, dy);
        if rho > 1.0 {
            return;
        }
        let th = libm::atan2(dy, dx);
        // the eye: a hole down into the dark, its far wall lit, a ring of foam racing round its lip
        if rho < EYE {
            let far = -dy / rho;
            // the far wall of the funnel, lit, falling off into the dark below the lip on the near side
            let wall = (rho / EYE - 0.35) / 0.65;
            out.c = if far > 0.0 {
                if wall > 0.8 {
                    if far > 0.6 { c[5] } else { c[4] }
                } else if wall > 0.45 {
                    c[3]
                } else if wall > 0.15 {
                    c[2]
                } else {
                    c[0]
                }
            } else if wall > 0.85 && far > -0.5 {
                c[2]
            } else {
                c[0]
            };
            return;
        }
        let k = (rho - EYE) / (1.0 - EYE);
        let fade = 1.0 - smooth(0.62, 1.0, k);
        // dark see-through water, darker toward the eye, a faint sheen mid-way
        p.blend(
            x,
            y,
            if k < 0.25 { c[0] } else { c[1] },
            (0.75 - 0.25 * k) * (1.0 - smooth(0.8, 1.0, k)),
        );
        if k > 0.3 && k < 0.8 {
            p.blend(x, y, c[2], 0.18 * libm::sin(PI * (k - 0.3) / 0.5));
        }
        // the foam ring round the eye
        if k < 0.035 {
            let g = 0.5 + 0.5 * libm::cos(th * 9.0 - ph * TAU * 6.0);
            out.c = if g > 0.7 {
                WHITE
            } else if g > 0.3 {
                c[6]
            } else {
                c[4]
            };
            return;
        }
        let i = foam(rho, th, ph)
            * fade
            * js_min(1.0, (k - 0.08) * 4.0)
            * (0.45 + 0.55 * libm::sin(PI * js_min(1.0, k * 1.25)));
        if i > 0.12 {
            out.c = if i > 0.75 {
                WHITE
            } else if i > 0.45 {
                c[6]
            } else if i > 0.25 {
                c[5]
            } else {
                c[4]
            };
            if i > 0.25 {
                out.l = c[5];
                out.k = 0.08 + 0.14 * js_min(1.0, i);
            }
        }
    });
    // flecks of foam drawn round and in along the arms, each a short streak along its path
    cycle(
        t,
        D,
        30,
        seed,
        |g| g.range(1.2, 2.2),
        |age, life, r, _| {
            let u = age / life;
            let rho = 0.8 - 0.62 * u * u;
            let a0 = r.range(0.0, TAU);
            let th = a0 - K * libm::log(rho);
            let ang = th - age * (1.5 + 2.2 / rho);
            let (x, y) = (libm::cos(ang) * rho * R, libm::sin(ang) * rho * ry);
            let ang2 = ang + 0.12 / rho;
            let (x2, y2) = (libm::cos(ang2) * rho * R, libm::sin(ang2) * rho * ry);
            if u > 0.85 && r.next() < (u - 0.85) * 6.0 {
                return;
            }
            let col = if u < 0.2 {
                c[5]
            } else if u < 0.6 {
                WHITE
            } else {
                c[6]
            };
            p.line(x2, y2, x, y, col, 1.0);
        },
    );
    // glints winking on the foam
    cycle(
        t,
        D,
        8,
        seed + 3.0,
        |g| g.range(0.25, 0.45),
        |age, life, r, _| {
            let rho = r.range(0.35, 0.8);
            let th = r.range(0.0, TAU) - age * 2.5;
            let k = libm::sin((PI * age) / life);
            let (x, y) = (libm::cos(th) * rho * R, libm::sin(th) * rho * ry);
            if foam(rho, th, ph) < 0.35 {
                return;
            }
            p.px(x, y, WHITE);
            if k > 0.55 {
                for (dx, dy) in CROSS {
                    p.px(x + dx, y + dy, c[6]);
                }
            }
            p.glow(x, y, 5.0, c[5], 0.4 * k);
        },
    );
    // light: a cool sheen on the swirl, a glow off the eye's lit rim
    p.glow(0.0, 0.0, R * 0.9, c[4], 0.2);
    p.glow(0.0, 0.0, R * EYE * 1.6 + 6.0, c[5], 0.28);
    p.glow(0.0, -ry * EYE * 0.8, 14.0, c[5], 0.32);
}
