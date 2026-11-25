//! Rain splash: port of pixelfx `src/fx/water/rain-splash.js`.
//!
//! The streaks falling in (a dim body, a bright head) follow Sprite Fusion's open-source rain
//! (https://www.spritefusion.com/pixel-art-vfx/rain), MIT License, Copyright (c) 2026 Hugo Duprez.

use core::f64::consts::PI;

use pixfx_core::particles::cycle;
use pixfx_core::{Effect, EffectState, Layer, Meta, Options};
use pixfx_draw::Painter;
use pixfx_draw::color::{WHITE, ramp};
use pixfx_draw::math::{TAU, clamp, js_max, js_min, rand};

use super::lib::{Ramp, RingOpts, WATER, bead, js_hypot, ring};

pub struct RainSplash;

pub static RAIN_SPLASH: RainSplash = RainSplash;

const META: Meta = Meta {
    id: "water/rain-splash",
    name: "Rain splash",
    desc: "A low close-up on one puddle in the rain: big drops land in crowns that throw beads and a rebound jet, rings spread and interfere, a lamp's reflection wobbles on the water.",
    tags: &[
        "water", "rain", "splash", "puddle", "close-up", "ambient", "loop",
    ],
    size: (176, 80),
    origin: (88, 50),
    duration: 2.4,
    looping: true,
    fps: 30,
    seed: 9,
    state: None,
    options: &[],
};

const D: f64 = 2.4;
const SY: f64 = 0.3;
const RX: f64 = 92.0;
const G: f64 = 640.0;
const N: usize = 8;
const LAMP: f64 = 34.0;

impl Effect for RainSplash {
    fn meta(&self) -> &Meta {
        &META
    }
    fn spawn(&self, opts: &Options, seed: u64) -> Box<dyn EffectState> {
        let s = seed as f64;
        Box::new(State {
            c: ramp(&opts.palette_or(&WATER)),
            s,
            drops: drops(s),
        })
    }
}

struct State {
    c: Ramp,
    s: f64,
    drops: [Big; N],
}

impl EffectState for State {
    fn draw(&self, p: &mut Painter, t: f64, layer: Layer) {
        if layer != Layer::Front {
            draw_back(p, t, self);
        }
        if layer != Layer::Back {
            draw(p, t, self);
        }
    }
}

// the big drops: where each lands on the puddle (x across, z into it), when, how tall its crown stands
#[derive(Clone, Copy)]
struct Big {
    x: f64,
    z: f64,
    t0: f64,
    h: f64,
    j: f64,
    spin: f64,
    i: f64,
}

fn drops(s: f64) -> [Big; N] {
    core::array::from_fn(|i| {
        let fi = i as f64;
        Big {
            x: (rand(&[fi, s]) - 0.5) * 120.0,
            z: (rand(&[fi, s + 1.0]) - 0.5) * 50.0,
            t0: ((fi + 0.25 * rand(&[fi, s + 2.0])) / N as f64) * D,
            h: 10.0 + 4.0 * rand(&[fi, s + 3.0]),
            j: 7.0 + 4.0 * rand(&[fi, s + 4.0]),
            spin: rand(&[fi, s + 5.0]) * TAU,
            i: fi,
        }
    })
}

fn age_of(t: f64, t0: f64) -> f64 {
    (((t - t0) % D) + D) % D
}
// a ring's radius and strength at age a
fn ring_r(a: f64) -> f64 {
    3.0 + 34.0 * libm::pow(a / 1.8, 0.7)
}
fn ring_k(a: f64) -> f64 {
    if a < 1.8 {
        1.3 * libm::pow(1.0 - a / 1.8, 1.3) * js_min(1.0, a / 0.06)
    } else {
        0.0
    }
}
// the crown at age a: how high its walls stand and how wide its foot is
fn crown_h(a: f64, h: f64) -> f64 {
    h * libm::pow(libm::sin(PI * clamp(a / 0.3, 0.0, 1.0)), 0.8) * (if a < 0.3 { 1.0 } else { 0.0 })
}
fn crown_r(a: f64) -> f64 {
    1.5 + 4.5 * libm::sqrt(clamp(a / 0.3, 0.0, 1.0))
}
// the rebound jet: up and back down after the crown has fallen
fn jet_h(a: f64, j: f64) -> f64 {
    if a > 0.26 && a < 0.56 {
        j * libm::sin((PI * (a - 0.26)) / 0.3)
    } else {
        0.0
    }
}

/// A ring alive now: where its drop landed, its age, radius and strength.
struct Live {
    x: f64,
    z: f64,
    a: f64,
    r: f64,
    k: f64,
}

// the water's height at a point: every ring's swell summed, so where two cross they add and cancel
fn field(x: f64, z: f64, list: &[&Live]) -> f64 {
    let mut h = 0.0;
    for q in list {
        let dx = x - q.x;
        if dx > q.r + 10.0 || dx < -q.r - 10.0 {
            continue;
        }
        let d = js_hypot(dx, z - q.z) - q.r;
        let w = 3.6 + q.a * 3.0;
        if d > 2.0 * w || d < -3.0 * w {
            continue;
        }
        h += q.k * libm::cos(d * 1.1) * libm::exp(-libm::pow(d / w, 2.0));
    }
    h
}

fn draw_back(p: &mut Painter, t: f64, o: &State) {
    let c = &o.c;
    let ry = RX * SY;
    let ph = t / D;
    let list: Vec<Live> = o
        .drops
        .iter()
        .map(|q| (q, age_of(t, q.t0)))
        .filter(|&(_, a)| a < 1.8)
        .map(|(q, a)| Live {
            x: q.x,
            z: q.z,
            a,
            r: ring_r(a),
            k: ring_k(a),
        })
        .collect();
    let mut yy = libm::floor(-ry);
    while yy < ry {
        let y = yy + 0.5;
        let z = y / SY;
        let act: Vec<&Live> = list
            .iter()
            .filter(|q| libm::fabs(z - q.z) <= q.r + 12.0)
            .collect();
        let far = (y + ry) / (2.0 * ry);
        // the lamp's reflection on this row: a dash of light, its width shimmering, pushed about by the swell
        let dash_w = (1.5 + far * 4.0)
            * (0.35
                + 0.65
                    * (0.5
                        + 0.5
                            * libm::sin(yy * 1.73 + ph * TAU * 3.0 + libm::sin(yy * 0.61) * 3.0)));
        let mut h_prev = 0.0;
        let mut xx = -88.0;
        while xx < 88.0 {
            let x = xx + 0.5;
            let col_x = xx;
            xx += 1.0;
            let d = libm::pow(x / RX, 2.0) + libm::pow(y / ry, 2.0);
            if d > 1.0 {
                continue;
            }
            let edge = js_min(1.0, (1.0 - d) * 6.0);
            // the puddle: dark see-through water, the far part holding a little of the sky
            p.blend(x, y, c[0], (0.62 + 0.2 * far) * edge);
            if far < 0.45 {
                p.blend(x, y, c[2], 0.3 * (1.0 - far / 0.45) * edge);
            }
            // its swell: every ring summed, so crests catch the light, troughs go dark, and crossings beat
            let h = if act.is_empty() {
                0.0
            } else {
                field(x, z, &act)
            };
            let hx = h - h_prev;
            h_prev = h;
            if h > 0.24 {
                p.blend(
                    x,
                    y,
                    if h > 0.75 { c[5] } else { c[4] },
                    js_min(0.95, (h - 0.24) * 2.2) * edge,
                );
            } else if h < -0.4 {
                p.blend(x, y, c[0], js_min(0.85, (-h - 0.4) * 1.8) * edge);
            }
            if x < LAMP - 14.0 || x > LAMP + 14.0 {
                continue;
            }
            let lx = libm::fabs(x - LAMP - hx * 9.0 - libm::sin(y * 0.8 + ph * TAU * 2.0) * 1.4);
            if lx < dash_w {
                let k = (1.0 - lx / dash_w) * edge;
                let col = if k > 0.72 {
                    WHITE
                } else if k > 0.35 {
                    c[6]
                } else {
                    c[5]
                };
                p.px(col_x, yy, col);
                p.light(x, y, c[5], 0.2 * k);
            }
        }
        yy += 1.0;
    }
    // the far rim of the puddle catches the light
    let mut xx = -82.0;
    while xx < 82.0 {
        let x = xx + 0.5;
        let y = -libm::sqrt(js_max(0.0, 1.0 - libm::pow(x / RX, 2.0))) * ry;
        p.px(
            xx,
            y + 0.5,
            if libm::fabs(x - LAMP) < 10.0 {
                c[6]
            } else {
                c[3]
            },
        );
        xx += 1.0;
    }
    // the back halves of the crowns, behind their own front walls
    for q in &o.drops {
        crown(p, c, q, age_of(t, q.t0), true);
    }
}

// a crown: a ring of fingers thrown up round the drop's hole, a bead on each tip until they tear away
fn crown(p: &mut Painter, c: &Ramp, q: &Big, a: f64, back: bool) {
    let h = crown_h(a, q.h);
    if h < 1.0 {
        return;
    }
    let r0 = crown_r(a);
    let cy = q.z * SY;
    let (c3, c5, c6) = (c[3], c[5], c[6]);
    for f in 0..10 {
        let ff = f64::from(f);
        let ang = q.spin + (ff / 10.0) * TAU;
        let s = libm::sin(ang);
        if (s < 0.0) != back {
            continue;
        }
        let tip = h * (0.75 + 0.25 * rand(&[q.i, ff, 7.0]));
        let lean = 0.35 * tip;
        let (x0, y0) = (q.x + libm::cos(ang) * r0, cy + s * r0 * SY);
        let x1 = q.x + libm::cos(ang) * (r0 + lean);
        let y1 = cy + s * (r0 + lean) * SY - tip;
        if back {
            p.line(x0, y0, x1, y1, |u: f64| if u > 0.7 { c5 } else { c3 }, 1.0);
        } else {
            p.line(
                x0,
                y0,
                x1,
                y1,
                |u: f64| {
                    if u > 0.75 {
                        WHITE
                    } else if u > 0.35 {
                        c6
                    } else {
                        c5
                    }
                },
                1.0,
            );
        }
        if a > 0.08 && a < 0.17 {
            p.px(x1, y1 - 1.0, if back { c[5] } else { WHITE });
        }
    }
    if !back {
        p.glow(q.x, cy - h * 0.6, 6.0 + h * 0.6, c[5], 0.3 * (h / q.h));
    }
}

fn draw(p: &mut Painter, t: f64, o: &State) {
    let (c, s) = (&o.c, o.s);
    for q in &o.drops {
        let a = age_of(t, q.t0);
        let cy = q.z * SY;
        // falling in: a streak dropping into the frame just before it lands
        let pre = D - a;
        if pre < 0.14 && cy - pre * 520.0 - 10.0 > -48.0 {
            let y = cy - pre * 520.0;
            p.line(q.x, y - 10.0, q.x, y - 4.0, c[4], 1.0);
            p.line(q.x, y - 4.0, q.x, y, c[6], 1.0);
        }
        // the hit: a white fleck and a flash of light, then the crown
        if a < 0.05 {
            p.px(q.x, cy, WHITE);
            p.glow(q.x, cy - 2.0, 12.0, c[5], 0.5 * (1.0 - a / 0.05));
        }
        crown(p, c, q, a, false);
        // beads torn off the crown's tips arc out and fall back, each with a fleck and a little ring
        if a > 0.16 && a < 0.8 {
            for f in (0..10).step_by(2) {
                let ff = f64::from(f);
                let ang = q.spin + (ff / 10.0) * TAU;
                let tip = crown_h(0.16, q.h) * (0.75 + 0.25 * rand(&[q.i, ff, 7.0]));
                let r0 = crown_r(0.16) + tip * 0.35;
                let v = 22.0 + 18.0 * rand(&[q.i, ff, 8.0]);
                let vh = 40.0 + 30.0 * rand(&[q.i, ff, 9.0]);
                let tt = a - 0.16;
                let hh = tip + vh * tt - 0.5 * G * tt * tt;
                let bx = q.x + libm::cos(ang) * (r0 + v * tt);
                let bz = q.z + libm::sin(ang) * (r0 + v * tt);
                let by = bz * SY;
                if hh > 0.0 {
                    bead(
                        p,
                        bx,
                        by - hh,
                        libm::cos(ang) * v,
                        -(vh - G * tt) + libm::sin(ang) * v * SY,
                        if rand(&[q.i, ff, 10.0]) < 0.4 {
                            2.0
                        } else {
                            1.0
                        },
                        c,
                        1.0,
                    );
                } else {
                    let land = (vh + libm::sqrt(vh * vh + 2.0 * G * tip)) / G;
                    let since = tt - land;
                    if since < 0.05 {
                        p.px(bx, by, WHITE);
                    }
                    if since < 0.35 {
                        ring(
                            p,
                            bx,
                            by,
                            1.0 + since * 12.0,
                            SY,
                            1.0 - since / 0.35,
                            c,
                            RingOpts {
                                n: 1.0,
                                half: 1.0,
                                seed: q.i * 10.0 + ff,
                                ..RingOpts::default()
                            },
                        );
                    }
                }
            }
        }
        // the rebound jet, with a bead that pinches off its top and drops back
        let jh = jet_h(a, q.j);
        if jh > 0.8 {
            let c6 = c[6];
            p.line(
                q.x,
                cy - 1.0,
                q.x,
                cy - jh,
                |u: f64| if u > 0.6 { WHITE } else { c6 },
                1.0,
            );
            p.px(q.x + 1.0, cy - 1.0, c[5]);
            if a < 0.41 {
                p.circle(q.x, cy - jh - 0.5, 1.2, c[6], 1.0);
            }
            p.glow(q.x, cy - jh * 0.6, 6.0, c[5], 0.3);
        }
        let ba = a - 0.41;
        let bh = q.j + 1.0 + 24.0 * ba - 0.5 * G * ba * ba;
        if ba > 0.0 && bh > 0.0 {
            p.circle(q.x, cy - bh, 1.1, c[6], 1.0);
            p.px(q.x, cy - bh - 1.0, WHITE);
        }
    }
    // the near halves of the main rings, crisp, so each splash has its own edge in the swell
    for q in &o.drops {
        let a = age_of(t, q.t0);
        if a < 1.8 {
            ring(
                p,
                q.x,
                q.z * SY,
                ring_r(a),
                SY,
                0.75 * ring_k(a),
                c,
                RingOpts {
                    n: 1.0,
                    half: 1.0,
                    hot: a < 0.1,
                    seed: q.i,
                    wob: 0.5 + a,
                    ..RingOpts::default()
                },
            );
        }
    }
    // finer rain dimpling the water, and a few streaks falling past
    let (c3, c5) = (c[3], c[5]);
    cycle(
        t,
        D,
        16,
        s + 20.0,
        |_| 0.5,
        |age, _life, r, _| {
            let x = r.range(-80.0, 80.0);
            let z = r.range(-26.0, 26.0);
            let y = z * SY;
            let s = age - 0.1;
            if s < 0.0 {
                let yy = y - (0.1 - age) * 600.0;
                if yy - 6.0 < -48.0 {
                    return;
                }
                p.line(
                    x,
                    yy - 6.0,
                    x,
                    yy,
                    |u: f64| if u > 0.6 { c5 } else { c3 },
                    1.0,
                );
            } else if s < 0.4 && libm::pow(x / RX, 2.0) + libm::pow(y / (RX * SY), 2.0) < 0.9 {
                if s < 0.04 {
                    p.px(x, y, c[6]);
                }
                ring(
                    p,
                    x,
                    y,
                    1.0 + s * 9.0,
                    SY,
                    0.8 * (1.0 - s / 0.4),
                    c,
                    RingOpts {
                        n: 1.0,
                        half: 1.0,
                        seed: 40.0,
                        ..RingOpts::default()
                    },
                );
            }
        },
    );
    // the lamp's glow on the water
    p.glow(LAMP, -6.0, 30.0, c[4], 0.16);
}
