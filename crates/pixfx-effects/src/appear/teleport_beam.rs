//! `appear/teleport-beam`: a sci-fi scan beam that prints the character it wraps from the head down, flashes white
//! with a horizontal flare, and settles as glitching scanline slices and rising square bits lift off.
//!
//! Adapted from Pixel VFX Studio's `scifi/teleport-beam` (MIT License, Copyright (c) 2026 Patch, Copyright (c) 2026
//! Hugo Duprez). In `pixfx-effects` it wraps its actor ([`Effect::wraps_actor`]): when handed a sprite via
//! [`EffectState::set_actor`], it reads [`Silhouette`] in `set_actor` and prints, flashes and tears the sprite's own
//! silhouette row by row; with no sprite it falls back to the stand-in figure footprint.

use std::sync::Arc;

use pixfx_core::cue::{Cue, FEEL, TimedCue};
use pixfx_core::{Actor, Area, Effect, EffectState, Layer, Meta, NumOpt, Options, Silhouette};
use pixfx_draw::Painter;
use pixfx_draw::color::Rgba;
use pixfx_draw::math::{TAU, clamp, hash, js_max, js_min, js_round, js_sign, rng, to_int32};

pub struct TeleportBeam;

pub static TELEPORT_BEAM: TeleportBeam = TeleportBeam;

const FPS: u32 = 16;
const END: i32 = 12;
const FLASH: i32 = 6;
/// Overhead emitter y in local pixels (above `Ruler::size(RULER.height, 0.9)`).
const TOP: i32 = -56;
/// Default crown row of the stand-in figure (`Ruler::size(RULER.height, 0.65)`).
const STAND_HEAD: i32 = -40;
const BOX: Area = Area {
    x: -22,
    y: -62,
    w: 44,
    h: 64,
};

const BITS: NumOpt = NumOpt::new("bits", 12.0).within(0.0, 24.0);
const TEAR: NumOpt = NumOpt::new("tear", 1.0).within(0.0, 2.0);
const FLARE: NumOpt = NumOpt::new("flare", 21.0).within(14.0, 23.0);

const META: Meta = Meta {
    id: "appear/teleport-beam",
    name: "Teleport beam",
    desc: "An emitter ring opens overhead and drops a scanline column onto bracketed floor marks; the column widens into a striped scan field that prints the wrapped sprite from the head down in horizontal bands, flashes white with a horizontal flare, then tears into sideways-glitching slices while square bits lift off and fade.",
    tags: &[
        "appear",
        "scifi",
        "teleport",
        "materialize",
        "shimmer",
        "character",
        "one-shot",
    ],
    size: (48, 72),
    origin: (24, 66),
    duration: 0.75,
    looping: false,
    fps: FPS,
    seed: 1_661_874_189,
    state: None,
    options: &[BITS.spec(), TEAR.spec(), FLARE.spec(), FEEL.spec()],
};

const PALETTE: [Rgba; 6] = [
    Rgba::hex24(0x0a1830),
    Rgba::hex24(0x0f4a66),
    Rgba::hex24(0x1a8fb4),
    Rgba::hex24(0x38d6f2),
    Rgba::hex24(0xa8f4ff),
    Rgba::hex24(0xffffff),
];

impl Effect for TeleportBeam {
    fn meta(&self) -> &Meta {
        &META
    }

    fn wraps_actor(&self) -> bool {
        true
    }

    fn cues(&self, opts: &Options) -> Vec<TimedCue> {
        let k = opts.num(&FEEL);
        if k.is_nan() || k <= 0.0 {
            return Vec::new();
        }
        let t = f64::from(FLASH) / f64::from(FPS);
        vec![TimedCue {
            t,
            cue: Cue::Flash {
                k: clamp(0.18 * k, 0.0, 1.0),
                dur: 0.08,
            },
            pan: None,
        }]
    }

    fn spawn(&self, opts: &Options, seed: u64) -> Box<dyn EffectState> {
        let pal = opts.palette_or(&PALETTE);
        let c = |i: usize| pal.get(i).copied().unwrap_or(PALETTE[0]);
        Box::new(State {
            r: [c(0), c(1), c(2), c(3), c(4), c(5)],
            bits: js_round(opts.num(&BITS)) as i32,
            tear: opts.num(&TEAR),
            flare: js_round(opts.num(&FLARE)),
            seed: to_int32(seed as f64) as f64,
            sprite: None,
        })
    }
}

struct SpriteShape {
    sil: Silhouette,
    head_y: i32,
    half_w: i32,
}

struct State {
    r: [Rgba; 6],
    bits: i32,
    tear: f64,
    flare: f64,
    seed: f64,
    sprite: Option<SpriteShape>,
}

/// Stand-in body span at row `y` (feet at 0).
fn stand_span(y: f64) -> Option<(f64, f64)> {
    if !(f64::from(STAND_HEAD)..=0.0).contains(&y) {
        return None;
    }
    let hw = if y < -33.0 {
        let u = (y + 36.5) / 4.0;
        3.6 * libm::sqrt(js_max(0.0, 1.0 - u * u))
    } else if y < -31.0 {
        2.0
    } else if y < -27.0 {
        4.0 + (y + 31.0) * 1.4
    } else if y < -14.0 {
        8.5 - (y + 27.0) * 0.14
    } else {
        6.2 + (y + 14.0) * 0.06
    };
    (hw >= 0.6).then_some((-hw, hw))
}

fn stand_in_body(x: f64, y: f64) -> bool {
    let Some((l, r)) = stand_span(y) else {
        return false;
    };
    if x < l || x > r {
        return false;
    }
    !(y > -10.0 && libm::fabs(x) < 1.2)
}

impl State {
    fn head_y(&self) -> i32 {
        self.sprite.as_ref().map_or(STAND_HEAD, |s| s.head_y)
    }

    fn half_w(&self) -> i32 {
        self.sprite.as_ref().map_or(10, |s| s.half_w)
    }

    fn body_span(&self, y: i32) -> Option<(f64, f64)> {
        if let Some(s) = &self.sprite {
            let mut x0 = i32::MAX;
            let mut x1 = i32::MIN;
            for x in -s.half_w - 2..=s.half_w + 2 {
                if s.sil.covers(x, y) {
                    x0 = x0.min(x);
                    x1 = x1.max(x);
                }
            }
            (x0 <= x1).then_some((f64::from(x0), f64::from(x1)))
        } else {
            stand_span(f64::from(y))
        }
    }

    fn in_body(&self, x: i32, y: i32) -> bool {
        if let Some(s) = &self.sprite {
            s.sil.covers(x, y)
        } else {
            stand_in_body(f64::from(x), f64::from(y))
        }
    }
}

fn floor_ring(p: &mut Painter, rx: f64, c0: Rgba, c1: Rgba, gap: f64) {
    let ry = js_max(2.0, js_round(rx * 0.28));
    for s in 0..72 {
        let a = (f64::from(s) / 72.0) * TAU;
        if gap > 0.0 && f64::from((s * 7) % 10) / 10.0 < gap {
            continue;
        }
        let near = libm::sin(a) > 0.0;
        let x = js_round(libm::cos(a) * rx);
        let y = js_round(libm::sin(a) * ry);
        p.px(x, y, if near { c0 } else { c1 });
        if near && libm::fabs(libm::cos(a)) < 0.6 {
            p.px(x, y + 1.0, c1);
        }
    }
}

impl EffectState for State {
    fn set_actor(&mut self, actor: Option<Arc<Actor>>) {
        self.sprite = actor.and_then(|a| {
            let sil = Silhouette::of(&a, BOX);
            let b = sil.bounds()?;
            let half_w = b.x.abs().max((b.x + b.w as i32).abs()).clamp(6, 20);
            Some(SpriteShape {
                sil,
                head_y: b.y.clamp(-60, -12),
                half_w,
            })
        });
    }

    fn draw(&self, p: &mut Painter, t: f64, layer: Layer) {
        if layer != Layer::Front {
            return;
        }
        let k = js_round(t * f64::from(FPS)) as i32;
        if !(0..END).contains(&k) {
            return;
        }
        let r = &self.r;
        let head = self.head_y();
        let body_hw = self.half_w();

        // Emitter ring overhead: opens, holds while the beam runs, closes into dashes.
        if k <= 7 {
            let rx = [5.0, 8.0, 9.0, 9.0, 9.0, 9.0, 8.0, 6.0][k as usize];
            let col = if k <= 5 {
                r[3]
            } else if k == 6 {
                r[4]
            } else {
                r[2]
            };
            for s in 0..48 {
                let a = (f64::from(s) / 48.0) * TAU;
                if k == 7 && s % 4 < 2 {
                    continue;
                }
                let near = libm::sin(a) > 0.0;
                p.px(
                    js_round(libm::cos(a) * rx),
                    f64::from(TOP) + js_round(libm::sin(a) * 2.0),
                    if near { col } else { r[2] },
                );
            }
            if (1..=6).contains(&k) {
                p.line(-2.0, f64::from(TOP + 2), 1.0, f64::from(TOP + 2), r[4], 1.0);
            }
        }

        // Floor brackets: four corners that close in and lock onto the target spot.
        if k <= 8 {
            let bx = [14.0, 12.0, 10.0, 10.0, 10.0, 10.0, 10.0, 11.0, 12.0][k as usize];
            let by = [5.0, 4.0, 3.0, 3.0, 3.0, 3.0, 3.0, 3.0, 4.0][k as usize];
            let col = if k <= 1 {
                r[2]
            } else if k <= 5 {
                r[3]
            } else if k == 6 {
                r[4]
            } else {
                r[2]
            };
            for sx in [-1.0, 1.0] {
                for sy in [-1.0, 1.0] {
                    if k == 8 && sx * sy > 0.0 {
                        continue;
                    }
                    let (x, y) = (sx * bx, sy * by);
                    p.line(x, y, x - sx * 3.0, y, col, 1.0);
                    p.line(x, y, x, y - sy * 2.0, col, 1.0);
                }
            }
        }

        // Scanline column: drops on frames 0..=2, widens into the scan field on 3..=5.
        let col_bots = [-44, -22, 0, 0, 0, 0];
        if k <= 2 {
            let col_bot = col_bots[k as usize];
            let w = k >= 1;
            for y in (TOP + 3)..=col_bot {
                let yf = f64::from(y);
                p.px(0.0, yf, if y > col_bot - 4 { r[4] } else { r[3] });
                if w {
                    p.px(-1.0, yf, r[2]);
                    p.px(1.0, yf, r[2]);
                }
            }
            if k == 2 {
                floor_ring(p, 7.0, r[4], r[3], 0.0);
            }
        } else if k <= 5 {
            let step = (k - 3) as usize;
            let hw = [body_hw.min(6), body_hw, body_hw][step];
            let prints = [head + (-head) / 4, head + (-head * 3) / 5, 0];
            let print = if head == STAND_HEAD {
                [-31, -16, 0][step]
            } else {
                prints[step]
            };
            for y in (TOP + 3)..=0 {
                let yf = f64::from(y);
                if (((y - 3 * k) % 6) + 6) % 6 != 0 {
                    p.px(f64::from(-hw), yf, r[3]);
                    p.px(f64::from(hw), yf, r[3]);
                }
                if (y - k).rem_euclid(4) == 0 {
                    for x in (-hw + 2)..=(hw - 2) {
                        if !(y >= head && y <= print + 1 && self.in_body(x, y)) {
                            p.px(f64::from(x), yf, r[2]);
                        }
                    }
                }
            }
            for y in head..=print {
                let yf = f64::from(y);
                for x in -body_hw..=body_hw {
                    if !self.in_body(x, y) {
                        continue;
                    }
                    let band = ((y - head) & 3) == 3;
                    let edge = !self.in_body(x - 1, y) || !self.in_body(x + 1, y);
                    let c = if band {
                        r[2]
                    } else if edge {
                        r[3]
                    } else if (y & 1) == 0 {
                        r[4]
                    } else {
                        r[3]
                    };
                    p.px(f64::from(x), yf, c);
                }
            }
            let pf = f64::from(print);
            for x in (-hw + 1)..=(hw - 1) {
                let xf = f64::from(x);
                p.px(xf, pf, r[4]);
                if print > head {
                    p.px(
                        xf,
                        f64::from(print - 1),
                        if self.in_body(x, print - 1) {
                            r[3]
                        } else {
                            r[2]
                        },
                    );
                }
            }
            p.px(f64::from(-hw - 1), pf, r[4]);
            p.px(f64::from(hw + 1), pf, r[4]);
            floor_ring(
                p,
                f64::from(9 + (k - 3)),
                if k == 5 { r[4] } else { r[3] },
                r[2],
                0.0,
            );
        } else if k == FLASH {
            for y in (head - 1)..=0 {
                let yf = f64::from(y);
                for x in (-body_hw - 1)..=(body_hw + 1) {
                    let xf = f64::from(x);
                    if self.in_body(x, y) {
                        p.px(xf, yf, r[5]);
                    } else if self.in_body(x - 1, y)
                        || self.in_body(x + 1, y)
                        || self.in_body(x, y - 1)
                        || self.in_body(x, y + 1)
                    {
                        p.px(xf, yf, r[4]);
                    }
                }
            }
            let fw = self.flare;
            let fi = js_round(fw * 5.0 / 7.0);
            p.line(-fw, -24.0, fw, -24.0, r[4], 1.0);
            p.line(-fi, -24.0, fi, -24.0, r[5], 1.0);
            p.line(-9.0, -25.0, 9.0, -25.0, r[5], 1.0);
            p.line(-9.0, -23.0, 9.0, -23.0, r[5], 1.0);
            floor_ring(p, 13.0, r[4], r[3], 0.0);
        } else {
            let a = (k - FLASH) as usize;
            let keeps = [0.0, 0.9, 0.75, 0.5, 0.28, 0.1];
            let shifts = [0.0, 1.0, 2.0, 3.0, 4.0, 5.0];
            for y in head..=0 {
                let sl = (y - head).div_euclid(2);
                let mut rg = rng(&[self.seed, 51.0, f64::from(sl), f64::from(a as i32)]);
                let keep = keeps.get(a).copied().unwrap_or(0.0);
                if rg.next() > keep {
                    continue;
                }
                if a >= 2 && (y & 1) != 0 {
                    continue;
                }
                let dir = if rg.next() < 0.5 { -1.0 } else { 1.0 };
                let move_k = if rg.next() < 0.35 { 0.0 } else { 1.0 };
                let shift =
                    js_round(shifts.get(a).copied().unwrap_or(0.0) * self.tear) * dir * move_k;
                let Some((sp0, sp1)) = self.body_span(y) else {
                    continue;
                };
                let cut = clamp(1.0 - (f64::from(a as i32) - 1.0) * 0.22, 0.2, 1.0);
                let len = (sp1 - sp0) * cut;
                let x0 = sp0 + rg.next() * (sp1 - sp0 - len);
                let col = if a == 1 {
                    if (y & 1) != 0 { r[4] } else { r[3] }
                } else if a == 2 {
                    r[3]
                } else if a <= 4 {
                    r[2]
                } else {
                    r[1]
                };
                let start_x = js_round(x0) as i32;
                let end_x = js_round(x0 + len) as i32;
                let yf = f64::from(y);
                for x in start_x..=end_x {
                    if a <= 2 && !self.in_body(x, y) {
                        continue;
                    }
                    p.px(f64::from(x) + shift, yf, col);
                }
                if shift != 0.0 && a <= 3 {
                    let lead = js_round(if shift > 0.0 { x0 + len } else { x0 }) + shift;
                    p.px(lead, yf, r[4]);
                }
            }
            if a <= 2 {
                floor_ring(
                    p,
                    f64::from(13 + (a as i32) * 2),
                    if a == 1 { r[3] } else { r[2] },
                    r[1],
                    if a == 2 { 0.5 } else { 0.2 },
                );
            }
        }

        // Square bits lifting off the body after the flash.
        if k > FLASH {
            let a = f64::from(k - FLASH);
            for i in 0..self.bits {
                let mut rg = rng(&[self.seed, 71.0, f64::from(i)]);
                let sx = js_round(rg.range(-8.0, 8.0));
                let sy = js_round(rg.range(f64::from(head + 2), -4.0));
                let born = 1.0 + libm::floor(rg.next() * 2.2);
                let life = rg.range(3.2, 5.4);
                let age = a - born;
                if age < 0.0 || age > life {
                    continue;
                }
                let vy = rg.range(2.0, 3.4);
                let sign_sx = if sx == 0.0 { 1.0 } else { js_sign(sx) };
                let drift = sign_sx * rg.range(0.4, 1.5);
                let x = sx + js_round(drift * age);
                let y = js_round(sy - vy * age - 0.15 * age * age);
                let u = age / life;
                let big = (i & 1) == 0;
                if u < 0.3 {
                    let s = if big { 3.0 } else { 2.0 };
                    p.rect(
                        x - 1.0,
                        y - 1.0,
                        s,
                        s,
                        if u < 0.15 { r[4] } else { r[3] },
                        1.0,
                    );
                } else if u < 0.65 && big {
                    for (dx, dy) in [
                        (-1.0, -1.0),
                        (0.0, -1.0),
                        (1.0, -1.0),
                        (-1.0, 0.0),
                        (1.0, 0.0),
                        (-1.0, 1.0),
                        (0.0, 1.0),
                        (1.0, 1.0),
                    ] {
                        p.px(x + dx, y + dy, r[3]);
                    }
                } else if u < 0.65 {
                    p.rect(x - 1.0, y - 1.0, 2.0, 2.0, r[2], 1.0);
                } else {
                    p.px(x, y, if u < 0.85 { r[2] } else { r[1] });
                }
            }
        }

        // Additive light layer
        if k <= 2 {
            let col_bot = f64::from(col_bots[k as usize]);
            p.glow_line(0.0, f64::from(TOP), 0.0, col_bot, 4.0, r[3], 0.35);
        } else if k <= 5 {
            let py = f64::from([-31, -16, 0][(k - 3) as usize]);
            p.glow_line(0.0, f64::from(TOP), 0.0, 0.0, 12.0, r[2], 0.3);
            p.glow_line(-10.0, py, 10.0, py, 4.0, r[4], 0.35);
        } else if k == FLASH {
            p.glow(0.0, -22.0, 28.0, r[4], 0.7);
            p.glow_line(
                -self.flare - 1.0,
                -24.0,
                self.flare + 1.0,
                -24.0,
                4.0,
                r[4],
                0.5,
            );
        } else if k <= FLASH + 2 {
            let da = f64::from(k - FLASH);
            p.glow(0.0, -20.0, 22.0 - da * 6.0, r[3], 0.3 / da);
        }
        if k <= 7 {
            p.glow(0.0, f64::from(TOP), 8.0, r[3], 0.3);
        }
        if (2..=8).contains(&k) {
            p.glow(0.0, 0.0, 12.0, r[3], if k == FLASH { 0.5 } else { 0.22 });
        }
        let _ = (hash, js_min);
    }
}
