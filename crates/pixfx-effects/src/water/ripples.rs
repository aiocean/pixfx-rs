//! Ripples: port of pixelfx `src/fx/water/ripples.js`.

use pixfx_core::particles::cycle;
use pixfx_core::{Effect, EffectState, Layer, Meta, Options};
use pixfx_draw::Painter;
use pixfx_draw::color::{WHITE, ramp};
use pixfx_draw::math::{ease, js_max, js_min, js_round, smooth};

use super::lib::{CROSS, Ramp, RingOpts, WATER, js_hypot, ring};

pub struct Ripples;

pub static RIPPLES: Ripples = Ripples;

const META: Meta = Meta {
    id: "water/ripples",
    name: "Ripples",
    desc: "A drop plinks on still dark water and rings spread out across it at a low angle: the swell of each ring catches the light, crests glint, and they fade into the calm.",
    tags: &[
        "water", "ripples", "rings", "surface", "pond", "ambient", "loop",
    ],
    size: (136, 52),
    origin: (68, 30),
    duration: 2.0,
    looping: true,
    fps: 30,
    seed: 2,
    state: None,
    options: &[],
};

const D: f64 = 2.0;
const SY: f64 = 0.26;
const EVERY: f64 = 0.5;
const LIFE: f64 = 2.0;
const RMAX: f64 = 60.0;
const PATCH: f64 = 64.0;

impl Effect for Ripples {
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
        if layer != Layer::Front {
            draw_back(p, t, &self.c);
        }
        if layer != Layer::Back {
            draw(p, t, &self.c, self.seed);
        }
    }
}

// one ring at age a: its radius (fast out, slowing) and strength
fn radius(a: f64) -> f64 {
    2.0 + (RMAX - 2.0) * ease::out_cubic(a / LIFE)
}
fn power(a: f64) -> f64 {
    libm::pow(1.0 - a / LIFE, 1.5) * js_min(1.0, a / 0.08)
}

// the rings alive at t, newest first: [age, index]
fn alive(t: f64) -> Vec<(f64, f64)> {
    let mut out = Vec::new();
    let mut k = 0.0;
    while k * EVERY < LIFE {
        let a = (((t - k * EVERY) % D) + D) % D;
        if a < LIFE {
            out.push((a, js_round(((((t - a) % D) + D) % D) / EVERY)));
        }
        k += 1.0;
    }
    out
}

// the water: dark and see-through, shading with the slope of the swell each ring carries, so the rings read as
// waves catching the light from the far left, not just lines
fn surface(p: &mut Painter, c: &Ramp, t: f64) {
    let rs: Vec<(f64, f64, f64)> = alive(t)
        .into_iter()
        .map(|(a, _)| (radius(a), power(a), 3.0 + a * 2.2))
        .collect();
    let mut yy = libm::floor(-PATCH * SY);
    while yy < PATCH * SY {
        let mut xx = -PATCH;
        while xx < PATCH {
            let (x, y) = (xx + 0.5, yy + 0.5);
            xx += 1.0;
            let z = y / SY;
            let r = js_hypot(x, z);
            let e = r / PATCH;
            if e > 1.0 {
                continue;
            }
            let edge = 1.0 - smooth(0.6, 1.0, e);
            p.blend(x, y, c[1], 0.55 * edge);
            let mut slope = 0.0;
            for &(rr, k, w) in &rs {
                let d = r - rr;
                if d > 2.2 * w || d < -2.2 * w {
                    continue;
                }
                slope += k * libm::exp(-libm::pow(d / w, 2.0)) * -libm::sin(d * 1.15) * 1.15;
            }
            if slope == 0.0 {
                continue;
            }
            // the side of a swell facing the light (back and left) catches it; the side away from it goes dark
            let face = slope * (-(x * -0.55 + z * -0.84) / js_max(1.0, r));
            if face > 0.18 {
                p.blend(
                    x,
                    y,
                    if face > 0.6 { c[5] } else { c[4] },
                    js_min(0.9, (face - 0.18) * 2.2) * edge,
                );
            } else if face < -0.25 {
                p.blend(x, y, c[0], js_min(0.8, (-face - 0.25) * 2.0) * edge);
            }
        }
        yy += 1.0;
    }
}

fn rings(p: &mut Painter, c: &Ramp, t: f64, half: f64) {
    for (a, i) in alive(t) {
        let (r, k) = (radius(a), power(a));
        ring(
            p,
            0.0,
            0.0,
            r,
            SY,
            0.85 * k,
            c,
            RingOpts {
                n: 2.0,
                gap: 3.0 + a * 1.4,
                half,
                hot: a < 0.14,
                glint: if half > 0.0 && a > 0.1 { k } else { 0.0 },
                seed: i + 1.0,
                wob: 0.4 + a * 1.1,
            },
        );
    }
}

fn draw_back(p: &mut Painter, t: f64, c: &Ramp) {
    surface(p, c, t);
    rings(p, c, t, -1.0);
}

fn draw(p: &mut Painter, t: f64, c: &Ramp, s: f64) {
    rings(p, c, t, 1.0);
    // each ring starts with a drop: a white fleck, a bead thrown up and back down, a little light
    for (a, _) in alive(t) {
        if a < 0.08 {
            p.px(0.0, 0.0, WHITE);
            p.px(-1.0, 0.0, c[6]);
            p.px(1.0, 0.0, c[6]);
            p.glow(0.0, 0.0, 8.0, c[5], 0.5 * (1.0 - a / 0.08));
        }
        let b = a - 0.04;
        let h = 90.0 * b - 0.5 * 700.0 * b * b;
        if b > 0.0 && h > 0.0 {
            p.px(0.0, -h, WHITE);
            p.px(0.0, -h + 1.0, c[5]);
        }
    }
    // glints ride the near crests, winking
    let all = alive(t);
    cycle(
        t,
        D,
        10,
        s + 5.0,
        |g| g.range(0.25, 0.45),
        |age, life, r, _| {
            let idx = libm::floor(r.next() * all.len() as f64) as usize;
            let a = all.get(idx).map_or(0.0, |q| q.0);
            let ang = r.range(0.3, core::f64::consts::PI - 0.3);
            let rr = radius(a);
            let k = libm::sin((core::f64::consts::PI * age) / life) * power(a);
            if k < 0.15 || rr < 6.0 {
                return;
            }
            let (x, y) = (libm::cos(ang) * rr, libm::sin(ang) * rr * SY);
            p.px(x, y, WHITE);
            if k > 0.5 {
                for (dx, dy) in CROSS {
                    p.px(x + dx, y + dy, c[6]);
                }
            }
            p.glow(x, y, 5.0, c[5], 0.4 * k);
        },
    );
    // a cool light on the water, brightest where the rings start
    p.glow(0.0, 0.0, 40.0, c[4], 0.14);
    p.glow(0.0, 0.0, 14.0, c[5], 0.12);
}
