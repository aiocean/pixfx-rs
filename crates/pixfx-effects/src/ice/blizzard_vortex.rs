//! Blizzard vortex: port of pixelfx `src/fx/ice/blizzard-vortex.js`.
//!
//! Particles and bands that pass behind and in front of a centre (dim far half drawn back, bright near half drawn
//! front) follow the recipe in Sprite Fusion's open-source black hole
//! (https://www.spritefusion.com/pixel-art-vfx/black-hole), MIT License, Copyright (c) 2026 Hugo Duprez.

use pixfx_core::particles::cycle;
use pixfx_core::{Effect, EffectState, Layer, Meta, NumOpt, Options};
use pixfx_draw::Painter;
use pixfx_draw::color::{Rgba, WHITE, ramp};
use pixfx_draw::math::{TAU, clamp, js_max, js_min, js_round, rand};

use super::lib::PALETTE;

pub struct BlizzardVortex;

pub static BLIZZARD_VORTEX: BlizzardVortex = BlizzardVortex;

const META: Meta = Meta {
    id: "ice/blizzard-vortex",
    name: "Blizzard vortex",
    desc: "A swirling snow tornado: wind bands spin up a leaning funnel, flakes and ice crystals orbit in front and behind, snow boils at its foot and cold light fills its throat.",
    tags: &[
        "ice", "snow", "blizzard", "tornado", "vortex", "wind", "loop",
    ],
    size: (128, 116),
    origin: (64, 102),
    duration: D,
    looping: true,
    fps: 30,
    seed: 17,
    state: None,
    options: &[HEIGHT.spec()],
};

const HEIGHT: NumOpt = NumOpt::new("height", 80.0);
const D: f64 = 2.0;
/// How flat the funnel's rings look.
const TILT: f64 = 0.24;

impl Effect for BlizzardVortex {
    fn meta(&self) -> &Meta {
        &META
    }
    fn spawn(&self, opts: &Options, seed: u64) -> Box<dyn EffectState> {
        Box::new(State {
            c: ramp(&opts.palette_or(&PALETTE)),
            height: opts.num(&HEIGHT),
            seed: seed as f64,
        })
    }
}

struct State {
    c: [Rgba; 8],
    height: f64,
    seed: f64,
}

impl EffectState for State {
    fn draw(&self, p: &mut Painter, t: f64, layer: Layer) {
        match layer {
            Layer::Back => draw_back(p, t, self),
            Layer::Front => draw(p, t, self),
            Layer::Both => {
                draw_back(p, t, self);
                draw(p, t, self);
            }
        }
    }
}

/// The funnel's radius by height: a narrow foot, a wide mouth.
fn radius(h: f64, hh: f64) -> f64 {
    4.0 + 28.0 * libm::pow(clamp(h / hh, 0.0, 1.0), 1.35)
}

/// The funnel's axis, which sways (one full sway a loop).
fn axis(h: f64, hh: f64, ph: f64) -> f64 {
    (5.0 * libm::sin(TAU * ph + h * 0.05) + 1.5 * libm::sin(TAU * ph * 2.0 - h * 0.11)) * (h / hh)
        - 2.0 * (h / hh)
}

/// Wind streaks: arcs of snow-laden air swirling round the funnel and climbing it, a bright head and a fading
/// tail, faster low down; the far half of each arc goes behind, the near half in front.
fn streaks(p: &mut Painter, c: &[Rgba; 8], t: f64, o: &State, front: bool) {
    let (hh, ph) = (o.height, t / D);
    cycle(
        t,
        D,
        58,
        o.seed + 9.0,
        |g| g.range(0.5, 0.9),
        |age, life, r, _| {
            let h0 = r.range(2.0, hh);
            let w = r.range(8.0, 12.0) * (1.45 - h0 / hh);
            let th = r.range(0.0, TAU) + age * w;
            let k = age / life;
            let fade = js_min(1.0, age / 0.08) * if k > 0.7 { (1.0 - k) / 0.3 } else { 1.0 };
            let l = r.range(0.7, 1.5);
            let m = libm::ceil(l / 0.07);
            let mut j = 0.0;
            while j <= m {
                let a = th - j * 0.07;
                let s = libm::sin(a);
                let jj = j;
                j += 1.0;
                if (s >= 0.0) != front {
                    continue;
                }
                let h = js_min(hh + 4.0, h0 + age * 16.0 - jj * 0.25);
                let rr = radius(js_min(h, hh), hh)
                    * (1.0 + js_min(0.3, js_max(0.0, h - hh * 0.84) * 0.03));
                let u = jj / m;
                let x = axis(js_min(h, hh), hh, ph) + libm::cos(a) * rr;
                let y = -h + s * rr * TILT;
                let col = if front {
                    if u < 0.15 {
                        WHITE
                    } else if u < 0.5 {
                        c[6]
                    } else {
                        c[5]
                    }
                } else if u < 0.3 {
                    c[5]
                } else if u < 0.6 {
                    c[4]
                } else {
                    c[3]
                };
                p.dot(
                    x,
                    y,
                    col,
                    fade * (if u < 0.6 { 1.0 } else { 1.5 - u * 1.2 })
                        * (if front { 1.0 } else { 0.85 }),
                );
            }
        },
    );
}

/// The body: a translucent column of snow-filled air, densest at the edges where you look through more of it.
fn body(p: &mut Painter, c: &[Rgba; 8], ph: f64, hh: f64) {
    p.each(-44.0, -hh - 6.0, 44.0, 2.0, |p, x, y, sx, sy| {
        let h = -y;
        if h < 0.0 || h > hh + 4.0 {
            return;
        }
        let rr = radius(h, hh);
        let dx = x - axis(h, hh, ph);
        let u = dx / rr;
        if u * u >= 1.0 {
            return;
        }
        let k = h / hh;
        let rim = libm::pow(libm::fabs(u), 3.0);
        let swirl = libm::sin(libm::asin(u) * 3.0 - TAU * ph * 2.0 + h * 0.2) * 0.5 + 0.5;
        let a = (0.07 + 0.24 * rim + 0.12 * swirl)
            * clamp((1.0 - libm::fabs(u)) / 0.18, 0.0, 1.0)
            * (if k > 0.7 {
                js_max(0.0, (1.0 - k) / 0.3)
            } else {
                1.0
            })
            * (if h < 4.0 { h / 4.0 } else { 1.0 });
        if a < 0.03 {
            return;
        }
        let col = if rim > 0.5 {
            c[6]
        } else if swirl > 0.6 {
            c[5]
        } else {
            c[4]
        };
        p.blend_px(sx, sy, col, a);
        if rim > 0.3 || swirl > 0.75 {
            p.add(
                sx,
                sy,
                c[5],
                0.16 * js_max(rim, swirl - 0.5) * (1.0 - k * 0.5),
            );
        }
    });
}

/// Snow and ice carried round: rising on a spiral, faster low down, flung out at the top.
fn orbiters(p: &mut Painter, c: &[Rgba; 8], t: f64, o: &State, front: bool) {
    let (hh, ph) = (o.height, t / D);
    cycle(
        t,
        D,
        76,
        o.seed,
        |g| g.range(0.9, 1.6),
        |age, life, r, i| {
            let h0 = r.range(0.0, hh * 0.6);
            let h = h0 + age * r.range(14.0, 30.0);
            let k = age / life;
            if h > hh + 8.0 {
                return;
            }
            let w = r.range(7.0, 11.0) * (1.5 - h / hh);
            let th = r.range(0.0, TAU) + age * w;
            let out = 1.0 + r.range(0.05, 0.3) + js_min(0.35, js_max(0.0, h - hh * 0.82) * 0.04);
            let rr = radius(js_min(h, hh), hh) * out;
            let s = libm::sin(th);
            if (s >= 0.0) != front {
                return;
            }
            let ax = axis(js_min(h, hh), hh, ph);
            let x = ax + libm::cos(th) * rr;
            let y = -h + s * rr * TILT;
            let big = i % 5 == 0;
            let fade = js_min(1.0, age / 0.1) * if k > 0.8 { (1.0 - k) / 0.2 } else { 1.0 };
            if fade <= 0.0 {
                return;
            }
            // a streak back along its path, a bright head
            let th2 = th - 0.25;
            let x2 = ax + libm::cos(th2) * rr;
            let y2 = -h + 2.0 + libm::sin(th2) * rr * TILT;
            if front {
                p.line(
                    x2,
                    y2,
                    x,
                    y,
                    |u: f64| if u > 0.6 { c[6] } else { c[4] },
                    |u: f64| u * fade,
                );
                let col = if big || r.chance(0.5) { WHITE } else { c[6] };
                p.dot(x, y, col, fade);
                if big {
                    for (dx, dy) in [(1.0, 0.0), (-1.0, 0.0), (0.0, 1.0), (0.0, -1.0)] {
                        p.dot(x + dx, y + dy, c[6], fade);
                    }
                    p.glow(x, y, 5.0, c[5], 0.35 * fade);
                }
            } else {
                p.line(x2, y2, x, y, c[3], |u: f64| u * fade * 0.8);
                p.dot(x, y, if big { c[5] } else { c[4] }, fade);
            }
        },
    );
}

/// Snow boiling at the foot: soft puffs spinning round the base, lit on top.
fn foot(p: &mut Painter, c: &[Rgba; 8], t: f64, o: &State, front: bool) {
    let ph = t / D;
    cycle(
        t,
        D,
        12,
        o.seed + 5.0,
        |g| g.range(0.8, 1.2),
        |age, life, r, _| {
            let th = r.range(0.0, TAU) + age * r.range(4.0, 6.0);
            let rr = 9.0 + age * r.range(8.0, 16.0);
            let s = libm::sin(th);
            let k = age / life;
            if (s >= 0.0) != front {
                return;
            }
            let x = axis(0.0, 1.0, ph) + libm::cos(th) * rr;
            let y = -2.0 - age * r.range(4.0, 10.0) + s * rr * 0.2;
            let rad = 2.0 + 4.0 * k;
            let a = 0.5 * libm::pow(1.0 - k, 1.2) * js_min(1.0, age / 0.1);
            p.disc(
                x,
                y,
                rad,
                if front { c[6] } else { c[5] },
                |d: f64, _: f64| a * (1.0 - d * d),
            );
            p.disc(
                x - rad * 0.25,
                y - rad * 0.3,
                rad * 0.5,
                WHITE,
                |d: f64, _: f64| a * 0.5 * (1.0 - d * d) * (1.0 - k),
            );
        },
    );
}

fn draw_back(p: &mut Painter, t: f64, o: &State) {
    let c = &o.c;
    // a snow drift on the floor, swept round
    for xi in -30..=30 {
        let x = f64::from(xi);
        let d = libm::fabs(x) / 30.0;
        let hgt = js_round((1.0 - d * d) * (1.5 + rand(&[x, 3.0]) * 1.5));
        let mut j = 0.0;
        while j < hgt {
            p.px(x, -j, if j == hgt - 1.0 { WHITE } else { c[5] });
            j += 1.0;
        }
        p.dot(x, 0.0, c[6], 1.0 - d * 0.5);
        p.dot(x, 1.0, c[4], 0.8 - d * 0.6);
    }
    p.glow(0.0, -2.0, 40.0, c[4], 0.22);
    foot(p, c, t, o, false);
    streaks(p, c, t, o, false);
    orbiters(p, c, t, o, false);
}

fn draw(p: &mut Painter, t: f64, o: &State) {
    let (c, hh) = (&o.c, o.height);
    let ph = t / D;
    body(p, c, ph, hh);
    // cold light in its throat
    p.glow(axis(hh * 0.45, hh, ph), -hh * 0.45, 40.0, c[4], 0.3);
    p.glow(
        axis(hh * 0.15, hh, ph),
        -hh * 0.15,
        18.0,
        c[5],
        0.4 + 0.1 * libm::sin(TAU * ph * 4.0),
    );
    p.glow(axis(hh * 0.05, hh, ph), -3.0, 10.0, WHITE, 0.3);
    streaks(p, c, t, o, true);
    orbiters(p, c, t, o, true);
    foot(p, c, t, o, true);
    // snow thrown off the mouth, spreading into a thin cloud
    cycle(
        t,
        D,
        10,
        o.seed + 3.0,
        |g| g.range(0.9, 1.3),
        |age, life, r, _| {
            let th = r.range(0.0, TAU);
            let rr = radius(hh, hh) * (0.6 + age * 0.9);
            let k = age / life;
            let rad = 3.0 + 6.0 * k;
            let rc = js_min(rr, 36.0);
            let x = axis(hh, hh, ph) + libm::cos(th) * rc;
            let y = -hh - 2.0 - age * 4.0 + libm::sin(th) * rc * TILT;
            let a = 0.24 * (1.0 - k) * js_min(1.0, age / 0.15);
            p.disc(x, y, rad * 0.8, c[6], |d: f64, _: f64| a * (1.0 - d * d));
            p.glow(x, y, rad * 1.5, c[4], 0.08 * (1.0 - k));
        },
    );
}
