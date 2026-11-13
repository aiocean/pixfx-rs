//! CRT off: port of pixelfx `src/fx/transitions/crt-off.js`.
//!
//! The glowing line (a white core, a pale fringe, an outer fringe while it's fresh, light along it) follows
//! Sprite Fusion's open-source lightning strike (https://www.spritefusion.com/pixel-art-vfx/lightning),
//! MIT License, Copyright (c) 2026 Hugo Duprez.

use pixfx_core::{Effect, EffectState, Layer, Meta, Options};
use pixfx_draw::Painter;
use pixfx_draw::color::{Rgba, WHITE, ramp};
use pixfx_draw::math::{clamp, ease, js_max, js_min, js_round, noise, rand};

use super::lib::{COLOR, H, W, clip_to, glint, rect, screen_meta};

pub struct CrtOff;

pub static CRT_OFF: CrtOff = CrtOff;

const META: Meta = screen_meta(
    "transitions/crt-off",
    "CRT off",
    "The picture squeezes into a white-hot glowing line, the line into a dot that lingers as a fading afterglow; then it switches back on, dot to line to picture, with a warm-up glow.",
    &[
        "transition",
        "screen",
        "crt",
        "tv",
        "retro",
        "switch",
        "wipe",
        "one-shot",
    ],
    2.0,
    1,
    &[COLOR.spec()],
);

// meta.options
const PALETTE: [Rgba; 8] = [
    Rgba::hex24(0x060a14),
    Rgba::hex24(0x0e1a33),
    Rgba::hex24(0x1a3360),
    Rgba::hex24(0x2a5a9a),
    Rgba::hex24(0x4a8fd0),
    Rgba::hex24(0x8cc4f0),
    Rgba::hex24(0xd4ecff),
    Rgba::hex24(0xffffff),
];

const WF: f64 = W as f64;
const HF: f64 = H as f64;
const YC: f64 = HF / 2.0;
const XC: f64 = WF / 2.0;
// off: squeeze to a line, the line to a dot, the dot fades; on: a dot, a line, the picture
const SQ0: f64 = 0.06;
const SQ1: f64 = 0.34;
const LN1: f64 = 0.56;
const DOT1: f64 = 0.86;
const ON0: f64 = 1.06;
const ON1: f64 = 1.16;
const OPEN1: f64 = 1.42;

/// The visible strip's height and the line's half-length at time `t`.
fn shape(t: f64) -> (f64, f64) {
    if t < SQ0 {
        (HF, XC)
    } else if t < SQ1 {
        (
            js_max(1.0, HF * (1.0 - ease::in_cubic((t - SQ0) / (SQ1 - SQ0)))),
            XC,
        )
    } else if t < LN1 {
        (1.0, XC * (1.0 - ease::in_quad((t - SQ1) / (LN1 - SQ1))))
    } else if t < ON0 {
        (0.0, 0.0)
    } else if t < ON1 {
        (1.0, XC * ease::out_expo((t - ON0) / (ON1 - ON0)))
    } else {
        (
            js_max(
                1.0,
                HF * ease::out_cubic(clamp((t - ON1) / (OPEN1 - ON1), 0.0, 1.0)),
            ),
            XC,
        )
    }
}

impl Effect for CrtOff {
    fn meta(&self) -> &Meta {
        &META
    }
    fn spawn(&self, opts: &Options, seed: u64) -> Box<dyn EffectState> {
        Box::new(State {
            c: ramp(&opts.palette_or(&PALETTE)),
            ink: opts.color(&COLOR),
            seed: seed as f64,
        })
    }
}

struct State {
    c: [Rgba; 8],
    ink: Rgba,
    seed: f64,
}

impl EffectState for State {
    fn update(&mut self, _dt: f64) {}

    fn draw(&self, p: &mut Painter, t: f64, layer: Layer) {
        if layer == Layer::Back {
            return;
        }
        let (c, ink, seed) = (&self.c, self.ink, self.seed);
        // pixelfx draws the glow, flecks, glint and dot through the screen-clipped painter and the rest
        // through the raw one. Every raw write lands inside the screen (rows within a few pixels of the
        // middle line, whole-screen rows, light on screen pixels), so clipping them all changes nothing.
        clip_to(p);
        let (h, w) = shape(t);
        let fl = 0.85 + 0.15 * rand(&[libm::floor(t * 30.0), 7.0, seed]);
        if (OPEN1 + 0.02..OPEN1 + 0.6).contains(&t) {
            // switched on: a warm-up glow over the picture, a bright refresh band rolling down it once, scanlines
            let u = (t - OPEN1) / 0.6;
            let band = -20.0 + u * (HF + 40.0);
            for y in 0..H {
                let yf = y as f64;
                let k = 0.22 * libm::pow(1.0 - u, 2.0)
                    + 0.3
                        * libm::pow(js_max(0.0, 1.0 - libm::fabs(yf - band) / 10.0), 2.0)
                        * (1.0 - u)
                    + if y & 1 == 1 { 0.05 * (1.0 - u) } else { 0.0 };
                if k > 0.01 {
                    for x in 0..W {
                        p.light(x as f64, yf, c[5], k);
                    }
                }
            }
            return;
        }
        if h >= HF {
            return;
        }
        let top = js_round(YC - h / 2.0);
        let bot = js_round(YC + h / 2.0);
        // the dark closing in from above and below
        if top > 0.0 {
            rect(p, 0.0, 0.0, WF, top, ink);
        }
        if bot < HF {
            rect(p, 0.0, bot, WF, HF - bot, ink);
        }
        if h > 2.0 {
            // the squeezed picture: brighter as it narrows, with scanlines, its edges glowing where the dark meets it
            let sq = 1.0 - h / HF;
            let mut y = top;
            while y < bot {
                let e = js_min(y - top, bot - 1.0 - y);
                let k = 0.55 * sq * sq
                    + if (y as i64) & 1 == 1 { 0.08 * sq } else { 0.0 }
                    + 0.35 * libm::pow(js_max(0.0, 1.0 - e / 6.0), 2.0);
                for x in 0..W {
                    p.light(x as f64, y, c[5], k * fl);
                }
                y += 1.0;
            }
            for y in [top - 1.0, bot] {
                if (0.0..HF).contains(&y) {
                    rect(p, 0.0, y, WF, 1.0, if sq > 0.6 { c[6] } else { c[5] });
                }
            }
            return;
        }
        // the line (and the dot it shrinks to): white-hot core, a pale fringe, an outer fringe while it's fresh, light
        let dot_phase = (LN1..ON0).contains(&t);
        let x0 = libm::floor(XC - w);
        let x1 = libm::ceil(XC + w);
        let fresh = t < SQ1 + 0.08 || (t > ON0 && t < ON1 + 0.04);
        if h > 0.0 && w >= 1.0 {
            let fr = libm::floor(t * 30.0);
            let mut x = js_max(0.0, x0);
            while x < js_min(WF, x1) {
                // it hums: the line wavers a pixel here and there, and crackles with flecks of static
                let u = libm::fabs(x + 0.5 - XC) / w; // 0 middle .. 1 ends
                let y = YC + js_round(noise(x / 23.0 + t * 9.0, seed) * (0.7 + 0.5 * u));
                p.px(x, y, if u < 0.85 { WHITE } else { c[6] });
                if u < 0.92 {
                    p.px(x, y - 1.0, c[6]);
                    p.px(x, y + 1.0, if u < 0.6 { c[6] } else { c[5] });
                }
                if fresh && u < 0.8 {
                    p.dot(x, y - 2.0, c[4], 0.5);
                    p.dot(x, y + 2.0, c[4], 0.5);
                }
                if rand(&[x, fr, seed + 3.0]) < 0.06 {
                    let side = if rand(&[x, fr, seed + 4.0]) < 0.5 {
                        -1.0
                    } else {
                        1.0
                    };
                    let off = 2.0 + libm::floor(rand(&[x, fr, seed + 5.0]) * 4.0);
                    p.px(
                        x,
                        y + side * off,
                        if rand(&[x, fr, 6.0]) < 0.5 {
                            WHITE
                        } else {
                            c[5]
                        },
                    );
                }
                p.light(x, y, c[6], 0.5 * fl * (1.0 - u * 0.6));
                x += 1.0;
            }
            let mut x = x0;
            while x <= x1 {
                p.glow(x, YC, 10.0, c[5], 0.35 * fl);
                x += 9.0;
            }
            p.glow(XC, YC, js_min(70.0, 16.0 + w * 0.4), c[5], 0.4 * fl);
        }
        // the dot: a glint that holds a moment, then an afterglow cooling to nothing
        if dot_phase {
            let u = (t - LN1) / (DOT1 - LN1);
            if u < 0.35 {
                glint(
                    p,
                    XC,
                    YC,
                    10.0 * (1.0 - u / 0.35) + 3.0,
                    0.2 + u * 2.0,
                    c[6],
                    c[5],
                    1.0,
                );
            }
            if u < 1.0 {
                p.px(
                    XC,
                    YC,
                    if u < 0.5 {
                        WHITE
                    } else if u < 0.75 {
                        c[6]
                    } else {
                        c[4]
                    },
                );
                p.px(XC - 1.0, YC, if u < 0.4 { c[6] } else { c[4] });
                // the phosphor smears the dot sideways as it cools
                let sm = js_round(9.0 * (1.0 - u));
                let mut k = 2.0;
                while k <= sm {
                    p.dot(XC - k, YC, c[4], 1.0 - k / (sm + 1.0));
                    p.dot(XC + k - 1.0, YC, c[4], 1.0 - k / (sm + 1.0));
                    k += 1.0;
                }
                p.glow(
                    XC,
                    YC,
                    22.0 * (1.0 - u * 0.5),
                    c[5],
                    0.55 * libm::pow(1.0 - u, 1.5),
                );
            }
        }
        // switching on: a pop of light at the heart before the line runs out
        let s = t - ON0;
        if (0.0..0.14).contains(&s) {
            glint(
                p,
                XC,
                YC,
                12.0 * (1.0 - s / 0.14) + 2.0,
                0.4 + s * 6.0,
                c[6],
                c[5],
                1.0,
            );
            p.glow(XC, YC, 40.0, WHITE, 0.5 * (1.0 - s / 0.14));
        }
    }
}
