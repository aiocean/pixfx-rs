//! Frames in between the drawn ones, made by moving the drawn pixels. An image model draws an effect in a few
//! frames with large changes between them; played as drawn it stutters. Here every pixel of a drawn frame finds the
//! pixel it becomes in the next, and an in-between at time `t` (0 at the first frame, 1 at the next) shows each
//! pixel `t` of the way along:
//!
//! 1. Coarse motion: per block of [`BLOCK`] pixels, the move within [`SEARCH`] pixels that best matches the block's
//!    brightness in the next frame (a small cost per pixel moved keeps it short), then per block the median of the
//!    moves of the blocks round it (3 by 3) that have something to match: an empty or flat block has no say, so a
//!    small thing moving among empty blocks keeps its move, and a flat block takes its neighbours'.
//! 2. Pairs: each pixel looks within [`NEAR`] pixels of where its block's move takes it for a pixel of the same kind
//!    (colour or light) in the next frame; the cost is the squared distance (per `NEAR` squared) plus
//!    [`COLOUR_COST`] times the colour difference (0..1). Cheapest pairs first, each pixel in one pair at most, none
//!    dearer than [`MAX_COST`].
//! 3. A dense motion: every cell's move is the inverse-distance-weighted mean of the [`K`] nearest pairs' moves, one
//!    field from the first frame to the next and one back.
//! 4. Pixels with no partner: one of the first frame vanishes from the far side of the next frame's shape first, one
//!    of the next frame appears nearest the first frame's shape first (a growth front, not a fade).
//! 5. An in-between gathers: each of its cells follows the fields back to where it was in the first frame and on to
//!    where it will be in the next, so every cell gets a value and none is left empty by rounding. A cell lit in both
//!    mixes their colours if they are alike (same kind, channels within [`ALIKE`]); unlike colours are never mixed,
//!    the cell takes one or the other, and swaps at its own time, read from a smooth noise over the first frame
//!    (between [`SWAP`] times), so a patch swaps as one and a frame swaps as a soft wipe rather than at once.
//!
//! Deterministic: pairs tie by the first frame's pixel, then the next's, in reading order; the noise is a hash.

use crate::matrix::{Frame, Matrix, Px};

/// Pixels a side of a block for the coarse motion.
pub const BLOCK: usize = 8;
/// Pixels a block's move is searched within.
pub const SEARCH: i32 = 6;
/// Pixels a pixel's partner is searched within, around where its block's move takes it.
pub const NEAR: f64 = 4.0;
/// Weight of the colour difference against the distance in a pair's cost.
pub const COLOUR_COST: f64 = 3.0;
/// Pairs dearer than this are not made.
pub const MAX_COST: f64 = 1.5;
/// Pairs each cell's move is the weighted mean of.
pub const K: usize = 6;
/// Two colours are alike, and mixed, when no channel differs by more than this (0..255).
pub const ALIKE: f64 = 60.0;
/// The earliest and latest time a cell of unlike colours swaps.
pub const SWAP: (f64, f64) = (0.15, 0.85);
/// Blocks flatter than this (standard deviation of brightness) keep no move.
const FLAT: f64 = 4.0;
/// Cost per pixel a block moves.
const MOVE_COST: f64 = 0.3;
/// Steps of following a field back to its source.
const STEPS: usize = 6;
/// Spread of the swap noise, in pixels.
const NOISE_SIGMA: f64 = 3.0;
/// Luma weights.
const LUM: [f64; 3] = [0.299, 0.587, 0.114];

/// The row a frame belongs to: its name without the last `-n` (`init-2` is in `init`).
pub fn row(name: &str) -> &str {
    name.rsplit_once('-').map_or(name, |s| s.0)
}

/// The name of in-between `k` (from 1) of the frames `a` and `b`: `init-1>init-2@3`.
pub fn name(a: &str, b: &str, k: usize) -> String {
    format!("{a}>{b}@{k}")
}

/// How two frames' pixels move into each other: the two dense fields, when each unpaired pixel vanishes or appears,
/// the swap times, and how many pairs were made.
#[derive(Clone, Debug, PartialEq)]
pub struct Motion {
    pub w: usize,
    pub h: usize,
    /// Pixels paired.
    pub pairs: usize,
    /// Per cell, its move from the first frame to the next (rows, columns), and back.
    forth: Vec<[f64; 2]>,
    back: Vec<[f64; 2]>,
    /// Per cell of the first frame, the time it is gone by (above 1 when it has a partner); of the next frame, the
    /// time it shows from (below 0 when it has a partner).
    gone: Vec<f64>,
    shows: Vec<f64>,
    /// Per cell of the first frame, when unlike colours there swap.
    swap: Vec<f64>,
}

fn kind(p: Px) -> u8 {
    match p {
        Px::Empty => 0,
        Px::Colour { .. } => 1,
        Px::Light { .. } => 2,
    }
}

fn rgb(p: Px) -> [f64; 3] {
    match p {
        Px::Empty => [0.0; 3],
        Px::Colour { rgb, .. } | Px::Light { rgb, .. } => rgb.map(f64::from),
    }
}

fn luma(p: Px) -> f64 {
    let c = rgb(p);
    c[0] * LUM[0] + c[1] * LUM[1] + c[2] * LUM[2]
}

/// Per block, its move (rows, columns) into the next frame's brightness, median-filtered over blocks.
fn coarse(w: usize, h: usize, la: &[f64], lb: &[f64]) -> (usize, Vec<[f64; 2]>) {
    let (bw, bh) = (w / BLOCK + 1, h / BLOCK + 1);
    let mut flow = vec![[0.0; 2]; bw * bh];
    let mut seen = vec![false; bw * bh];
    for by in (0..h).step_by(BLOCK) {
        for bx in (0..w).step_by(BLOCK) {
            let (y1, x1) = ((by + BLOCK).min(h), (bx + BLOCK).min(w));
            let n = ((y1 - by) * (x1 - bx)) as f64;
            let cells = || (by..y1).flat_map(move |y| (bx..x1).map(move |x| (y, x)));
            let mean = cells().map(|(y, x)| la[y * w + x]).sum::<f64>() / n;
            let var = cells()
                .map(|(y, x)| (la[y * w + x] - mean) * (la[y * w + x] - mean))
                .sum::<f64>()
                / n;
            if libm::sqrt(var) < FLAT {
                continue;
            }
            let diff = |dy: i64, dx: i64| {
                cells()
                    .map(|(y, x)| {
                        let (qy, qx) = ((y as i64 + dy) as usize, (x as i64 + dx) as usize);
                        (la[y * w + x] - lb[qy * w + qx]).abs()
                    })
                    .sum::<f64>()
                    / n
            };
            let mut best = (diff(0, 0), 0, 0);
            for dy in -SEARCH..=SEARCH {
                for dx in -SEARCH..=SEARCH {
                    let (y, x) = (by as i64 + i64::from(dy), bx as i64 + i64::from(dx));
                    let (hh, ww) = ((y1 - by) as i64, (x1 - bx) as i64);
                    if y < 0 || x < 0 || y + hh > h as i64 || x + ww > w as i64 {
                        continue;
                    }
                    let s = diff(i64::from(dy), i64::from(dx))
                        + MOVE_COST * libm::sqrt(f64::from(dy * dy + dx * dx));
                    if s < best.0 {
                        best = (s, dy, dx);
                    }
                }
            }
            flow[(by / BLOCK) * bw + bx / BLOCK] = [f64::from(best.1), f64::from(best.2)];
            seen[(by / BLOCK) * bw + bx / BLOCK] = true;
        }
    }
    // per component, the median over the 3 by 3 blocks that had something to match (the lower middle of an even
    // count); none, no move
    let mut out = vec![[0.0; 2]; bw * bh];
    for y in 0..bh {
        for x in 0..bw {
            let round: Vec<usize> = (0..9_i64)
                .filter_map(|i| {
                    let (yy, xx) = (y as i64 + i / 3 - 1, x as i64 + i % 3 - 1);
                    let inside = yy >= 0 && xx >= 0 && yy < bh as i64 && xx < bw as i64;
                    let at = (yy * bw as i64 + xx) as usize;
                    (inside && seen[at]).then_some(at)
                })
                .collect();
            for c in 0..2 {
                let mut v: Vec<f64> = round.iter().map(|&at| flow[at][c]).collect();
                v.sort_by(f64::total_cmp);
                if let Some(m) = v.get(v.len().saturating_sub(1) / 2) {
                    out[y * bw + x][c] = *m;
                }
            }
        }
    }
    (bw, out)
}

/// The cells of a frame that hold a pixel, in reading order, as (row, column).
fn lit(w: usize, px: &[Px]) -> Vec<(usize, usize)> {
    px.iter()
        .enumerate()
        .filter(|(_, p)| kind(**p) > 0)
        .map(|(i, _)| (i / w, i % w))
        .collect()
}

/// Pairs of pixels, first frame to next, as indices into `pa` and `pb`.
fn pair(
    w: usize,
    a: &[Px],
    b: &[Px],
    pa: &[(usize, usize)],
    pb: &[(usize, usize)],
    flow: &(usize, Vec<[f64; 2]>),
) -> Vec<(usize, usize)> {
    let r = NEAR as i64;
    let mut buckets: std::collections::HashMap<(i64, i64), Vec<usize>> =
        std::collections::HashMap::new();
    for (j, &(y, x)) in pb.iter().enumerate() {
        buckets
            .entry((y as i64 / r, x as i64 / r))
            .or_default()
            .push(j);
    }
    let (bw, f) = flow;
    let mut cands: Vec<(f64, usize, usize)> = Vec::new();
    for (i, &(y, x)) in pa.iter().enumerate() {
        let m = f[(y / BLOCK) * bw + x / BLOCK];
        let (py, px) = (y as f64 + m[0], x as f64 + m[1]);
        let (cy, cx) = ((py as i64).div_euclid(r), (px as i64).div_euclid(r));
        let pk = a[y * w + x];
        for gy in cy - 1..=cy + 1 {
            for gx in cx - 1..=cx + 1 {
                for &j in buckets.get(&(gy, gx)).map_or(&[][..], Vec::as_slice) {
                    let (qy, qx) = pb[j];
                    let q = b[qy * w + qx];
                    if kind(q) != kind(pk) {
                        continue;
                    }
                    let d2 =
                        (qy as f64 - py) * (qy as f64 - py) + (qx as f64 - px) * (qx as f64 - px);
                    if d2 > NEAR * NEAR {
                        continue;
                    }
                    let (ca, cb) = (rgb(pk), rgb(q));
                    let c = (0..3).map(|k| (ca[k] - cb[k]).abs()).sum::<f64>() / 3.0 / 255.0;
                    cands.push((d2 / (NEAR * NEAR) + COLOUR_COST * c, i, j));
                }
            }
        }
    }
    cands.sort_by(|p, q| p.0.total_cmp(&q.0).then(p.1.cmp(&q.1)).then(p.2.cmp(&q.2)));
    let (mut ua, mut ub) = (vec![false; pa.len()], vec![false; pb.len()]);
    let mut out = Vec::new();
    for (c, i, j) in cands {
        if c > MAX_COST || ua[i] || ub[j] {
            continue;
        }
        ua[i] = true;
        ub[j] = true;
        out.push((i, j));
    }
    out
}

/// Points in buckets for nearest-neighbour queries.
struct Near {
    side: usize,
    bw: usize,
    bh: usize,
    buckets: Vec<Vec<usize>>,
    pts: Vec<(usize, usize)>,
}

impl Near {
    const SIDE: usize = 8;

    fn new(w: usize, h: usize, pts: Vec<(usize, usize)>) -> Near {
        let side = Near::SIDE;
        let (bw, bh) = (w / side + 1, h / side + 1);
        let mut buckets = vec![Vec::new(); bw * bh];
        for (i, &(y, x)) in pts.iter().enumerate() {
            buckets[(y / side) * bw + x / side].push(i);
        }
        Near {
            side,
            bw,
            bh,
            buckets,
            pts,
        }
    }

    /// The `k` nearest points to `(y, x)` as (distance squared, index), nearest first; ties by index.
    fn nearest(&self, y: usize, x: usize, k: usize) -> Vec<(f64, usize)> {
        let (cy, cx) = ((y / self.side) as i64, (x / self.side) as i64);
        let mut found: Vec<(f64, usize)> = Vec::new();
        let rings = self.bw.max(self.bh) as i64;
        for r in 0..=rings {
            for gy in cy - r..=cy + r {
                for gx in cx - r..=cx + r {
                    let ring = (gy - cy).abs().max((gx - cx).abs()) == r;
                    if !ring || gy < 0 || gx < 0 || gy >= self.bh as i64 || gx >= self.bw as i64 {
                        continue;
                    }
                    for &i in &self.buckets[gy as usize * self.bw + gx as usize] {
                        let (py, px) = self.pts[i];
                        let (dy, dx) = (py as f64 - y as f64, px as f64 - x as f64);
                        found.push((dy * dy + dx * dx, i));
                    }
                }
            }
            found.sort_by(|p, q| p.0.total_cmp(&q.0).then(p.1.cmp(&q.1)));
            // every point not met yet is farther than r buckets' width
            let reach = (r * self.side as i64) as f64;
            if found.len() >= k && found.get(k - 1).is_some_and(|f| f.0 <= reach * reach) {
                break;
            }
        }
        found.truncate(k);
        found
    }
}

/// Every cell's move: the inverse-distance-weighted mean of the `K` nearest paired pixels' moves.
fn field(w: usize, h: usize, at: Vec<(usize, usize)>, moves: &[[f64; 2]]) -> Vec<[f64; 2]> {
    if at.is_empty() {
        return vec![[0.0; 2]; w * h];
    }
    let near = Near::new(w, h, at);
    (0..w * h)
        .map(|c| {
            let (mut sum, mut wt) = ([0.0; 2], 0.0);
            for (d2, i) in near.nearest(c / w, c % w, K) {
                let k = 1.0 / (d2 + 1.0);
                sum[0] += k * moves[i][0];
                sum[1] += k * moves[i][1];
                wt += k;
            }
            [sum[0] / wt, sum[1] / wt]
        })
        .collect()
}

/// For the unpaired cells of one frame, their distance to the other frame's nearest pixel over the largest such
/// distance (0..0.999), in the order given.
fn order(w: usize, h: usize, unpaired: &[(usize, usize)], other: Vec<(usize, usize)>) -> Vec<f64> {
    if other.is_empty() {
        return vec![0.0; unpaired.len()];
    }
    let near = Near::new(w, h, other);
    let d: Vec<f64> = unpaired
        .iter()
        .map(|&(y, x)| {
            near.nearest(y, x, 1)
                .first()
                .map_or(0.0, |f| libm::sqrt(f.0))
        })
        .collect();
    let max = d.iter().copied().fold(1e-9, f64::max);
    d.iter().map(|v| v / max * 0.999).collect()
}

/// A hash of a cell, 0..1.
fn hash(i: usize) -> f64 {
    let mut v = (i as u64)
        .wrapping_add(0x9E37_79B9_7F4A_7C15)
        .wrapping_mul(0xBF58_476D_1CE4_E5B9);
    v ^= v >> 31;
    v = v.wrapping_mul(0x94D0_49BB_1331_11EB);
    v ^= v >> 29;
    (v >> 11) as f64 / (1u64 << 53) as f64
}

/// Smooth noise over the cells, scaled to the [`SWAP`] times: a hash per cell blurred by a Gaussian.
fn swap_times(w: usize, h: usize) -> Vec<f64> {
    let r = libm::ceil(3.0 * NOISE_SIGMA) as i64;
    let kernel: Vec<f64> = (-r..=r)
        .map(|d| libm::exp(-((d * d) as f64) / (2.0 * NOISE_SIGMA * NOISE_SIGMA)))
        .collect();
    let blur = |src: &[f64], along_x: bool| -> Vec<f64> {
        (0..w * h)
            .map(|c| {
                let (y, x) = ((c / w) as i64, (c % w) as i64);
                let (mut s, mut n) = (0.0, 0.0);
                for (k, d) in kernel.iter().zip(-r..=r) {
                    let (yy, xx) = if along_x {
                        (y, (x + d).clamp(0, w as i64 - 1))
                    } else {
                        ((y + d).clamp(0, h as i64 - 1), x)
                    };
                    s += k * src[yy as usize * w + xx as usize];
                    n += k;
                }
                s / n
            })
            .collect()
    };
    let noise: Vec<f64> = (0..w * h).map(hash).collect();
    let smooth = blur(&blur(&noise, true), false);
    let (lo, hi) = smooth
        .iter()
        .fold((f64::MAX, f64::MIN), |(lo, hi), &v| (lo.min(v), hi.max(v)));
    let span = (hi - lo).max(1e-9);
    smooth
        .iter()
        .map(|v| SWAP.0 + (SWAP.1 - SWAP.0) * (v - lo) / span)
        .collect()
}

/// How the pixels of frame `a` move into frame `b`, both `w` by `h`.
pub fn motion(w: usize, h: usize, a: &[Px], b: &[Px]) -> Motion {
    let (la, lb): (Vec<f64>, Vec<f64>) = (
        a.iter().map(|p| luma(*p)).collect(),
        b.iter().map(|p| luma(*p)).collect(),
    );
    let flow = coarse(w, h, &la, &lb);
    let (pa, pb) = (lit(w, a), lit(w, b));
    let pairs = pair(w, a, b, &pa, &pb, &flow);
    let moves: Vec<[f64; 2]> = pairs
        .iter()
        .map(|&(i, j)| {
            [
                pb[j].0 as f64 - pa[i].0 as f64,
                pb[j].1 as f64 - pa[i].1 as f64,
            ]
        })
        .collect();
    let forth = field(w, h, pairs.iter().map(|&(i, _)| pa[i]).collect(), &moves);
    let back_moves: Vec<[f64; 2]> = moves.iter().map(|m| [-m[0], -m[1]]).collect();
    let back = field(
        w,
        h,
        pairs.iter().map(|&(_, j)| pb[j]).collect(),
        &back_moves,
    );
    let (mut ma, mut mb) = (vec![false; w * h], vec![false; w * h]);
    for &(i, j) in &pairs {
        ma[pa[i].0 * w + pa[i].1] = true;
        mb[pb[j].0 * w + pb[j].1] = true;
    }
    let (mut gone, mut shows) = (vec![2.0; w * h], vec![-1.0; w * h]);
    let ua: Vec<(usize, usize)> = pa
        .iter()
        .copied()
        .filter(|&(y, x)| !ma[y * w + x])
        .collect();
    let ub: Vec<(usize, usize)> = pb
        .iter()
        .copied()
        .filter(|&(y, x)| !mb[y * w + x])
        .collect();
    for (&(y, x), o) in ua.iter().zip(order(w, h, &ua, pb.clone())) {
        gone[y * w + x] = 1.0 - o;
    }
    for (&(y, x), o) in ub.iter().zip(order(w, h, &ub, pa.clone())) {
        shows[y * w + x] = o;
    }
    Motion {
        w,
        h,
        pairs: pairs.len(),
        forth,
        back,
        gone,
        shows,
        swap: swap_times(w, h),
    }
}

/// The in-between of `a` and `b` at time `t` (0..1), by their [`Motion`].
pub fn between(a: &[Px], b: &[Px], m: &Motion, t: f64) -> Vec<Px> {
    let (w, h) = (m.w, m.h);
    let cell = |q: [f64; 2]| {
        let y = (libm::rint(q[0]) as i64).clamp(0, h as i64 - 1) as usize;
        let x = (libm::rint(q[1]) as i64).clamp(0, w as i64 - 1) as usize;
        y * w + x
    };
    // follows `field` scaled by `k` back from cell `c` to its source
    let source = |c: usize, field: &[[f64; 2]], k: f64| {
        let p = [(c / w) as f64, (c % w) as f64];
        let mut s = p;
        for _ in 0..STEPS {
            let v = field[cell(s)];
            s = [p[0] - k * v[0], p[1] - k * v[1]];
        }
        cell(s)
    };
    (0..w * h)
        .map(|c| {
            let (s, e) = (source(c, &m.forth, t), source(c, &m.back, 1.0 - t));
            let (pa, pb) = (a[s], b[e]);
            let a_on = kind(pa) > 0 && t < m.gone[s];
            let b_on = kind(pb) > 0 && t >= m.shows[e];
            match (a_on, b_on) {
                (false, false) => Px::Empty,
                (true, false) => pa,
                (false, true) => pb,
                (true, true) => {
                    let (ca, cb) = (rgb(pa), rgb(pb));
                    let alike =
                        kind(pa) == kind(pb) && (0..3).all(|k| (ca[k] - cb[k]).abs() <= ALIKE);
                    let take_b = t >= m.swap[s];
                    if !alike {
                        return if take_b { pb } else { pa };
                    }
                    let mix = |k: usize| {
                        libm::rint(ca[k] * (1.0 - t) + cb[k] * t).clamp(0.0, 255.0) as u8
                    };
                    let rgb = [mix(0), mix(1), mix(2)];
                    match if take_b { pb } else { pa } {
                        Px::Colour { ink, .. } => Px::Colour { rgb, ink },
                        Px::Light { ink, .. } => Px::Light { rgb, ink },
                        Px::Empty => Px::Empty,
                    }
                }
            }
        })
        .collect()
}

/// `n` in-betweens between each two frames next to each other in a row of the matrix (none across rows, none
/// between parts), named by [`name`], in order.
pub fn tweens(m: &Matrix, n: usize) -> Vec<Frame> {
    let (w, h) = (m.w as usize, m.h as usize);
    let mut out = Vec::new();
    for pair in m.frames.windows(2) {
        let [a, b] = pair else { continue };
        if row(&a.name) != row(&b.name) || n == 0 || crate::spec::is_part(&a.name) {
            continue;
        }
        let mo = motion(w, h, &a.px, &b.px);
        for k in 1..=n {
            let t = k as f64 / (n + 1) as f64;
            out.push(Frame {
                name: name(&a.name, &b.name, k),
                px: between(&a.px, &b.px, &mo, t),
                pivots: Vec::new(),
            });
        }
    }
    out
}
