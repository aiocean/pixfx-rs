//! Grid recovery: a picture drawn big (an image model's "pixels" are many real pixels wide, rarely a whole number,
//! often blurred at the edges) back to one pixel per drawn cell.
//!
//! 1. Edge energy per column and row: forward OKLab differences, a change of alpha counted as a full edge.
//! 2. The period per axis by autocorrelation of that profile, refined to a fraction through its harmonics.
//! 3. Candidates: the period, its half and third, and 1 (no grid).
//! 4. Grid lines per candidate by dynamic programming over cell widths; the first and last cell may be partial.
//! 5. Each candidate collapsed and repainted; the one that rebuilds the picture best for its cell count wins.
//!
//! Deterministic: no RNG, sums in index order, every tie broken by a total order.

use std::collections::HashMap;

use crate::oklab::{Lab, delta_e, lab};
use crate::{OPAQUE, Rgba, RgbaImage};

/// Cut positions in source pixels: first 0, last the width (height); cell `i` spans `cols[i]..cols[i + 1]`.
#[derive(Clone, Debug, PartialEq)]
pub struct Grid {
    pub cols: Vec<u32>,
    pub rows: Vec<u32>,
}

/// A recovered picture: its grid, its pitch (source pixels per cell), the picture one pixel per cell, and how sure
/// each cell is (row by row, 0..1) with their mean.
#[derive(Clone, Debug, PartialEq)]
pub struct Recovered {
    pub grid: Grid,
    pub pitch: f64,
    pub picture: RgbaImage,
    pub confidence: Vec<f64>,
    pub total: f64,
}

/// Voters within this OKLab distance of a cell's colour agree with it.
const AGREE: f64 = 0.04;
/// How hard a cell width is pulled toward the pitch, in units of the mean edge energy.
const SMOOTH: f64 = 2.0;
/// What a cell costs when a grid is weighed against none, in mean OKLab error per pixel it stands for: keeping
/// every pixel costs this much, a grid of pitch `p` this over `p²` plus its repaint error.
const CELL_COST: f64 = 0.1;
/// A finer candidate must bring the error inside the cells below this share of the coarser one's...
const FINER: f64 = 0.5;
/// ...and by more than this much OKLab error, which noise alone moves it.
const NOISE: f64 = 0.02;
/// The most distinct colours a cell's medoid is chosen among (the most frequent ones).
const MEDOID_CHOICES: usize = 32;

/// The picture's pixels in OKLab, each colour converted once, and which are painted.
struct Pixels {
    w: usize,
    h: usize,
    lab: Vec<Lab>,
    opaque: Vec<bool>,
}

impl Pixels {
    fn new(img: &RgbaImage) -> Self {
        let (w, h) = (img.w as usize, img.h as usize);
        let mut cache: HashMap<[u8; 3], Lab> = HashMap::new();
        let mut labs = Vec::with_capacity(w * h);
        let mut opaque = Vec::with_capacity(w * h);
        for p in img.rgba.as_chunks::<4>().0 {
            let key = [p[0], p[1], p[2]];
            let l = *cache
                .entry(key)
                .or_insert_with(|| lab(Rgba(p[0], p[1], p[2], 255)));
            labs.push(l);
            opaque.push(p[3] >= OPAQUE);
        }
        Pixels {
            w,
            h,
            lab: labs,
            opaque,
        }
    }

    /// How far apart two pixels look: a change of alpha is a full edge, two empty pixels are equal.
    fn edge(&self, i: usize, j: usize) -> f64 {
        match (self.opaque[i], self.opaque[j]) {
            (true, true) => delta_e(self.lab[i], self.lab[j]),
            (false, false) => 0.0,
            _ => 1.0,
        }
    }
}

/// Edge energy at each cut position `0..=w` (columns) and `0..=h` (rows): position `x` holds the summed difference
/// between pixel `x - 1` and pixel `x`; the borders hold 0.
fn profiles(px: &Pixels) -> (Vec<f64>, Vec<f64>) {
    let (w, h) = (px.w, px.h);
    let mut cols = vec![0.0; w + 1];
    let mut rows = vec![0.0; h + 1];
    for (y, row) in rows.iter_mut().enumerate().take(h) {
        for (x, col) in cols.iter_mut().enumerate().take(w) {
            let i = y * w + x;
            if x > 0 {
                *col += px.edge(i - 1, i);
            }
            if y > 0 {
                *row += px.edge(i - w, i);
            }
        }
    }
    (cols, rows)
}

/// The period of an edge profile, to a fraction of a pixel; `None` when it has none.
fn period(profile: &[f64]) -> Option<f64> {
    // The edges between pixels only: the border positions hold no measurement.
    let e = profile.get(1..profile.len().saturating_sub(1))?;
    let n = e.len();
    let last = n / 3;
    if last < 3 {
        return None;
    }
    let mean = e.iter().sum::<f64>() / n as f64;
    let d: Vec<f64> = e.iter().map(|v| v - mean).collect();
    let top = n / 2;
    let ac: Vec<f64> = (0..=top)
        .map(|lag| (0..n - lag).map(|i| d[i] * d[i + lag]).sum())
        .collect();
    let zero = ac[0];
    if zero <= 0.0 {
        return None;
    }
    let first =
        (2..last).find(|&l| ac[l] > ac[l - 1] && ac[l] >= ac[l + 1] && ac[l] > 0.2 * zero)?;
    // Each harmonic's peak near k times the first, then the least-squares slope through them.
    // Each next harmonic is looked for near k times the estimate so far, within less than half a period.
    let radius = ((first - 1) / 2).min(2);
    let (mut num, mut den) = (0.0, 0.0);
    for k in 1..=5u32 {
        let p = if den > 0.0 { num / den } else { first as f64 };
        let k = f64::from(k);
        let centre = libm::floor(k * p + 0.5) as usize;
        let (lo, hi) = (centre.saturating_sub(radius).max(1), centre + radius);
        if hi > top {
            break;
        }
        let mut best = lo;
        for l in lo..=hi {
            if ac[l] > ac[best] {
                best = l;
            }
        }
        num += k * best as f64;
        den += k * k;
    }
    (den > 0.0).then(|| num / den)
}

/// Cut positions `0..=n` that best follow `profile` with cells about `p` wide.
fn lines(profile: &[f64], p: f64) -> Vec<u32> {
    let n = profile.len() - 1;
    if p <= 1.0 {
        return (0..=n as u32).collect();
    }
    let inner = profile.get(1..n).unwrap_or(&[]);
    let mean = if inner.is_empty() {
        0.0
    } else {
        inner.iter().sum::<f64>() / inner.len() as f64
    };
    // The noise floor: the median edge. A cut gains only what stands above it, so noise inside a flat run of cells
    // never buys an extra cut there.
    let mut sorted = inner.to_vec();
    sorted.sort_by(f64::total_cmp);
    let floor = sorted.get(sorted.len() / 2).copied().unwrap_or(0.0);
    let lo = (libm::floor(p * 0.65) as usize).max(1);
    let hi = (libm::ceil(p * 1.35) as usize).max(lo);
    let cost = |w: usize, edge: bool| {
        let dev = (w as f64 - p) / p;
        // A first or last cell may be cut short by the picture's border; only a wide one, or a sliver narrower
        // than any inner cell (which only a real edge pays for), pays.
        if edge && dev <= 0.0 && w >= lo {
            0.0
        } else {
            SMOOTH * mean * dev * dev
        }
    };
    let mut best = vec![f64::NEG_INFINITY; n + 1];
    let mut from = vec![0usize; n + 1];
    best[0] = 0.0;
    for j in 1..=n {
        let reward = if j < n { profile[j] - floor } else { 0.0 };
        // Earlier cuts first, so on a tie the earlier one stays.
        for i in j.saturating_sub(hi)..j {
            let w = j - i;
            let edge = i == 0 || j == n;
            if !edge && w < lo {
                continue;
            }
            if best[i] == f64::NEG_INFINITY {
                continue;
            }
            let s = best[i] + reward - cost(w, edge);
            if s > best[j] {
                best[j] = s;
                from[j] = i;
            }
        }
    }
    let mut cuts = vec![n as u32];
    let mut j = n;
    while j > 0 {
        j = from[j];
        cuts.push(j as u32);
    }
    cuts.reverse();
    cuts
}

/// The span of a cell's middle half: at least one pixel.
fn middle(a: u32, b: u32) -> (u32, u32) {
    let w = b - a;
    let m = (w / 2).max(1);
    let start = a + (w - m) / 2;
    (start, start + m)
}

/// The colour a cell's voters choose, and the share of them that agree with it.
fn vote(voters: &mut Vec<(u32, usize)>, px: &Pixels) -> ([u8; 4], f64) {
    let total = voters.len();
    if total == 0 {
        return ([0; 4], 0.0);
    }
    let painted: Vec<(u32, usize)> = voters
        .iter()
        .copied()
        .filter(|&(_, i)| px.opaque[i])
        .collect();
    if painted.len() * 2 <= total {
        return ([0; 4], (total - painted.len()) as f64 / total as f64);
    }
    // Distinct colours with their counts and one pixel each, by colour key.
    let mut keys = painted;
    keys.sort_unstable();
    let mut distinct: Vec<(u32, usize, usize)> = Vec::new();
    for (k, i) in keys {
        match distinct.last_mut() {
            Some((last, count, _)) if *last == k => *count += 1,
            _ => distinct.push((k, 1, i)),
        }
    }
    let opaque_n: usize = distinct.iter().map(|d| d.1).sum();
    let mut by_count = distinct.clone();
    by_count.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    let winner = match by_count.first() {
        Some(&(k, c, i)) if c * 2 > opaque_n => (k, i),
        _ => {
            let mut chosen = (u32::MAX, 0usize, f64::INFINITY);
            for &(k, _, i) in by_count.iter().take(MEDOID_CHOICES) {
                let cost: f64 = distinct
                    .iter()
                    .map(|&(_, c, j)| c as f64 * delta_e(px.lab[i], px.lab[j]))
                    .sum();
                if cost < chosen.2 || (cost == chosen.2 && k < chosen.0) {
                    chosen = (k, i, cost);
                }
            }
            (chosen.0, chosen.1)
        }
    };
    let agree: usize = distinct
        .iter()
        .filter(|&&(_, _, j)| delta_e(px.lab[winner.1], px.lab[j]) <= AGREE)
        .map(|d| d.1)
        .sum();
    voters.clear();
    (winner.0.to_be_bytes(), agree as f64 / total as f64)
}

/// One pixel per cell of `grid`: each cell's middle half votes, the exact colour with more than half the painted
/// voters wins, else their OKLab medoid; empty when at most half are painted. With it, each cell's confidence (the
/// share of voters within ΔE 0.04 of its colour), row by row.
pub fn collapse(img: &RgbaImage, grid: &Grid) -> (RgbaImage, Vec<f64>) {
    collapse_pixels(img, &Pixels::new(img), grid)
}

fn collapse_pixels(img: &RgbaImage, px: &Pixels, grid: &Grid) -> (RgbaImage, Vec<f64>) {
    let (cw, ch) = (
        grid.cols.len().saturating_sub(1),
        grid.rows.len().saturating_sub(1),
    );
    let mut out = RgbaImage::new(cw as u32, ch as u32);
    let mut confidence = Vec::with_capacity(cw * ch);
    let mut voters = Vec::new();
    for (cy, r) in grid.rows.windows(2).enumerate() {
        let (y0, y1) = middle(r[0], r[1]);
        for (cx, c) in grid.cols.windows(2).enumerate() {
            let (x0, x1) = middle(c[0], c[1]);
            voters.clear();
            for y in y0..y1 {
                for x in x0..x1 {
                    let i = y as usize * px.w + x as usize;
                    let key = img.get(x, y).map_or(0, u32::from_be_bytes);
                    voters.push((key, i));
                }
            }
            let (colour, sure) = vote(&mut voters, px);
            out.set(cx as u32, cy as u32, colour);
            confidence.push(sure);
        }
    }
    (out, confidence)
}

/// How well a candidate's collapsed cells repaint the picture: the mean OKLab error over every pixel, and over the
/// middle half of each cell (the pixels that voted).
struct Fit {
    whole: f64,
    middle: f64,
}

fn repaint_error(px: &Pixels, grid: &Grid, picture: &RgbaImage) -> Fit {
    let (mut whole, mut inner, mut voters) = (0.0, 0.0, 0usize);
    for (cy, r) in grid.rows.windows(2).enumerate() {
        let (my0, my1) = middle(r[0], r[1]);
        for (cx, c) in grid.cols.windows(2).enumerate() {
            let Some(p) = picture.get(cx as u32, cy as u32) else {
                continue;
            };
            let (mx0, mx1) = middle(c[0], c[1]);
            let painted = p[3] >= OPAQUE;
            let colour = lab(Rgba(p[0], p[1], p[2], 255));
            for y in r[0]..r[1] {
                for x in c[0]..c[1] {
                    let i = y as usize * px.w + x as usize;
                    let e = match (painted, px.opaque[i]) {
                        (true, true) => delta_e(colour, px.lab[i]),
                        (false, false) => 0.0,
                        _ => 1.0,
                    };
                    whole += e;
                    if (my0..my1).contains(&y) && (mx0..mx1).contains(&x) {
                        inner += e;
                        voters += 1;
                    }
                }
            }
        }
    }
    Fit {
        whole: whole / (px.w * px.h).max(1) as f64,
        middle: inner / voters.max(1) as f64,
    }
}

/// One candidate grid, collapsed.
struct Candidate {
    pitch: f64,
    grid: Grid,
    picture: RgbaImage,
    confidence: Vec<f64>,
    fit: Fit,
}

fn candidate(
    img: &RgbaImage,
    px: &Pixels,
    profiles: (&[f64], &[f64]),
    (x, y): (f64, f64),
) -> Candidate {
    let grid = Grid {
        cols: lines(profiles.0, x),
        rows: lines(profiles.1, y),
    };
    let (picture, confidence) = collapse_pixels(img, px, &grid);
    let fit = repaint_error(px, &grid, &picture);
    Candidate {
        pitch: (x + y) / 2.0,
        grid,
        picture,
        confidence,
        fit,
    }
}

/// The picture's pixel grid, found with no hint, and the picture one pixel per cell. A picture with no grid (a
/// sprite at its true size) comes back unchanged with pitch 1.
pub fn recover(img: &RgbaImage) -> Recovered {
    let px = Pixels::new(img);
    let (pc, pr) = profiles(&px);
    let (fx, fy) = (period(&pc), period(&pr));
    let (bx, by) = match (fx, fy) {
        (Some(x), Some(y)) => (Some(x), Some(y)),
        (Some(x), None) => (Some(x), Some(x)),
        (None, Some(y)) => (Some(y), Some(y)),
        (None, None) => (None, None),
    };
    // Candidate pitches per axis, then the pairs whose cells are close to square.
    let mut candidates: Vec<(f64, f64)> = Vec::new();
    if let (Some(bx), Some(by)) = (bx, by) {
        let axis = |b: f64| -> Vec<f64> {
            (1..=3u32)
                .map(|d| b / f64::from(d))
                .filter(|&p| p >= 1.5)
                .collect()
        };
        for &x in &axis(bx) {
            for &y in &axis(by) {
                if x.max(y) / x.min(y) <= 1.5 {
                    candidates.push((x, y));
                }
            }
        }
    }
    // Coarsest first: a finer grid replaces a coarser one only when it halves the error inside the cells, and by
    // more than noise (the period's half and third catch an autocorrelation that locked on two or three cells).
    candidates.sort_by(|a, b| {
        (b.0 + b.1)
            .total_cmp(&(a.0 + a.1))
            .then(b.0.total_cmp(&a.0))
    });
    let profiles = (pc.as_slice(), pr.as_slice());
    let mut grid: Option<Candidate> = None;
    for pair in candidates {
        let c = candidate(img, &px, profiles, pair);
        let finer = match &grid {
            None => true,
            Some(best) => {
                c.fit.middle < FINER * best.fit.middle && best.fit.middle - c.fit.middle > NOISE
            }
        };
        if finer {
            grid = Some(c);
        }
    }
    // Then the grid against none at all: its error over every pixel plus what its cells cost, against keeping
    // every pixel.
    let none = candidate(img, &px, profiles, (1.0, 1.0));
    let pixels = (px.w * px.h).max(1) as f64;
    let cost = |c: &Candidate| {
        c.fit.whole + CELL_COST * f64::from(c.picture.w) * f64::from(c.picture.h) / pixels
    };
    let chosen = match grid {
        Some(g) if cost(&g) <= cost(&none) => g,
        _ => none,
    };
    let total = if chosen.confidence.is_empty() {
        0.0
    } else {
        chosen.confidence.iter().sum::<f64>() / chosen.confidence.len() as f64
    };
    Recovered {
        grid: chosen.grid,
        pitch: chosen.pitch,
        picture: chosen.picture,
        confidence: chosen.confidence,
        total,
    }
}

/// `img` re-sampled cell by cell to `height` rows, its width in proportion: each target cell takes the colour most
/// of its source block has (empty pixels count as one colour; ties: the lowest RGBA key).
pub fn fit_height(img: &RgbaImage, height: u32) -> RgbaImage {
    if img.w == 0 || img.h == 0 || height == 0 {
        return RgbaImage::new(0, 0);
    }
    let (w, h) = (u64::from(img.w), u64::from(img.h));
    let width = ((w * u64::from(height) * 2 + h) / (h * 2)).max(1) as u32;
    let span = |t: u32, out: u32, len: u64| {
        let a = (u64::from(t) * len / u64::from(out)) as u32;
        let b = ((u64::from(t) + 1) * len / u64::from(out)) as u32;
        (a, b.max(a + 1))
    };
    let mut out = RgbaImage::new(width, height);
    let mut keys = Vec::new();
    for ty in 0..height {
        let (y0, y1) = span(ty, height, h);
        for tx in 0..width {
            let (x0, x1) = span(tx, width, w);
            keys.clear();
            for y in y0..y1 {
                for x in x0..x1 {
                    if let Some(p) = img.get(x, y) {
                        let p = if p[3] >= OPAQUE { p } else { [0; 4] };
                        keys.push(u32::from_be_bytes(p));
                    }
                }
            }
            keys.sort_unstable();
            let mut best = (0usize, 0u32);
            let mut i = 0;
            while i < keys.len() {
                let k = keys[i];
                let mut j = i;
                while j < keys.len() && keys[j] == k {
                    j += 1;
                }
                if j - i > best.0 {
                    best = (j - i, k);
                }
                i = j;
            }
            out.set(tx, ty, best.1.to_be_bytes());
        }
    }
    out
}
