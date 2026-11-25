//! Drip: port of pixelfx `src/fx/water/drip.js`.
//!
//! The bead swelling at the tip and the falling drop's tail follow Sprite Fusion's open-source blood
//! (https://www.spritefusion.com/pixel-art-vfx/blood), MIT License, Copyright (c) 2026 Hugo Duprez.
//!
//! `source` (pixelfx `true`) is a number here: 0 hides the dripstone, anything else draws it.

use core::f64::consts::PI;

use pixfx_core::{Effect, EffectState, Layer, Meta, NumOpt, Options};
use pixfx_draw::Painter;
use pixfx_draw::color::{Rgba, WHITE, ramp};
use pixfx_draw::math::{clamp, ease, js_max, js_min, smooth};

use super::lib::{ROCK, Ramp, RingOpts, WATER, bead, ring};

pub struct Drip;

pub static DRIP: Drip = Drip;

const HEIGHT: NumOpt = NumOpt::new("height", 62.0).within(4.0, 400.0);
const SOURCE: NumOpt = NumOpt::new("source", 1.0);
const META: Meta = Meta {
    id: "water/drip",
    name: "Drip",
    desc: "A bead swells under a wet stone tip and catches the light, pinches off, falls stretched by its speed and plinks into a puddle with a crown, a ring and a tiny rebound.",
    tags: &["water", "drip", "drop", "puddle", "cave", "ambient", "loop"],
    size: (72, 88),
    origin: (36, 78),
    duration: 1.6,
    looping: true,
    fps: 30,
    seed: 1,
    state: None,
    options: &[HEIGHT.spec(), SOURCE.spec()],
};

const SY: f64 = 0.28;
const G: f64 = 760.0;
const LET: f64 = 0.78;
const PRX: f64 = 20.0;
const REBOUND: f64 = 0.06;
const BACK: f64 = 0.2;
const C_WET: [Rgba; 2] = [Rgba::hex24(0x5d6a86), Rgba::hex24(0x9fb6d6)];

impl Effect for Drip {
    fn meta(&self) -> &Meta {
        &META
    }
    fn spawn(&self, opts: &Options, _seed: u64) -> Box<dyn EffectState> {
        let source = opts.num(&SOURCE);
        Box::new(State {
            c: ramp(&opts.palette_or(&WATER)),
            height: opts.num(&HEIGHT),
            source: source != 0.0 && !source.is_nan(),
        })
    }
}

struct State {
    c: Ramp,
    height: f64,
    source: bool,
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

// the stone the water gathers on: a lump of ceiling rock tapering to a dripstone point, lit on its left, a dark
// rim, a wet streak running down to the point
fn dripstone(p: &mut Painter, h: f64) {
    let st = &ROCK;
    let top = -h - 14.0;
    p.shade(-24.0, top, 22.0, -h + 0.5, |x, y, out| {
        // a ragged underside of ceiling rock that hangs down into a short dripstone, its point at x = 0
        let under = top + 7.0 + libm::sin(x * 0.45) * 1.2 + libm::sin(x * 1.3 + 1.0) * 0.6
            - libm::fabs(x + 4.0) * 0.12;
        let cone = -h - libm::fabs(x) * 1.25;
        let bottom = js_max(under, js_min(cone, -h));
        let span = if x < 0.0 { 24.0 } else { 22.0 };
        if y > bottom || libm::fabs(x) > span - js_max(0.0, top + 3.0 - y) * 4.0 {
            return None;
        }
        let rim = y > bottom - 1.0 || libm::fabs(x) > span - 1.0;
        let (lx, v) = ((x + 24.0) / 46.0, (y - top) / 14.0);
        out.c = if rim {
            st[0]
        } else if v < 0.25 {
            st[1]
        } else if y > under && libm::fabs(x) < 6.0 {
            if x < 0.0 { st[4] } else { st[2] }
        } else if lx < 0.3 {
            st[4]
        } else if lx < 0.62 {
            st[3]
        } else {
            st[2]
        };
        if !rim && libm::fabs(x - 9.0 - (y - top) * 0.4) < 0.5 && v > 0.3 && v < 0.7 {
            out.c = st[0];
        }
        None
    });
    // a wet streak down to the point, catching the light
    let mut y = -h - 7.0;
    while y < -h {
        p.px(-1.0, y, if y > -h - 3.0 { C_WET[1] } else { C_WET[0] });
        y += 1.0;
    }
    p.px(-1.0, -h - 2.0, WHITE);
}

// the puddle: dark see-through water with a lit far edge, the drop's light in it
fn puddle(p: &mut Painter, c: &Ramp) {
    let ry = PRX * SY;
    let mut yy = libm::floor(-ry);
    while yy < ry {
        let mut xx = -PRX;
        while xx < PRX {
            let (x, y) = (xx + 0.5, yy + 0.5);
            xx += 1.0;
            let d = libm::pow(x / PRX, 2.0) + libm::pow(y / ry, 2.0);
            if d > 1.0 {
                continue;
            }
            p.blend(x, y, c[1], 0.72 * js_min(1.0, (1.0 - d) * 5.0));
        }
        yy += 1.0;
    }
    // the far edge catches the light in one fine line
    let mut xx = -PRX + 3.0;
    while xx < PRX - 3.0 {
        let x = xx + 0.5;
        let y = -libm::sqrt(1.0 - libm::pow(x / PRX, 2.0)) * ry;
        p.dot(
            x,
            y + 0.5,
            if libm::fabs(x) < PRX * 0.5 {
                c[4]
            } else {
                c[3]
            },
            0.95,
        );
        xx += 1.0;
    }
}

fn land_at(h: f64) -> f64 {
    LET + libm::sqrt((2.0 * h) / G)
}

fn draw_back(p: &mut Painter, t: f64, o: &State) {
    let c = &o.c;
    puddle(p, c);
    let u = t - land_at(o.height);
    if u >= 0.0 {
        ring(
            p,
            0.0,
            0.0,
            2.0 + 17.0 * ease::out_cubic(u / 0.75),
            SY,
            1.1 * (1.0 - u / 0.75),
            c,
            RingOpts {
                n: 2.0,
                gap: 2.5,
                half: -1.0,
                seed: 1.0,
                wob: 0.4,
                ..RingOpts::default()
            },
        );
    }
    let v = t - land_at(o.height) - REBOUND - BACK;
    if v >= 0.0 {
        ring(
            p,
            0.0,
            0.0,
            1.5 + 9.0 * ease::out_cubic(v / 0.5),
            SY,
            0.9 * (1.0 - v / 0.5),
            c,
            RingOpts {
                n: 1.0,
                half: -1.0,
                seed: 2.0,
                ..RingOpts::default()
            },
        );
    }
}

fn draw(p: &mut Painter, t: f64, o: &State) {
    let c = &o.c;
    let h = o.height;
    let land = land_at(h);
    if o.source {
        dripstone(p, h);
    }
    // the bead swells at the tip, wobbling, and catches the light; its neck stretches before it lets go
    if t < LET {
        let k = t / LET;
        let r = 0.7 + 2.3 * ease::in_quad(k);
        let wob = 0.15 * libm::sin(t * 26.0) * k;
        let neck = smooth(0.82, 1.0, k) * 2.5;
        let cy = -h + r + neck;
        if neck > 0.2 {
            p.line(0.0, -h, 0.0, cy - r, c[5], 1.0);
        }
        let (c3, c4, c5, c6) = (c[3], c[4], c[5], c[6]);
        p.ellipse(
            0.0,
            cy,
            r * (1.0 - wob),
            r * (1.0 + wob) + 0.3 * k,
            move |d: f64, a: f64| {
                if d > 0.72 {
                    if libm::cos(a) < 0.0 && libm::sin(a) < 0.3 {
                        c5
                    } else {
                        c3
                    }
                } else if d < 0.35 && libm::cos(a) > 0.0 {
                    c6
                } else {
                    c4
                }
            },
            1.0,
        );
        if r > 1.0 {
            p.px(-r * 0.4, cy - r * 0.45, WHITE);
        }
        p.glow(0.0, cy, 4.0 + r * 2.0, c[5], 0.14 + 0.14 * k);
    } else {
        // a little of it springs back up the tip after it lets go
        let s = (t - LET) / 0.15;
        if s < 1.0 {
            p.ellipse(
                0.0,
                -h + 0.8,
                1.2 * (1.0 - s * 0.6),
                1.0 + libm::sin(s * PI) * 0.8,
                c[4],
                1.0,
            );
        }
    }
    // the fall, stretched by its speed
    let f = t - LET;
    if f > 0.0 && t < land {
        let y = -h + 3.0 + 0.5 * G * f * f;
        let vy = G * f;
        bead(p, 0.0, y, 0.0, vy, 3.0, c, 1.0);
        p.glow(0.0, y, 6.0, c[5], 0.2);
    }
    // the plink: a white fleck, a little crown thrown up, the near half of the ring, light
    let u = t - land;
    if u >= 0.0 {
        ring(
            p,
            0.0,
            0.0,
            2.0 + 17.0 * ease::out_cubic(u / 0.75),
            SY,
            1.1 * (1.0 - u / 0.75),
            c,
            RingOpts {
                n: 2.0,
                gap: 2.5,
                half: 1.0,
                hot: u < 0.08,
                seed: 1.0,
                wob: 0.4,
                glint: if u < 0.5 { 1.0 - u * 2.0 } else { 0.0 },
            },
        );
        if u < 0.06 {
            p.px(0.0, 0.0, WHITE);
            p.px(-1.0, 0.0, c[6]);
            p.px(1.0, 0.0, c[6]);
            p.glow(0.0, -1.0, 10.0, c[5], 0.5 * (1.0 - u / 0.06));
        }
        // the crown: three flecks thrown out and back
        for (vx, vh) in [(-26.0, 70.0), (24.0, 64.0), (-6.0, 90.0)] {
            let hh = vh * u - 0.5 * G * u * u;
            if hh > 0.0 && u < 0.3 {
                p.px(vx * u, -hh, if u < 0.1 { WHITE } else { c[6] });
            }
        }
        // the rebound: a bead pops up out of the middle and drops back
        let r = u - REBOUND;
        let hh = 52.0 * r - 0.5 * G * 0.5 * r * r * 2.0;
        if r > 0.0 && hh > 0.0 {
            p.line(0.0, -1.0, 0.0, -hh + 1.0, c[5], 1.0);
            p.circle(0.0, -hh, 1.1, c[6], 1.0);
            p.px(0.0, -hh - 1.0, WHITE);
        }
        let v = u - REBOUND - BACK;
        if v >= 0.0 {
            ring(
                p,
                0.0,
                0.0,
                1.5 + 9.0 * ease::out_cubic(v / 0.5),
                SY,
                0.9 * (1.0 - v / 0.5),
                c,
                RingOpts {
                    n: 1.0,
                    half: 1.0,
                    seed: 2.0,
                    hot: v < 0.06,
                    ..RingOpts::default()
                },
            );
        }
        p.glow(
            0.0,
            0.0,
            18.0,
            c[4],
            0.22 * (1.0 - clamp(u / 0.6, 0.0, 1.0)),
        );
    }
    // the tip and the drop shimmer in the puddle
    let sh = if libm::sin(t * 20.0) > 0.0 { 0.0 } else { 1.0 };
    p.px(-1.0 + sh, 2.0, c[4]);
    if f > 0.0 && t < land {
        let ry = 2.0 + 0.5 * G * f * f * 0.04;
        p.px(sh, js_min(4.0, ry), c[5]);
    }
    // the wet light under the tip and in the puddle
    p.glow(0.0, -h + 2.0, 8.0, c[5], 0.16);
    p.glow(0.0, 0.0, 26.0, c[3], 0.14);
}
