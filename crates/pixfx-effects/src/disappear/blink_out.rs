//! `disappear/blink-out`: the wrapped sprite's silhouette folds inward into vertical streaks that stretch upward and
//! pinch into a bright lens and line, flash white with a cross, and snap upward off the top, leaving a collapsing
//! implosion ring, drifting rune motes and a fading floor glyph.
//!
//! Adapted from Pixel VFX Studio's `arcane/blink-out` (MIT License, Copyright (c) 2026 Patch, Copyright (c) 2026 Hugo
//! Duprez). Wraps its actor ([`Effect::wraps_actor`]): when handed a sprite via [`EffectState::set_actor`], it reads
//! [`Silhouette`] once per pose so the folding vertical streaks compress the sprite's actual row spans and column tops.

use std::sync::Arc;

use pixfx_core::cue::{Cue, FEEL, TimedCue};
use pixfx_core::{Actor, Area, Effect, EffectState, Layer, Meta, NumOpt, Options, Silhouette};
use pixfx_draw::Painter;
use pixfx_draw::color::Rgba;
use pixfx_draw::math::{Rng, TAU, clamp, hash, js_max, js_round, rand, to_int32};
use pixfx_draw::shapes::{flash_star, wedge};

pub struct BlinkOut;

pub static BLINK_OUT: BlinkOut = BlinkOut;

const FPS: u32 = 15;
const END: i32 = 18;
const PEAK: i32 = 4;
/// Fold centre height at the chest (`-f64::from(Ruler::size(RULER.height, 0.4))`).
const CY: f64 = -24.0;
const BOX: Area = Area {
    x: -20,
    y: -56,
    w: 40,
    h: 58,
};

const MOTES: NumOpt = NumOpt::new("motes", 30.0).within(8.0, 48.0);
const SPECKS: NumOpt = NumOpt::new("specks", 12.0).within(0.0, 20.0);
const GLYPH: NumOpt = NumOpt::new("glyph", 1.0).within(0.6, 1.4);

const META: Meta = Meta {
    id: "disappear/blink-out",
    name: "Blink out",
    desc: "The wrapped sprite folds inward into vertical streaks that pinch to a bright line, flash white once with a cross, and snap upward and away, leaving a collapsing ring, drifting rune motes and a fading floor glyph.",
    tags: &[
        "disappear",
        "arcane",
        "blink",
        "teleport",
        "vanish",
        "character",
        "one-shot",
    ],
    size: (64, 80),
    origin: (32, 72),
    duration: 1.2,
    looping: false,
    fps: FPS,
    seed: 412_001_821,
    state: None,
    options: &[MOTES.spec(), SPECKS.spec(), GLYPH.spec(), FEEL.spec()],
};

const PALETTE: [Rgba; 6] = [
    Rgba::hex24(0x1f1342),
    Rgba::hex24(0x43277f),
    Rgba::hex24(0x7a45cc),
    Rgba::hex24(0xb98cff),
    Rgba::hex24(0xeedcff),
    Rgba::hex24(0xffffff),
];

const RUNES: [[&str; 3]; 5] = [
    ["x.x", ".x.", ".x."],
    ["xx.", "x.x", "x.."],
    [".x.", "xxx", "x.."],
    ["x..", "xx.", "x.x"],
    [".xx", "x..", ".xx"],
];

impl Effect for BlinkOut {
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
        let t = f64::from(PEAK) / f64::from(FPS);
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
            c: [c(0), c(1), c(2), c(3), c(4), c(5)],
            motes: js_round(opts.num(&MOTES)) as i32,
            specks: js_round(opts.num(&SPECKS)) as i32,
            glyph: opts.num(&GLYPH),
            seed: f64::from(to_int32(seed as f64)),
            sil: None,
        })
    }
}

struct State {
    c: [Rgba; 6],
    motes: i32,
    specks: i32,
    glyph: f64,
    seed: f64,
    sil: Option<Silhouette>,
}

fn stand_span(y: f64) -> Option<(f64, f64)> {
    if !(-42.0..=0.0).contains(&y) {
        return None;
    }
    let lean = -y * 0.04;
    if y < -34.0 {
        let v = (y + 38.2) / 4.3;
        let hw = 3.4 * libm::sqrt(clamp(1.0 - v * v, 0.0, 1.0));
        return (hw >= 0.6).then_some((lean + 0.8 - hw, lean + 0.8 + hw));
    }
    if y < -32.0 {
        return Some((lean - 1.6, lean + 2.4));
    }
    let u = (y + 32.0) / 32.0;
    let mut l = -7.0 - u * 2.2;
    let mut r = 6.4 + u * 2.6;
    if y > -24.0 && y < -15.0 {
        l += 1.6 * libm::sin(((y + 24.0) / 9.0) * core::f64::consts::PI);
    }
    if y > -6.0 {
        r += (y + 6.0) * 0.25;
    }
    Some((lean + l, lean + r))
}

fn draw_rune(p: &mut Painter, x: f64, y: f64, idx: usize, c: Rgba) {
    let g = RUNES[idx % RUNES.len()];
    for (r, row) in g.iter().enumerate() {
        for (q, ch) in row.bytes().enumerate() {
            if ch == b'x' {
                p.px(x + f64::from(q as i32 - 1), y + f64::from(r as i32 - 1), c);
            }
        }
    }
}

#[expect(clippy::too_many_arguments, reason = "lumpRing parameters")]
fn lump_ring(
    p: &mut Painter,
    cx: f64,
    cy: f64,
    r: f64,
    w: f64,
    cols: [Rgba; 2],
    seed: f64,
    broken: f64,
    tear: f64,
) {
    let big_r = r + w + 1.0;
    let ph = rand(&[seed, 1.0]) * TAU;
    let ph2 = rand(&[seed, 2.0]) * TAU;
    p.each(
        cx - big_r,
        cy - big_r,
        cx + big_r,
        cy + big_r,
        |painter, x, y, sx, sy| {
            let (dx, dy) = (x - cx, (y - cy) * 1.08);
            let d = libm::hypot(dx, dy);
            let a = libm::atan2(dy, dx);
            let lump = js_max(
                0.0,
                0.6 * libm::sin(a * 2.0 + ph) + 0.5 * libm::sin(a * 3.0 + ph2) + 0.15,
            );
            let ww = 1.3 + w * lump;
            if tear > 0.0 && libm::fabs(a + core::f64::consts::FRAC_PI_2) < tear {
                return;
            }
            let inner = r - ww * 0.35;
            let outer = r + ww * 0.65;
            if d < inner || d > outer {
                return;
            }
            if broken > 0.0 {
                let seg = libm::floor(((a + core::f64::consts::PI) / TAU) * js_max(6.0, r * 1.6));
                if rand(&[seed, seg, 9.0]) < broken {
                    return;
                }
            }
            let u = (d - inner) / (outer - inner + 1e-6);
            painter.put(sx, sy, if u < 0.4 { cols[0] } else { cols[1] }, 1.0);
        },
    );
}

fn draw_glyph(
    p: &mut Painter,
    lvl: i32,
    gone: f64,
    ramp: &[Rgba; 5],
    seed: f64,
    draw_frac: f64,
    gs: f64,
) {
    if lvl <= 0 {
        return;
    }
    let (rx, ry) = (15.0 * gs, 4.0 * gs);
    let steps = 120;
    for i in 0..steps {
        let frac = f64::from(i) / f64::from(steps);
        if frac > draw_frac {
            break;
        }
        let seg = f64::from(i / 5);
        if rand(&[seed, seg, 31.0]) < gone {
            continue;
        }
        let a = frac * TAU;
        let (x, y) = (libm::cos(a) * rx, libm::sin(a) * ry);
        let near = libm::sin(a) > 0.0;
        let ci = (lvl - if near { 0 } else { 1 }).clamp(0, 4) as usize;
        p.px(js_round(x), js_round(y), ramp[ci]);
        if near && lvl >= 3 && libm::fabs(libm::cos(a)) < 0.8 {
            p.px(
                js_round(x),
                js_round(y) + 1.0,
                ramp[(lvl - 1).clamp(0, 4) as usize],
            );
        }
    }
    if lvl >= 2 {
        for i in 0..70 {
            let a = (f64::from(i) / 70.0) * TAU;
            let sg = i / 9;
            if sg % 2 == 1 || rand(&[seed, f64::from(sg), 33.0]) < gone * 1.2 {
                continue;
            }
            let near = libm::sin(a) > 0.0;
            let ci = (lvl - if near { 1 } else { 2 }).clamp(0, 4) as usize;
            p.px(
                js_round(libm::cos(a) * (9.0 * gs)),
                js_round(libm::sin(a) * (2.2 * gs)),
                ramp[ci],
            );
        }
        for i in 0..4 {
            let fi = f64::from(i);
            if rand(&[seed, fi, 35.0]) < gone * 1.3 {
                continue;
            }
            let a = ((fi + 0.5 + (rand(&[seed, fi, 34.0]) - 0.5) * 0.3) / 4.0) * TAU;
            let x = js_round(libm::cos(a) * (12.0 * gs));
            let y = js_round(libm::sin(a) * (3.1 * gs));
            let near = libm::sin(a) > 0.0;
            let ci = (lvl - if near { 0 } else { 2 }).clamp(0, 4) as usize;
            let c = ramp[ci];
            p.px(x, y, c);
            p.px(x, y - 1.0, c);
        }
    }
}

impl State {
    fn span(&self, y: f64) -> Option<(f64, f64)> {
        if let Some(sil) = &self.sil {
            let yi = libm::floor(y) as i32;
            let mut x0 = i32::MAX;
            let mut x1 = i32::MIN;
            for x in -18..=18 {
                if sil.covers(x, yi) {
                    x0 = x0.min(x);
                    x1 = x1.max(x);
                }
            }
            (x0 <= x1).then_some((f64::from(x0), f64::from(x1) + 1.0))
        } else {
            stand_span(y)
        }
    }

    fn span_top(&self, lx: f64) -> f64 {
        for y in -48..=0 {
            let yf = f64::from(y);
            if let Some((l, r)) = self.span(yf)
                && lx >= l
                && lx <= r
            {
                return yf;
            }
        }
        0.0
    }

    fn draw_streaks(&self, p: &mut Painter, f: f64, ramp: &[Rgba; 5], seed: f64, first: bool) {
        let sx = 1.0 - f * 0.86;
        p.each(-14.0, -64.0, 14.0, 1.0, |painter, x, y, px, py| {
            let yn = clamp((y - CY) / 26.0, -1.2, 1.2);
            let s = sx * (1.0 - f * 0.5 * yn * yn);
            if s <= 0.02 {
                return;
            }
            let lx = x / s;
            let col = libm::floor(lx + 20.0);
            let cr = rand(&[seed, col, 11.0]);
            let ly = y / (1.0 + (f * 0.35 + 0.04) * cr);
            let Some((sp0, sp1)) = self.span(ly) else {
                return;
            };
            if lx < sp0 || lx > sp1 {
                return;
            }
            if rand(&[seed, col, 13.0]) < 0.12 + f * 0.2 {
                let a = -40.0 + rand(&[seed, col, 14.0]) * 30.0;
                let b = a + 5.0 + rand(&[seed, col, 15.0]) * 12.0;
                if ly > a && ly < b {
                    return;
                }
            }
            let w = (sp1 - sp0) / 2.0;
            let mid = (sp0 + sp1) / 2.0;
            let cen = 1.0 - libm::fabs(lx - mid) / (w + 0.5);
            let mut v = 0.6 + f * 1.9 + cen * 1.3 + if cr > 0.72 { 1.0 } else { 0.0 }
                - if first { 0.5 } else { 0.0 };
            if lx - sp0 < 1.2 / js_max(s, 0.3) {
                v += 0.8;
            }
            if ly - self.span_top(lx) < 2.0 {
                v += 0.6;
            }
            let ci = clamp(libm::floor(v), 0.0, (ramp.len() - 1) as f64) as usize;
            painter.put(px, py, ramp[ci], 1.0);
        });
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
        let k = js_round(t * f64::from(FPS)) as i32;
        if !(0..END).contains(&k) {
            return;
        }
        let [n_col, v_col, p_col, l_col, pa_col, w_col] = self.c;
        let ramp = [n_col, v_col, p_col, l_col, pa_col];
        let ku = k as usize;

        // Floor glyph
        let gl = [1, 2, 3, 3, 4, 3, 3, 3, 2, 2, 2, 2, 2, 1, 1, 1, 0, 0][ku];
        let gone = [
            0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.08, 0.12, 0.2, 0.28, 0.36, 0.45, 0.55, 0.62, 0.74,
            0.86, 1.0, 1.0,
        ][ku];
        let glyph_seed = f64::from(hash(&[self.seed, 300.0]));
        draw_glyph(
            p,
            gl,
            gone,
            &ramp,
            glyph_seed,
            if k == 0 { 0.55 } else { 1.0 },
            self.glyph,
        );
        if gl >= 3 {
            p.glow(0.0, 0.0, 14.0, p_col, 0.18 * f64::from(gl));
        }

        // Rune motes rising from the figure's space
        for i in 0..self.motes {
            let fi = f64::from(i);
            let mut r = Rng::new(hash(&[self.seed, 400.0, fi]));
            let b = 3 + r.int(0.0, 7.0) as i32;
            let life = 4 + r.int(0.0, 5.0) as i32;
            let age = k - b;
            if age < 0 || age >= life || b + life >= END {
                continue;
            }
            let near = r.next() < 0.45;
            let x0 = r.range(-12.0, 12.0);
            let y0 = r.range(-40.0, -6.0);
            let vy = if near {
                r.range(0.9, 1.6)
            } else {
                r.range(0.4, 0.9)
            };
            let ph = r.next() * TAU;
            let f_age = f64::from(age);
            let x = js_round(x0 * (1.0 + f_age * 0.05) + libm::sin(ph + f_age * 0.9) * 1.2);
            let y = js_round(y0 - vy * f_age);
            let u = f_age / f64::from(life);
            if age > 0 && rand(&[self.seed, 401.0, fi, f64::from(k)]) < 0.15 {
                continue;
            }
            if near {
                let near_cols = [pa_col, l_col, l_col, p_col, p_col, v_col];
                let c = near_cols[(libm::floor(u * 6.0) as usize).min(5)];
                if u < 0.55 {
                    draw_rune(p, x, y, r.int(0.0, 4.0) as usize, c);
                } else {
                    p.px(x, y, c);
                    p.px(x, y - 1.0, c);
                }
                if u < 0.6 {
                    p.glow(x, y, 4.0, p_col, 0.35 * (1.0 - u));
                }
            } else {
                let far_cols = [l_col, p_col, v_col, v_col];
                let c = far_cols[(libm::floor(u * 4.0) as usize).min(3)];
                p.px(x, y, c);
                if u < 0.4 {
                    p.px(x + 1.0, y, c);
                }
            }
        }

        // The fold: streaks pinch toward the spine, specks sucked in
        if k <= 3 {
            let speck_cols = [v_col, p_col, l_col, l_col];
            for i in 0..self.specks {
                let mut r = Rng::new(hash(&[self.seed, 500.0, f64::from(i)]));
                let ang = r.next() * TAU;
                let rad = r.range(20.0, 28.0) - f64::from(k) * r.range(5.0, 7.0);
                if rad < 4.0 {
                    continue;
                }
                let len = 2.0 + r.int(0.0, 2.0);
                let (ex, ey) = (libm::cos(ang), libm::sin(ang) * 1.4);
                let (x, y) = (ex * rad, CY + ey * rad);
                p.line(x, y, x - ex * len, y - ey * len, speck_cols[ku], 1.0);
            }
        }
        let streak_seed = f64::from(hash(&[self.seed, 100.0]));
        if k == 0 {
            self.draw_streaks(p, 0.0, &ramp, streak_seed, true);
        } else if k == 1 {
            self.draw_streaks(p, 0.36, &ramp, streak_seed, false);
        } else if k == 2 {
            self.draw_streaks(p, 0.72, &ramp, streak_seed, false);
        }
        if (1..=2).contains(&k) {
            p.glow_line(0.0, -40.0, 0.0, -4.0, 6.0, p_col, 0.3 + f64::from(k) * 0.15);
        }

        if k == 3 {
            p.poly(
                &[
                    (0.5, -55.0),
                    (3.0, CY - 4.0),
                    (2.0, -2.0),
                    (0.5, 1.0),
                    (-1.5, -2.0),
                    (-2.2, CY - 2.0),
                ],
                l_col,
                1.0,
            );
            p.poly(
                &[(0.5, -50.0), (1.4, CY), (0.5, -3.0), (-0.6, CY)],
                pa_col,
                1.0,
            );
            p.glow_line(0.5, -52.0, 0.5, 0.0, 6.0, l_col, 0.6);
        }

        if k == PEAK {
            wedge(p, 0.5, CY, -22.0, CY + 1.0, 3.0, 0.6, l_col, 1.0);
            wedge(p, 0.5, CY, 23.0, CY - 1.0, 3.0, 0.6, l_col, 1.0);
            wedge(p, 0.5, CY, -15.0, CY + 0.5, 1.6, 0.4, w_col, 1.0);
            wedge(p, 0.5, CY, 16.0, CY - 0.5, 1.6, 0.4, w_col, 1.0);
            p.poly(
                &[
                    (0.5, -68.0),
                    (4.6, CY - 7.0),
                    (3.8, -3.0),
                    (0.5, 2.0),
                    (-3.0, -3.0),
                    (-3.6, CY - 2.0),
                ],
                p_col,
                1.0,
            );
            p.poly(
                &[
                    (0.5, -66.0),
                    (3.0, CY - 6.0),
                    (2.4, -2.0),
                    (0.5, 1.0),
                    (-1.8, -2.0),
                    (-2.2, CY - 2.0),
                ],
                l_col,
                1.0,
            );
            p.poly(
                &[
                    (0.5, -62.0),
                    (1.9, CY),
                    (1.3, -4.0),
                    (0.5, -1.0),
                    (-0.9, CY),
                ],
                w_col,
                1.0,
            );
            flash_star(
                p,
                0.5,
                CY,
                9.0,
                l_col,
                4,
                0.3,
                core::f64::consts::FRAC_PI_4,
                1.0,
            );
            flash_star(
                p,
                0.5,
                CY,
                6.5,
                w_col,
                4,
                0.3,
                core::f64::consts::FRAC_PI_4,
                1.0,
            );
            p.glow(0.5, CY, 26.0, p_col, 0.9);
            p.glow_line(0.5, -64.0, 0.5, 0.0, 8.0, l_col, 0.6);
        }

        // The snap: the line leaves upward
        if k == 5 {
            wedge(p, 0.5, -86.0, 0.5, -42.0, 4.4, 0.6, l_col, 1.0);
            wedge(p, 0.5, -86.0, 0.5, -54.0, 2.2, 0.6, pa_col, 1.0);
            p.line(0.5, -38.0, 0.5, -37.0, p_col, 1.0);
            p.glow_line(0.5, -80.0, 0.5, -34.0, 5.0, p_col, 0.5);
        } else if k == 6 {
            wedge(p, 0.5, -84.0, 0.5, -62.0, 2.4, 0.4, p_col, 1.0);
            p.line(0.5, -58.0, 0.5, -55.0, v_col, 1.0);
            p.line(0.5, -52.0, 0.5, -51.0, v_col, 1.0);
        } else if k == 7 {
            p.line(0.5, -80.0, 0.5, -76.0, v_col, 1.0);
            p.px(0.5, -72.0, n_col);
        }

        // Implosion ring
        if (5..=9).contains(&k) {
            let j = (k - 5) as usize;
            let rs_list = [12.5, 9.0, 6.0, 3.5, 2.0];
            let rs = rs_list[j];
            if j <= 2 {
                let ws = [4.6, 3.8, 2.8];
                let cols = [[pa_col, l_col], [l_col, p_col], [p_col, v_col]];
                let brokens = [0.06, 0.14, 0.2];
                let tears = [0.45, 0.55, 0.6];
                lump_ring(
                    p,
                    0.5,
                    CY,
                    rs,
                    ws[j],
                    cols[j],
                    f64::from(hash(&[self.seed, 600.0])),
                    brokens[j],
                    tears[j],
                );
            } else if j == 3 {
                flash_star(p, 0.5, CY, 5.0, p_col, 4, 0.32, 0.3, 1.0);
                flash_star(p, 0.5, CY, 3.0, l_col, 4, 0.3, 0.3, 1.0);
            } else if j == 4 {
                p.rect(0.0, CY - 1.0, 2.0, 2.0, v_col, 1.0);
                p.px(0.5, CY - 3.0, n_col);
                p.px(-1.5, CY + 2.0, n_col);
            }
            if j <= 3 {
                p.glow(0.5, CY, rs + 4.0, p_col, 0.5 - f64::from(j as i32) * 0.1);
            }
        }
    }
}
