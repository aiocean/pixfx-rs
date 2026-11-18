//! `feedback/cloak-shimmer`: a sci-fi stealth cloak loop on the wrapped sprite, shown only by a broken refractive 1 px
//! outline whose bright runs crawl up the body, torn sideways by two scanline distortion bands travelling down per loop,
//! and faltering once per loop into a pale full outline with a faint body hatch.
//!
//! Adapted from Pixel VFX Studio's `scifi/cloak-shimmer` (MIT License, Copyright (c) 2026 Patch, Copyright (c) 2026
//! Hugo Duprez). Wraps its actor ([`Effect::wraps_actor`]): when handed a sprite via [`EffectState::set_actor`], it
//! reads [`Silhouette`] once per pose and shimmers/tears that sprite's mask; with no sprite it uses the stand-in
//! fluttering cloak silhouette.

use std::sync::Arc;

use pixfx_core::{Actor, Area, Effect, EffectState, Layer, Meta, NumOpt, Options, Silhouette};
use pixfx_draw::Painter;
use pixfx_draw::color::Rgba;
use pixfx_draw::math::{Rng, TAU, hash, js_max, js_round, to_int32};

pub struct CloakShimmer;

pub static CLOAK_SHIMMER: CloakShimmer = CloakShimmer;

const FPS: u32 = 15;
const N: i32 = 24;
const BEAT: i32 = 15;
const X0: i32 = -24;
const X1: i32 = 24;
const Y0: i32 = -66;
const Y1: i32 = 2;
const BOX: Area = Area {
    x: X0,
    y: Y0,
    w: (X1 - X0) as u32,
    h: (Y1 - Y0) as u32,
};

const TEAR: NumOpt = NumOpt::new("tear", 2.0).within(1.0, 3.0);
const BROKEN: NumOpt = NumOpt::new("broken", 0.35).within(0.15, 0.5);

const META: Meta = Meta {
    id: "feedback/cloak-shimmer",
    name: "Cloak shimmer",
    desc: "Seamless stealth cloak loop on the wrapped sprite: a broken refractive outline whose bright runs crawl up the body, torn sideways by two scanline distortion bands travelling down per loop, faltering once into a pale full outline with a faint body hatch.",
    tags: &[
        "feedback",
        "scifi",
        "cloak",
        "stealth",
        "shimmer",
        "character",
        "loop",
    ],
    size: (48, 72),
    origin: (22, 69),
    duration: 1.6,
    looping: true,
    fps: FPS,
    seed: 811_631_543,
    state: None,
    options: &[TEAR.spec(), BROKEN.spec()],
};

const PALETTE: [Rgba; 5] = [
    Rgba::hex24(0x0d2a36),
    Rgba::hex24(0x1f6f83),
    Rgba::hex24(0x3fc4d8),
    Rgba::hex24(0x9cf0f2),
    Rgba::hex24(0xe2fff8),
];

impl Effect for CloakShimmer {
    fn meta(&self) -> &Meta {
        &META
    }

    fn wraps_actor(&self) -> bool {
        true
    }

    fn spawn(&self, opts: &Options, seed: u64) -> Box<dyn EffectState> {
        let pal = opts.palette_or(&PALETTE);
        let c = |i: usize| pal.get(i).copied().unwrap_or(PALETTE[0]);
        Box::new(State {
            r: [c(0), c(1), c(2), c(3), c(4)],
            tear: opts.num(&TEAR),
            broken: opts.num(&BROKEN),
            seed: f64::from(to_int32(seed as f64)),
            sil: None,
        })
    }
}

struct State {
    r: [Rgba; 5],
    tear: f64,
    broken: f64,
    seed: f64,
    sil: Option<Silhouette>,
}

/// Stand-in hooded cloak silhouette (`Ruler::size(RULER.height, 1.0)` tall).
fn stand_inside(x: f64, y: f64, u: f64) -> bool {
    let hx = (x + 0.6 + (y + 52.0) * 0.12) / 5.4;
    let hy = (y + 52.0) / 6.6;
    if hx * hx + hy * hy <= 1.0 {
        return true;
    }
    if (-60.0..=-55.0).contains(&y) && x >= -4.0 - (y + 60.0) * 0.4 && x <= -1.5 + (y + 60.0) * 0.6
    {
        return true;
    }
    if (-4.0..-1.0).contains(&y) && ((-6.0..-2.0).contains(&x) || (2.0..6.5).contains(&x)) {
        return true;
    }
    if !(-47.0..=-3.0).contains(&y) {
        return false;
    }
    let f = (y + 47.0) / 44.0;
    let flut = libm::sin(x * 0.5 + u * TAU * 2.0) * 0.9 + libm::sin(x * 0.21 - u * TAU) * 0.5;
    let hem = -3.0 - js_max(0.0, flut) - if x > 0.0 { x * 0.12 } else { 0.0 };
    if y > hem {
        return false;
    }
    let wl = 8.5
        + f * 3.5
        + if f > 0.2 {
            libm::sin(f * 3.0 + u * TAU) * 0.4
        } else {
            0.0
        };
    let wr = 9.0 + f * 7.0 + libm::sin(f * 4.0 - u * TAU * 2.0) * f * 1.6;
    let sh = if f < 0.12 {
        let q = (0.12 - f) / 0.12;
        (1.0 - libm::sqrt(js_max(0.0, 1.0 - q * q))) * 3.5
    } else {
        0.0
    };
    x >= -wl + sh && x <= wr - sh
}

fn seam(x: i32, y: i32) -> bool {
    y > -40 && x == js_round(-1.0 + f64::from(y + 40) * 0.08) as i32
}

impl State {
    fn build_mask(&self, u: f64) -> Vec<bool> {
        let w = (X1 - X0) as usize;
        let h = (Y1 - Y0) as usize;
        let mut mask = vec![false; w * h];
        for y in Y0..Y1 {
            for x in X0..X1 {
                let cov = match &self.sil {
                    Some(s) => s.covers(x, y),
                    None => stand_inside(f64::from(x) + 0.5, f64::from(y) + 0.5, u),
                };
                mask[(y - Y0) as usize * w + (x - X0) as usize] = cov;
            }
        }
        mask
    }
}

impl EffectState for State {
    fn set_actor(&mut self, actor: Option<Arc<Actor>>) {
        self.sil = actor.and_then(|a| {
            let sil = Silhouette::of(&a, BOX);
            (!sil.is_empty()).then_some(sil)
        });
    }

    fn draw(&self, p: &mut Painter, t: f64, layer: Layer) {
        if layer != Layer::Front {
            return;
        }
        let k = (js_round(t * f64::from(FPS)) as i32).rem_euclid(N);
        let u = f64::from(k) / f64::from(N);
        let r = &self.r;
        let beat = k - BEAT;
        let mask = self.build_mask(u);
        let m_at = |x: i32, y: i32| -> bool {
            if !(X0..X1).contains(&x) || !(Y0..Y1).contains(&y) {
                false
            } else {
                mask[(y - Y0) as usize * (X1 - X0) as usize + (x - X0) as usize]
            }
        };

        let bands = [
            (-74.0 + ((u * 2.0) % 1.0) * 84.0, 1.0),
            (-74.0 + ((u * 2.0 + 0.5) % 1.0) * 84.0, -1.0),
        ];
        let band_at = |y: i32| -> Option<(f64, f64)> {
            let yf = f64::from(y);
            for &(by, s) in &bands {
                let d = yf - by;
                if (-3.0..=3.0).contains(&d) {
                    return Some((d, s));
                }
            }
            None
        };
        let shift_at = |y: i32| -> i32 {
            let Some((d, s)) = band_at(y) else {
                return 0;
            };
            let sl = (libm::floor((d + 3.0) / 2.0) as i32).clamp(0, 3) as usize;
            let mult = [0.5, -1.0, 1.0, -0.5][sl];
            js_round(s * self.tear * mult) as i32
        };

        let seed_mod = to_int32(self.seed) % 7;
        let mut glows: Vec<(f64, f64)> = Vec::new();

        for y in Y0..Y1 {
            let dx = shift_at(y);
            let b = band_at(y);
            let yf = f64::from(y);
            for x in (X0 - 4)..(X1 + 4) {
                let sx = x - dx;
                if !m_at(sx, y) {
                    continue;
                }
                let edge =
                    !m_at(sx - 1, y) || !m_at(sx + 1, y) || !m_at(sx, y - 1) || !m_at(sx, y + 1);
                let (px, py) = (f64::from(x) + 0.5, yf + 0.5);
                if !edge {
                    if let Some((bd, _)) = b
                        && bd == 0.0
                    {
                        let h_mod = (hash(&[self.seed, 31.0, yf]) % 3) as i32;
                        if ((sx + 40 + h_mod) & 3) < 3 {
                            p.px(px, py, r[2]);
                        }
                    } else if let Some((bd, _)) = b
                        && libm::fabs(bd) == 2.0
                    {
                        let g = (sx + 40).div_euclid(4);
                        let hv = hash(&[self.seed, 37.0, yf, f64::from(g)]);
                        let signed_shift = (hv as i32) >> 4;
                        if hv.is_multiple_of(3) && ((sx + 40) % 4) < 1 + (signed_shift % 3) {
                            p.px(px, py, r[1]);
                        }
                    } else if (0..=1).contains(&beat)
                        && (y - Y0 + beat) % 3 == 0
                        && !hash(&[
                            self.seed,
                            f64::from(41 + beat),
                            yf,
                            f64::from((sx + 40) >> 2),
                        ])
                        .is_multiple_of(if beat != 0 { 2 } else { 5 })
                    {
                        p.px(px, py, r[1]);
                    } else if m_at(sx + 1, y) && !m_at(sx + 2, y) && beat != 0 {
                        p.px(px, py, r[0]);
                    } else if seam(sx, y) && libm::sin(yf * 0.16 + u * TAU * 2.0) > -0.3 {
                        p.px(px, py, r[if beat == 0 { 3 } else { 1 }]);
                    }
                    continue;
                }

                let ph = (yf * 0.16 - f64::from(sx) * 0.05) + u * TAU * 2.0;
                let wave =
                    libm::sin(ph) * 0.6 + libm::sin(ph * 0.5 + 1.3 + f64::from(seed_mod)) * 0.4;
                let hole = f64::from(hash(&[self.seed, 7.0, f64::from(sx), yf]) % 1000) / 1000.0
                    < self.broken * (1.0 - wave) * 0.9;
                let c = if beat == 0 {
                    4
                } else if beat == 1 {
                    3
                } else if let Some((bd, _)) = b {
                    if libm::fabs(bd) <= 1.0 { 3 } else { 2 }
                } else if hole && beat != 2 {
                    continue;
                } else if wave > 0.35 {
                    3
                } else if wave > -0.25 {
                    2
                } else {
                    1
                };
                p.px(px, py, r[c]);
                if c >= 3 {
                    glows.push((px, py));
                }
            }

            if let Some((bd, _)) = b
                && libm::fabs(bd) == 2.0
            {
                let mut rg = Rng::new(hash(&[self.seed, 53.0, yf, libm::floor(u * 2.0 + 0.5)]));
                if rg.next() < 0.6 {
                    let mut ex = None;
                    for x in (X0 - 3..=X1 + 3).rev() {
                        if m_at(x - dx, y) {
                            ex = Some(x);
                            break;
                        }
                    }
                    if let Some(edge_x) = ex {
                        let len = 2 + libm::floor(rg.next() * 3.0) as i32;
                        let gap = 2 + libm::floor(rg.next() * 2.0) as i32;
                        for j in 0..len {
                            p.px(
                                f64::from(edge_x + gap + j) + 0.5,
                                yf + 0.5,
                                r[if j == 0 { 3 } else { 2 }],
                            );
                        }
                    }
                }
            }
        }

        for (gx, gy) in glows {
            p.glow(gx, gy, 2.5, r[2], if beat == 0 { 0.08 } else { 0.05 });
        }
        for &(by, _) in &bands {
            if by > -68.0 && by < 2.0 {
                p.glow_line(-12.0, by, 12.0, by, 4.0, r[2], 0.12);
            }
        }
    }
}
