//! `disappear/incinerate`: the wrapped sprite catches fire from the feet up, flares white, and collapses from the
//! crown down into tumbling black ash flakes that drift up and away, leaving a char smear on the floor.
//!
//! Adapted from Pixel VFX Studio's `fire/incinerate` (MIT License, Copyright (c) 2026 Patch, Copyright (c) 2026 Hugo
//! Duprez). Wraps its actor ([`Effect::wraps_actor`]): when handed a sprite via [`EffectState::set_actor`], it reads
//! [`Silhouette`] and its pixels once per pose: until the flare the sprite burns in its own colours, charring from the
//! feet up behind an ember line, and the charred body and its crumbling 3x3 ash chunks come from the sprite's own
//! cells, crown included. Flame roots climb the silhouette's edge, loose ash starts on that edge, and the glow
//! and floor char follow the sprite's centre and footprint. With no sprite it uses the stand-in silhouette.

use std::sync::Arc;

use pixfx_core::cue::{Cue, FEEL, TimedCue};
use pixfx_core::{Actor, Area, Effect, EffectState, Layer, Meta, NumOpt, Options, Silhouette};
use pixfx_draw::Painter;
use pixfx_draw::color::{Rgba, mix};
use pixfx_draw::math::{Rng, TAU, clamp, js_max, js_min, js_round, lerp, rng, to_int32};

pub struct Incinerate;

pub static INCINERATE: Incinerate = Incinerate;

const FPS: u32 = 15;
const MS: f64 = 0.07;
const N: i32 = 18;
const PEAK: i32 = 4;
const K_T: f64 = 0x1c7 as f64;
const K_A: f64 = 0xa5 as f64;
const K_C: f64 = 0xc4 as f64;
/// 3x3 pixel ash chunks (`Ruler::size(RULER.hand, 1.0)`).
const CH: i32 = 3;
/// Reference dimensions for sizing flames and glow.
const STAND_IN_HALF_W: f64 = 6.2;
const STAND_IN_H: f64 = 31.1;
/// The stand-in's ash grid: chunk columns and rows, and its top row (the crown that crumbles first).
const STAND_IN_COLS: (i32, i32) = (-3, 3);
const STAND_IN_ROWS: i32 = 11;
const STAND_IN_TOP: i32 = 10;
/// How far toward char a pixel below the burn front is.
const CHAR: f64 = 0.8;
const BOX: Area = Area {
    x: -22,
    y: -60,
    w: 64,
    h: 64,
};

const FLAKES: NumOpt = NumOpt::new("flakes", 14.0).within(8.0, 28.0);
const BREEZE: NumOpt = NumOpt::new("breeze", 1.0).within(0.0, 1.2);

const META: Meta = Meta {
    id: "disappear/incinerate",
    name: "Incinerate",
    desc: "The wrapped sprite catches fire from the feet up, flares white, and collapses into tumbling black ash flakes with dying ember edges that drift up and away, leaving a char smear on the floor.",
    tags: &[
        "disappear",
        "fire",
        "burn",
        "ash",
        "dissolve",
        "character",
        "one-shot",
    ],
    size: (BOX.w, BOX.h),
    origin: (-BOX.x, -BOX.y),
    duration: 1.2,
    looping: false,
    fps: FPS,
    seed: 4_128_573_911,
    state: None,
    options: &[FLAKES.spec(), BREEZE.spec(), FEEL.spec()],
};

const PALETTE: [Rgba; 8] = [
    Rgba::hex24(0x151017),
    Rgba::hex24(0x4b4250),
    Rgba::hex24(0x3a1430),
    Rgba::hex24(0xb3262f),
    Rgba::hex24(0xee5b23),
    Rgba::hex24(0xffa531),
    Rgba::hex24(0xffe066),
    Rgba::hex24(0xffffff),
];

const COLH: [f64; 10] = [0.3, 0.55, 0.85, 1.0, 1.12, 0.85, 0.62, 0.42, 0.28, 0.16];
const HEAT: [i32; 10] = [0, 0, 0, 0, 1, 0, -1, -1, -2, -2];

const SHAPES: [&[(i32, i32)]; 6] = [
    &[(0, 0), (1, 0), (0, -1), (1, -1), (2, -1)],
    &[(0, 0), (1, 0), (1, -1), (2, -1)],
    &[(0, 0), (1, 0), (2, 0)],
    &[(0, 0), (0, -1), (1, -1), (1, -2)],
    &[(0, 0), (1, 0), (1, -1), (0, 1)],
    &[(0, 0), (0, -1), (0, -2)],
];

impl Effect for Incinerate {
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
                k: clamp(0.2 * k, 0.0, 1.0),
                dur: 0.09,
            },
            pan: None,
        }]
    }

    fn spawn(&self, opts: &Options, seed: u64) -> Box<dyn EffectState> {
        let pal = opts.palette_or(&PALETTE);
        let c = |i: usize| pal.get(i).copied().unwrap_or(PALETTE[0]);
        Box::new(State {
            c: [c(0), c(1), c(2), c(3), c(4), c(5), c(6), c(7)],
            flakes: js_round(opts.num(&FLAKES)) as usize,
            breeze: opts.num(&BREEZE),
            seed: to_int32(seed as f64) as f64,
            body: None,
            actor_set: false,
        })
    }
}

struct State {
    c: [Rgba; 8],
    flakes: usize,
    breeze: f64,
    seed: f64,
    body: Option<Body>,
    actor_set: bool,
}

/// The sprite it burns, read once per pose: the cells it covers, its colours over `BOX`, and how it is sized.
struct Body {
    sil: Silhouette,
    px: Vec<Option<Rgba>>,
    /// Its flames, glow, floor and loose ash against the stand-in's: across and up.
    sx: f64,
    sy: f64,
    bounds: Area,
    centre: (f64, f64),
    edge: Vec<(i32, i32)>,
    footprint: (f64, f64),
    chunks: Vec<Chunk>,
}

impl Body {
    fn of(actor: &Actor, seed: f64) -> Option<Body> {
        let sil = Silhouette::of(actor, BOX);
        let b = sil.bounds()?;
        let (x0, x1) = (b.x, b.x + b.w as i32);
        let floor = b.y + b.h as i32;
        let centre = sil.centroid()?;
        let edge = sil.edge().collect();
        let (left, right) = sil
            .cells()
            .filter(|&(_, y)| y == floor - 1)
            .fold((x1, x0), |(left, right), (x, _)| {
                (left.min(x), right.max(x + 1))
            });
        let cols = (x0.div_euclid(CH), (x1 - 1).div_euclid(CH) + 1);
        let top = (b.h as i32 - 1).div_euclid(CH);
        let chunks = chunks(seed, cols, top + 1, top.max(1), floor, |x, y| {
            sil.covers(x, y)
        });
        let mut px = vec![None; (BOX.w * BOX.h) as usize];
        for (x, y, c) in actor.opaque() {
            if let Some(i) = box_index(x, y) {
                px[i] = Some(c);
            }
        }
        Some(Body {
            sil,
            px,
            sx: f64::from(b.w) / (2.0 * STAND_IN_HALF_W),
            sy: f64::from(b.h) / STAND_IN_H,
            bounds: b,
            centre,
            edge,
            footprint: (f64::from(left + right) / 2.0, f64::from(right - left)),
            chunks,
        })
    }

    fn floor(&self) -> i32 {
        self.bounds.y + self.bounds.h as i32
    }

    fn front(&self, k: i32) -> f64 {
        f64::from(self.bounds.h) * f64::from((k + 1).min(PEAK + 1)) / f64::from(PEAK + 1)
    }

    fn flame_root(&self, x: f64, k: i32) -> Option<(f64, f64)> {
        let target_x = f64::from(self.bounds.x) + f64::from(self.bounds.w) / 2.0 + x;
        let target_y = f64::from(self.floor()) - self.front(k) * 0.8;
        self.edge
            .iter()
            .map(|&(x, y)| (f64::from(x) + 0.5, f64::from(y) + 0.5))
            .filter(|&(_, y)| y >= f64::from(self.floor()) - self.front(k).max(0.5))
            .min_by(|&(ax, ay), &(bx, by)| {
                let distance = |x: f64, y: f64| {
                    let dx = x - target_x;
                    let dy = y - target_y;
                    dx * dx + 2.0 * dy * dy
                };
                distance(ax, ay).total_cmp(&distance(bx, by))
            })
    }

    fn color(&self, x: i32, y: i32) -> Option<Rgba> {
        box_index(x, y).and_then(|i| self.px.get(i).copied().flatten())
    }
}

/// Where pixel (`x`, `y`) of the effect's space is in `BOX`, none outside it.
fn box_index(x: i32, y: i32) -> Option<usize> {
    let (i, j) = (x - BOX.x, y - BOX.y);
    (i >= 0 && j >= 0 && i < BOX.w as i32 && j < BOX.h as i32)
        .then(|| (j * BOX.w as i32 + i) as usize)
}

struct Tongue {
    x: f64,
    y: f64,
    h: f64,
    w: f64,
    lean: f64,
    lick: f64,
    curl: f64,
    cut: (f64, f64),
    z: f64,
    heat: i32,
    hollow: f64,
}

struct Chunk {
    cx: i32,
    cy: i32,
    x: f64,
    y: f64,
    at: f64,
    r: Rng,
    mask: u16,
}

fn stand_in_body(x: f64, y: f64) -> bool {
    let (tx, ty) = (x / 6.2, (y + 15.0) / 9.5);
    if tx * tx + ty * ty <= 1.0 {
        return true;
    }
    let (hx, hy) = (x / 3.6, (y + 27.5) / 3.6);
    if hx * hx + hy * hy <= 1.0 {
        return true;
    }
    y > -8.0 && y < 0.0 && libm::fabs(x) < 5.2 && !(libm::fabs(x) < 1.2 && y > -5.0)
}

fn bez(pts: &[(f64, f64); 4], s: f64) -> (f64, f64) {
    let r = 1.0 - s;
    let a = r * r * r;
    let b = 3.0 * s * r * r;
    let c = 3.0 * s * s * r;
    let d = s * s * s;
    (
        a * pts[0].0 + b * pts[1].0 + c * pts[2].0 + d * pts[3].0,
        a * pts[0].1 + b * pts[1].1 + c * pts[2].1 + d * pts[3].1,
    )
}

fn spine_of(t: &Tongue) -> [(f64, f64); 4] {
    let at = |a: f64, b: f64| (t.x + b, t.y - a);
    [
        at(0.0, 0.0),
        at(t.h * 0.35, -t.lick * 0.4),
        at(t.h * 0.72, t.lean * 0.3 + t.lick - t.curl * 0.5),
        at(t.h, t.lean + t.curl),
    ]
}

fn near_poly(pts: &[(f64, f64)], x: f64, y: f64) -> (f64, f64) {
    let mut best = f64::INFINITY;
    let mut bs = 0.0;
    let n = pts.len().saturating_sub(1);
    for (i, pair) in pts.windows(2).enumerate() {
        let ((ax, ay), (bx, by)) = (pair[0], pair[1]);
        let (dx, dy) = (bx - ax, by - ay);
        let l2 = dx * dx + dy * dy;
        let l2 = if l2 == 0.0 || l2.is_nan() { 1.0 } else { l2 };
        let u = clamp(((x - ax) * dx + (y - ay) * dy) / l2, 0.0, 1.0);
        let (ex, ey) = (ax + dx * u - x, ay + dy * u - y);
        let d = ex * ex + ey * ey;
        if d < best {
            best = d;
            bs = (f64::from(i as u32) + u) / f64::from(n.max(1) as u32);
        }
    }
    (libm::sqrt(best), bs)
}

fn prof(s: f64) -> f64 {
    if s < 0.18 {
        lerp(0.85, 1.0, s / 0.18)
    } else {
        libm::pow(clamp((1.0 - s) / 0.82, 0.0, 1.0), 0.8)
    }
}

fn shell(m: f64, w: f64, q: f64) -> i32 {
    if m < 1.0 {
        return 0;
    }
    let inner = (m - 1.0) / js_max(1.0, w - 1.0);
    if inner > 0.55 + 0.9 * q && q < 0.5 {
        3
    } else if inner > 0.3 + 0.6 * q && q < 0.72 {
        2
    } else if q < 0.9 {
        1
    } else {
        0
    }
}

fn tongues(seed: f64, k: i32, t: f64, (sx, sy): (f64, f64), body: Option<&Body>) -> Vec<Tongue> {
    if k < 0 || (k as usize) >= COLH.len() {
        return Vec::new();
    }
    let h_scale = COLH[k as usize];
    let heat = HEAT[k as usize];
    let specs: [(f64, f64, f64, f64, i32); 7] = [
        (-1.0, 36.0, 6.2, 1.0, 0),
        (-7.0, 24.0, 4.2, -4.0, 1),
        (6.0, 27.0, 4.4, 4.0, 0),
        (-4.0, 30.0, 4.4, -2.0, 1),
        (3.0, 32.0, 4.4, 2.0, 2),
        (-10.0, 14.0, 3.0, -5.0, 2),
        (10.0, 16.0, 3.2, 5.0, 1),
    ];
    let mut out = Vec::new();
    for (i, &(x, h, w, lean, del)) in specs.iter().enumerate() {
        let mut r = rng(&[seed, K_T, f64::from(i as u32)]);
        let age = k - del;
        if age < 0 {
            continue;
        }
        let grow = clamp(f64::from(age + 1) / 3.0, 0.0, 1.0);
        let rem = (i % 3) as i32;
        let hollow = if k >= 5 + rem {
            clamp(f64::from(k - 4 - rem) / 4.0, 0.0, 1.0)
        } else {
            0.0
        };
        if hollow >= 1.0 {
            continue;
        }
        let mut hh = h * sy * h_scale * grow * r.range(0.88, 1.12);
        if body.is_some() {
            hh = (hh * 0.52).max(2.0);
        }
        if hh < 2.0 {
            continue;
        }
        let x_val = x * sx + r.range(-1.0, 1.0);
        let mut w_val =
            w * sx * js_min(1.0, grow + 0.3) * if i == 0 && k == 4 { 1.25 } else { 1.0 };
        if body.is_some() {
            w_val = w_val.clamp(1.0, 6.0);
        }
        let lean_scale = if body.is_some() { sx.min(1.4) } else { sx };
        let lean_val = lean * lean_scale + r.range(-1.0, 1.0);
        let lick_amp = r.range(0.8, 1.6);
        let lick_freq = r.range(2.2, 3.2);
        let lick_ph = r.next();
        let lick = lick_amp * libm::sin(TAU * (t * lick_freq + lick_ph));
        let curl_sign = if r.next() < 0.5 { -1.0 } else { 1.0 };
        let curl = curl_sign * r.range(1.0, 2.5);
        let cut_x = (if x < 0.0 { -1.0 } else { 1.0 }) * r.range(2.0, 3.0);
        let cut_y = r.range(2.0, 3.5);
        let t_heat = if hollow > 0.0 {
            0
        } else {
            (if i == 0 { heat } else { heat.min(0) }) + if i >= 5 { -1 } else { 0 }
        };
        let (x, y) = match body {
            Some(body) => {
                let Some(root) = body.flame_root(x_val, k) else {
                    continue;
                };
                root
            }
            None => (x_val, 0.0),
        };
        if body.is_some() {
            // Let the tip taper inside the canvas when the crown is near its top edge.
            hh = hh.min(y - f64::from(BOX.y) - 1.0);
            if hh < 2.0 {
                continue;
            }
        }
        out.push(Tongue {
            x,
            y,
            h: hh,
            w: w_val,
            lean: lean_val,
            lick,
            curl,
            cut: (cut_x, cut_y),
            z: if i == 0 { 1.0 } else { 0.0 },
            heat: t_heat,
            hollow,
        });
    }
    out
}

fn paint_flames(p: &mut Painter, tongues: &[Tongue], cols: &[Rgba; 6], k: i32) {
    struct Live<'a> {
        t: &'a Tongue,
        pts: Vec<(f64, f64)>,
        box_bounds: [f64; 4],
    }
    let live: Vec<Live<'_>> = tongues
        .iter()
        .map(|t| {
            let ctrl = spine_of(t);
            let mut pts = Vec::with_capacity(13);
            let (mut x0, mut y0, mut x1, mut y1) = (
                f64::INFINITY,
                f64::INFINITY,
                f64::NEG_INFINITY,
                f64::NEG_INFINITY,
            );
            for i in 0..=12 {
                let pt = bez(&ctrl, f64::from(i) / 12.0);
                x0 = x0.min(pt.0);
                x1 = x1.max(pt.0);
                y0 = y0.min(pt.1);
                y1 = y1.max(pt.1);
                pts.push(pt);
            }
            Live {
                t,
                pts,
                box_bounds: [
                    x0 - t.w - 1.0,
                    y0 - t.w - 1.0,
                    x1 + t.w + 1.0,
                    y1 + t.w + 1.0,
                ],
            }
        })
        .collect();

    for j in 0..BOX.h as i32 {
        let y = f64::from(BOX.y + j) + 0.5;
        for i in 0..BOX.w as i32 {
            let x = f64::from(BOX.x + i) + 0.5;
            let mut b1 = f64::NEG_INFINITY;
            let mut m1 = 0.0;
            let mut s1 = 0.0;
            let mut l1: Option<&Live<'_>> = None;
            let mut m2 = f64::NEG_INFINITY;
            let mut s2 = 0.0;
            for l in &live {
                let b = l.box_bounds;
                if x < b[0] || x > b[2] || y < b[1] || y > b[3] {
                    continue;
                }
                let t = l.t;
                let (d, s) = near_poly(&l.pts, x, y);
                let m = t.w * prof(s) - d;
                if m <= 0.0 {
                    continue;
                }
                if t.hollow > 0.0 {
                    let (d2, s2b) = near_poly(&l.pts, x - t.cut.0, y - t.cut.1);
                    let mc = t.w * lerp(0.35, 1.3, t.hollow) * prof(s2b) - d2;
                    if mc > 0.0 || m > lerp(t.w, 1.4, t.hollow) {
                        continue;
                    }
                }
                let sc = m + t.z;
                if sc > b1 {
                    if l1.is_some() {
                        m2 = m1;
                        s2 = s1;
                    }
                    b1 = sc;
                    m1 = m;
                    s1 = s;
                    l1 = Some(l);
                } else if m > m2 {
                    m2 = m;
                    s2 = s;
                }
            }
            let Some(chosen) = l1 else {
                continue;
            };
            let t = chosen.t;
            let raw = shell(m1, t.w, s1);
            let hollow_add = if t.hollow > 0.0 {
                js_round(lerp(1.4, 0.0, t.hollow)) as i32
            } else {
                0
            };
            let mut v = raw + t.heat + hollow_add;
            if raw == 0 && s1 > 0.62 {
                v -= 1;
            }
            if m2 > 0.0 && m1 - m2 < 0.7 && s1 > 0.25 && s2 > 0.25 && raw > 0 {
                v -= 1;
            }
            if t.hollow > 0.0 {
                v = v.min(2);
            }
            let lvl = v.clamp(-1, 4);
            let max_lvl = if k == 4 { 4 } else { 3 };
            let idx = (lvl.min(max_lvl) + 1) as usize;
            if let Some(&col) = cols.get(idx) {
                p.px(x, y, col);
            }
        }
    }
}

fn chunks(
    seed: f64,
    (c0, c1): (i32, i32),
    rows: i32,
    top: i32,
    floor: i32,
    covers: impl Fn(i32, i32) -> bool,
) -> Vec<Chunk> {
    let mut out = Vec::new();
    for cy in 0..rows {
        for cx in c0..c1 {
            let left = cx * CH;
            let bottom = floor - cy * CH;
            let mut mask = 0;
            for y in 0..CH {
                for x in 0..CH {
                    let cell = (left + x, bottom - y - 1);
                    if covers(cell.0, cell.1) {
                        mask |= 1u16 << (y * CH + x);
                    }
                }
            }
            if mask == 0 {
                continue;
            }
            let anchor = mask.trailing_zeros() as i32;
            let (x, y) = (left + anchor % CH, bottom - anchor / CH - 1);
            let stand_in_cell =
                (STAND_IN_COLS.0..STAND_IN_COLS.1).contains(&cx) && cy < STAND_IN_ROWS;
            let key = if stand_in_cell {
                1000 + cy * 8 + cx
            } else {
                100_000 + cy * 64 + cx
            };
            let mut r = rng(&[seed, K_A, f64::from(key)]);
            let at = 5.6 + (1.0 - f64::from(cy) / f64::from(top)) * 3.2 + r.range(-0.5, 0.6);
            out.push(Chunk {
                cx,
                cy,
                x: f64::from(x) + 0.5,
                y: f64::from(y) + 0.5,
                at,
                r,
                mask,
            });
        }
    }
    out
}

impl State {
    /// Across and up against the stand-in: 1 with no sprite.
    fn scale(&self) -> (f64, f64) {
        self.body.as_ref().map_or((1.0, 1.0), |b| (b.sx, b.sy))
    }

    /// Until the flare the sprite burns in its own colours: charred below a front climbing from the feet, an ember
    /// line along the front, and white-hot at the flare. With no sprite nothing is drawn here.
    fn draw_sprite(&self, p: &mut Painter, k: i32) {
        let Some(b) = &self.body else { return };
        if k > PEAK {
            return;
        }
        let c = &self.c;
        let front = b.front(k);
        for (x, y) in b.sil.cells() {
            let Some(own) = b.color(x, y) else { continue };
            let up = f64::from(b.floor() - y) - 0.5;
            let col = if k == PEAK {
                if (x + y).rem_euclid(3) == 0 {
                    c[6]
                } else {
                    c[7]
                }
            } else if up < front - 2.0 {
                mix(own, c[0], CHAR)
            } else if up < front {
                let r = rng(&[self.seed, K_C, f64::from(x * 97 + y), f64::from(k)]).next();
                if r < 0.5 { c[4] } else { c[5] }
            } else {
                own
            };
            p.paint_cell(f64::from(x) + 0.5, f64::from(y) + 0.5, col, 1.0);
        }
    }

    fn draw_body(&self, p: &mut Painter, k: i32, chunks: &[Chunk]) {
        if k < 5 {
            return;
        }
        let c = &self.c;
        let kf = f64::from(k);
        let floor = self.body.as_ref().map_or(0, Body::floor);
        for ck in chunks {
            if kf >= ck.at {
                continue;
            }
            let mut r = ck.r.clone();
            for y in 0..CH {
                for x in 0..CH {
                    let px = f64::from(ck.cx * CH + x) + 0.5;
                    let py = f64::from(floor - (ck.cy * CH + y)) - 0.5;
                    if ck.mask & (1 << (y * CH + x)) == 0 {
                        continue;
                    }
                    let crack = (x == 0 || y == 0) && r.next() < 0.5;
                    let col = if crack && k < 7 {
                        if k == 5 { c[4] } else { c[3] }
                    } else if (x + y + ck.cy) % 5 == 0 && k >= 6 {
                        c[1]
                    } else {
                        c[0]
                    };
                    p.paint_cell(px, py, col, 1.0);
                }
            }
        }
    }

    fn draw_ash(&self, p: &mut Painter, t: f64, chunks: &[Chunk]) {
        let c = &self.c;
        let ck_len = chunks.len();
        let n = ck_len + self.flakes;
        for i in 0..n {
            let chunk = chunks.get(i);
            let mut r = chunk.map_or_else(
                || rng(&[self.seed, K_A, f64::from(i as u32)]),
                |ck| ck.r.clone(),
            );
            let t0 = match chunk {
                Some(ck) => ck.at * MS,
                None => r.range(4.5, 7.0) * MS,
            };
            let life = r.range(0.5, 0.82);
            let age = t - t0;
            if age < 0.0 || age >= life {
                continue;
            }
            let u = age / life;
            let (x0, y0) = match chunk {
                Some(ck) => (ck.x, ck.y),
                None if let Some(body) = &self.body => {
                    let (x, y) = body.edge[(r.next() * body.edge.len() as f64) as usize];
                    (f64::from(x) + 0.5, f64::from(y) + 0.5)
                }
                None => {
                    let rx0 = r.range(-10.0, 10.0);
                    let ry0 = -r.range(4.0, 30.0) * (0.4 + 0.6 * (1.0 - libm::fabs(rx0) / 12.0));
                    (rx0, ry0)
                }
            };
            let up = r.range(18.0, 30.0);
            let shear = match chunk {
                Some(ck) => 0.55 + f64::from(ck.cy) / 14.0,
                None => 1.0,
            };
            let wind = r.range(10.0, 22.0) * self.breeze * shear;
            let freq = r.range(5.0, 9.0);
            let ph = r.range(0.0, 6.0);
            let amp = r.range(1.0, 2.5);
            let sway = libm::sin(age * freq + ph)
                - if self.body.is_some() {
                    libm::sin(ph)
                } else {
                    0.0
                };
            let x = x0 + wind * age * (0.4 + age) + sway * amp;
            let y = y0 - up * age - 16.0 * age * age;
            if self.body.is_none() && y < -57.0 {
                continue;
            }
            let big = if chunk.is_some() {
                true
            } else {
                r.next() < 0.45
            };
            let spin = r.int(0.0, 5.0) as usize;
            let rate = r.range(0.6, 1.4);
            let sh_idx = (spin + libm::floor((age / MS) * rate).max(0.0) as usize) % SHAPES.len();
            let mut ash_cells = [(0, 0); (CH * CH) as usize];
            let sh = if let Some(ck) = chunk.filter(|_| self.body.is_some()) {
                let quarter_turns = libm::floor(age / MS * rate) as i32 % 4;
                let anchor = ck.mask.trailing_zeros() as i32;
                let mut count = 0;
                for cell in 0..CH * CH {
                    if ck.mask & (1 << cell) == 0 {
                        continue;
                    }
                    let dx = cell % CH - anchor % CH;
                    let dy = anchor / CH - cell / CH;
                    ash_cells[count] = match quarter_turns {
                        1 => (-dy, dx),
                        2 => (-dx, -dy),
                        3 => (dy, -dx),
                        _ => (dx, dy),
                    };
                    count += 1;
                }
                &ash_cells[..count]
            } else {
                SHAPES[sh_idx]
            };
            let scale_u = if u < 0.4 {
                1.0
            } else if u < 0.7 {
                0.6
            } else {
                0.25
            };
            let keep = (js_round(sh.len() as f64 * (if big { 1.0 } else { 0.6 }) * scale_u)
                as usize)
                .clamp(1, sh.len());
            let slice = &sh[..keep];
            for (j, &(dx, dy)) in slice.iter().enumerate() {
                let mut col = c[0];
                if j + 1 == slice.len() && slice.len() > 1 {
                    col = if u < 0.18 {
                        c[4]
                    } else if u < 0.32 {
                        c[3]
                    } else if (libm::floor(age / MS + spin as f64) as i32).rem_euclid(3) == 0 {
                        c[1]
                    } else {
                        c[0]
                    };
                }
                p.px(x + f64::from(dx), y + f64::from(dy), col);
            }
        }
    }

    fn draw_floor(&self, p: &mut Painter, k: i32) {
        if k > 15 {
            return;
        }
        let c = &self.c;
        let (centre, span, floor) = self.body.as_ref().map_or((0.0, 1.0, 0.0), |body| {
            let (centre, width) = body.footprint;
            (
                centre,
                width / (2.0 * STAND_IN_HALF_W),
                f64::from(body.floor()),
            )
        });
        let w = span
            * if k < 2 {
                f64::from(5 + k * 3)
            } else if k < 9 {
                12.0
            } else {
                lerp(12.0, 4.0, f64::from(k - 9) / 6.0)
            };
        let mut x = -w;
        while x <= w {
            let mut r = rng(&[self.seed, K_C, x + 40.0]);
            if k >= 10 && r.next() < f64::from(k - 9) / 7.0 {
                x += 1.0;
                continue;
            }
            p.px(centre + x, floor + 0.5, if k < 8 { c[3] } else { c[0] });
            if libm::fabs(x) < w - 2.0 {
                p.px(centre + x, floor + 1.5, c[0]);
            }
            if (6..13).contains(&k) && r.next() < 0.18 {
                p.px(centre + x, floor + 0.5, if k < 10 { c[4] } else { c[3] });
            }
            x += 1.0;
        }
    }
}

impl EffectState for State {
    fn set_actor(&mut self, actor: Option<Arc<Actor>>) {
        self.actor_set = actor.is_some();
        self.body = actor.and_then(|a| Body::of(&a, self.seed));
    }

    fn draw(&self, p: &mut Painter, t: f64, layer: Layer) {
        if layer != Layer::Front || (self.actor_set && self.body.is_none()) {
            return;
        }
        let k = js_round(t * f64::from(FPS)) as i32;
        if !(0..N).contains(&k) {
            return;
        }
        let t_ms = f64::from(k) * MS;
        let c = &self.c;
        let (sx, sy) = self.scale();
        let (glow_x, glow_y) = self.body.as_ref().map_or((0.0, -14.0), |b| b.centre);
        if k < 9 {
            let ch = COLH.get(k as usize).copied().unwrap_or(0.0);
            p.glow(
                glow_x,
                glow_y,
                if k == 4 { 26.0 } else { 18.0 } * sy,
                c[4],
                if k == 4 { 0.55 } else { 0.3 * ch },
            );
        }
        self.draw_floor(p, k);
        let t_list = tongues(self.seed, k, t_ms, (sx, sy), self.body.as_ref());
        if !t_list.is_empty() {
            let flame_cols = [c[2], c[3], c[4], c[5], c[6], c[7]];
            paint_flames(p, &t_list, &flame_cols, k);
        }
        if let Some(body) = &self.body {
            self.draw_ash(p, t_ms, &body.chunks);
            p.clean_flecks(3, false);
            // Preserve fine sprite details and let the ember line read against the flames.
            self.draw_sprite(p, k);
            self.draw_body(p, k, &body.chunks);
        } else {
            let mut chunks = chunks(
                self.seed,
                STAND_IN_COLS,
                STAND_IN_ROWS,
                STAND_IN_TOP,
                0,
                |x, y| stand_in_body(f64::from(x) + 0.5, f64::from(y) + 0.5),
            );
            for chunk in &mut chunks {
                chunk.x = f64::from(chunk.cx * CH) + 1.5;
                chunk.y = f64::from(-(chunk.cy * CH)) - 1.5;
            }
            self.draw_body(p, k, &chunks);
            self.draw_ash(p, t_ms, &chunks);
            p.clean_flecks(3, false);
        }
        if k == PEAK {
            let flare_y = self.body.as_ref().map_or(-18.0, |b| b.centre.1);
            p.glow(glow_x, flare_y, 12.0 * sy, c[6], 0.4);
        }
    }
}
