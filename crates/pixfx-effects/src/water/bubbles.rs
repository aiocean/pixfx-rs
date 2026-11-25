//! Bubbles: port of pixelfx `src/fx/water/bubbles.js`.
//!
//! The bubble shading (a lit rim top-left, a shaded rim bottom-right, a see-through body, a highlight and a
//! refraction fleck, 2x2 and 1 px forms when small) and the surface line are adapted from Sprite Fusion's
//! open-source bubbles (https://www.spritefusion.com/pixel-art-vfx/bubbles), MIT License, Copyright (c) 2026 Hugo
//! Duprez.

use core::f64::consts::{PI, SQRT_2};

use pixfx_core::particles::cycle;
use pixfx_core::{Effect, EffectState, Layer, Meta, Options};
use pixfx_draw::Painter;
use pixfx_draw::color::{WHITE, ramp};
use pixfx_draw::math::{TAU, clamp, js_max, js_min, js_round, smooth, to_int32};

use super::lib::{ROCK, Ramp, WATER, js_hypot, shade_with};

pub struct Bubbles;

pub static BUBBLES: Bubbles = Bubbles;

const META: Meta = Meta {
    id: "water/bubbles",
    name: "Bubbles",
    desc: "Bubbles stream up from a vent of pebbles, wobbling and swelling as they rise, flare as they cross shafts of light from the surface, then dome up and pop there.",
    tags: &["water", "bubbles", "underwater", "vent", "ambient", "loop"],
    size: (104, 112),
    origin: (52, 104),
    duration: 4.0,
    looping: true,
    fps: 30,
    seed: 4,
    state: None,
    options: &[],
};

const D: f64 = 4.0;
const TOP: f64 = -92.0;
const HALF: f64 = 48.0;

impl Effect for Bubbles {
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

// the surface: a wavy lit line, a darker band under it
fn surface_y(x: f64, ph: f64) -> f64 {
    TOP + libm::sin(x * 0.13 + ph * TAU * 2.0) * 1.1 + libm::sin(x * 0.051 - ph * TAU) * 1.3
}

// light shafts slanting down from the surface, drifting: how much of one a point is in
fn shaft(x: f64, y: f64, ph: f64) -> f64 {
    let u = x + (y - TOP) * 0.32;
    let s = libm::sin(u * 0.16 + ph * TAU) * 0.6 + libm::sin(u * 0.07 - ph * TAU + 1.0) * 0.4;
    js_max(0.0, s - 0.35) / 0.65
}

// one bubble at (x, y), radius r, squash q: tiny ones a pixel or a shaded 2x2, bigger ones a lit rim round a
// see-through body with a highlight and a fleck of light refracted through it
fn bubble(p: &mut Painter, c: &Ramp, x: f64, y: f64, r: f64, q: f64, lit: f64) {
    if r < 0.9 {
        let col = if lit > 0.3 {
            WHITE
        } else if r < 0.7 {
            c[5]
        } else {
            c[6]
        };
        return p.px(x, y, col);
    }
    if r < 1.5 {
        let (bx, by) = (libm::floor(x), libm::floor(y));
        p.px(bx, by, WHITE);
        p.px(bx + 1.0, by, c[6]);
        p.px(bx, by + 1.0, c[5]);
        p.px(bx + 1.0, by + 1.0, c[4]);
        p.glow(x + 0.5, y + 0.5, 3.0, c[5], 0.12 + 0.2 * lit);
        return;
    }
    let (rx, ry) = (r * (1.0 + q), r * (1.0 - q));
    shade_with(
        p,
        x - rx - 1.0,
        y - ry - 1.0,
        x + rx + 1.0,
        y + ry + 1.0,
        |p, px, py, out| {
            let (dx, dy) = ((px - x) / rx, (py - y) / ry);
            let d = js_hypot(dx, dy) * r;
            if d > r + 0.5 {
                return;
            }
            let l = (-dx - dy) / SQRT_2 / js_max(0.01, d / r);
            if d > r - 0.6 {
                out.c = if l > 0.45 {
                    c[6]
                } else if l < -0.55 {
                    c[3]
                } else {
                    c[4]
                };
            } else {
                p.blend(px, py, c[5], 0.16 + 0.2 * lit);
            }
        },
    );
    p.px(js_round(x - rx * 0.45), js_round(y - ry * 0.45), WHITE);
    if r >= 2.5 {
        p.px(js_round(x - rx * 0.45) + 1.0, js_round(y - ry * 0.45), c[6]);
    }
    if r >= 2.0 {
        p.px(js_round(x + rx * 0.5), js_round(y + ry * 0.5), c[6]);
    }
    p.glow(x, y, r * 2.4 + 2.0, c[5], 0.12 + 0.3 * lit);
}

// the bubbles, from the vent's three mouths: each rises faster the bigger it is, swells as the pressure drops,
// wobbles side to side, and pops at the surface
const MOUTHS: [f64; 3] = [-5.0, 1.0, 6.0];

struct Bubble {
    x: f64,
    y: f64,
    r: f64,
    q: f64,
    vy: f64,
}

fn each(t: f64, s: f64, mut f: impl FnMut(Bubble)) {
    cycle(
        t,
        D,
        46,
        s,
        |g| g.range(2.2, 3.4),
        |age, _life, r, i| {
            let m = MOUTHS[i % 3];
            let r0 = if r.next() < 0.7 {
                r.range(0.5, 1.4)
            } else {
                r.range(1.6, 2.6)
            };
            let vy = 30.0 + 9.0 * r0 + r.range(-4.0, 4.0);
            let wf = r.range(3.0, 6.0);
            let wa = 0.8 + r0 * 1.2;
            let ph0 = r.range(0.0, TAU);
            let drift = r.range(-4.0, 4.0);
            let y = -2.0 - vy * age;
            let k = clamp(-y / -TOP, 0.0, 1.0);
            let x = m + drift * k + libm::sin(age * wf + ph0) * wa * k;
            f(Bubble {
                x,
                y,
                r: r0 * (1.0 + 0.45 * k),
                q: 0.12 * libm::sin(age * wf * 2.0 + ph0) * js_min(1.0, r0 / 2.0),
                vy,
            });
        },
    );
}

const VENT: [(f64, f64, f64); 6] = [
    (-9.0, -1.5, 2.4),
    (-4.0, -1.0, 1.8),
    (3.5, -1.2, 2.2),
    (9.0, -1.4, 2.6),
    (0.0, -0.6, 1.4),
    (-12.5, -0.8, 1.6),
];

fn draw_back(p: &mut Painter, t: f64, c: &Ramp) {
    let ph = t / D;
    // the water: deeper toward the floor, light falling through it in shafts
    let mut yy = TOP - 2.0;
    while yy < 0.0 {
        let mut xx = -HALF;
        while xx < HALF {
            let (x, y) = (xx + 0.5, yy + 0.5);
            xx += 1.0;
            let sy = surface_y(x, ph);
            if y < sy {
                continue;
            }
            let e = 1.0 - smooth(HALF - 14.0, HALF, libm::fabs(x));
            let depth = (y - TOP) / -TOP;
            p.blend(x, y, c[1], (0.22 + 0.22 * depth) * e);
            let s = shaft(x, y, ph) * (1.0 - depth * 0.7) * e;
            if s > 0.05 {
                p.light(x, y, c[4], 0.2 * s);
            }
        }
        yy += 1.0;
    }
    // where the shafts reach the floor, caustic light dances on it
    let mut xx = -HALF;
    while xx < HALF {
        let x = xx + 0.5;
        xx += 1.0;
        let e = 1.0 - smooth(HALF - 14.0, HALF, libm::fabs(x));
        let s = shaft(x, -2.0, ph);
        let cc = js_max(
            0.0,
            1.0 - libm::fabs(libm::sin(
                x * 0.42 + libm::sin(x * 0.11 + ph * TAU * 2.0) * 2.2,
            )) * 2.4,
        );
        if s * cc * e > 0.05 {
            p.light(x, -1.0, c[5], 0.5 * s * cc * e);
            p.light(x, -2.0, c[4], 0.25 * s * cc * e);
            p.light(x, 0.0, c[4], 0.3 * s * cc * e);
        }
    }
    // the vent: a little heap of pebbles round a dark mouth
    let st = &ROCK;
    for (x, y, rr) in VENT {
        p.shade(
            x - rr - 1.0,
            y - rr - 1.0,
            x + rr + 1.0,
            y + rr + 1.0,
            |px, py, out| {
                let (dx, dy) = (px - x, py - y);
                let d = js_hypot(dx * 1.1, dy);
                if d > rr + 0.7 || py > 0.0 {
                    return None;
                }
                out.c = if d > rr {
                    st[0]
                } else if dx + dy < -rr * 0.6 {
                    st[5]
                } else if dx + dy < rr * 0.3 {
                    st[3]
                } else {
                    st[2]
                };
                None
            },
        );
    }
    for m in MOUTHS {
        p.px(m, -1.0, st[0]);
    }
}

fn draw(p: &mut Painter, t: f64, c: &Ramp, s: f64) {
    let ph = t / D;
    // specks drifting in the water, flaring in the light
    cycle(
        t,
        D,
        26,
        s + 7.0,
        |_| D,
        |age, _life, r, _| {
            let x = r.range(-HALF + 6.0, HALF - 6.0)
                + libm::sin((age / D) * TAU + r.range(0.0, TAU)) * 3.0;
            let y = TOP + 6.0 + ((r.range(0.0, 86.0) + age * 4.0) % 86.0);
            let sh = shaft(x, y, ph);
            let tw = libm::sin((age / D) * TAU * 3.0 + r.range(0.0, TAU));
            if tw < -0.3 && sh < 0.2 {
                return;
            }
            let col = if sh > 0.4 {
                WHITE
            } else if tw > 0.6 {
                c[5]
            } else {
                c[4]
            };
            p.px(x, y, col);
            if sh > 0.4 {
                p.glow(x, y, 3.0, c[5], 0.25 * sh);
            }
        },
    );
    // the bubbles; when one reaches the surface it domes up, then pops with a ring and a couple of flecks
    each(t, s, |b| {
        let sy = surface_y(b.x, ph);
        if b.y > sy + b.r {
            bubble(p, c, b.x, b.y, b.r, b.q, shaft(b.x, b.y, ph));
            return;
        }
        let since = (sy + b.r - b.y) / b.vy;
        let big = b.r >= 1.4;
        if since < 0.12 && big {
            let mut a = PI;
            while a <= TAU + 0.01 {
                p.px(
                    b.x + libm::cos(a) * b.r,
                    sy + libm::sin(a) * b.r * 0.9,
                    c[6],
                );
                a += 0.4 / b.r;
            }
            p.px(b.x - b.r * 0.4, sy - b.r * 0.6, WHITE);
        } else if since < (if big { 0.32 } else { 0.2 }) {
            let k = clamp((since - (if big { 0.12 } else { 0.0 })) / 0.2, 0.0, 1.0);
            let w = js_max(1.0, b.r + 1.0 + k * 4.0);
            let mut a = 0.0;
            while a < PI {
                if libm::sin(a) > 0.15 {
                    p.dot(
                        b.x + libm::cos(a) * w,
                        sy + libm::sin(a) * w * 0.3,
                        if k < 0.4 { WHITE } else { c[5] },
                        1.2 - k,
                    );
                }
                a += 0.5 / w;
            }
            if big && k < 0.6 {
                p.px(b.x - 2.0, sy - 2.0 - k * 4.0, c[6]);
                p.px(b.x + 2.0, sy - 3.0 - k * 5.0, c[6]);
            }
        }
    });
    // the surface: a lit line with glints running along it, a darker band under it
    let mut xx = -HALF;
    while xx < HALF {
        let x = xx + 0.5;
        let y = surface_y(x, ph);
        let e = 1.0 - smooth(HALF - 12.0, HALF, libm::fabs(x));
        let glint = libm::sin(x * 0.37 - ph * TAU * 3.0) * libm::sin(x * 0.13 + ph * TAU) > 0.55;
        p.dot(xx, y, if glint { WHITE } else { c[5] }, e * 1.3);
        p.dot(xx, y + 1.0, c[3], e * 1.3);
        if to_int32(xx) & 3 == 0 {
            p.glow(x, y + 1.0, 7.0, c[5], (if glint { 0.22 } else { 0.1 }) * e);
        }
        xx += 1.0;
    }
}
