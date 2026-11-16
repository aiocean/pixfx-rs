//! Grid recovery for the frames of one drawing: an image model's sheet of an effect or a character, cut into cells
//! of one size. [`crate::recover::recover`] solves each picture alone, and on a sheet that gives every frame its own
//! pixel count, and none at all on a frame with few pixels. Here the frames share what they have in common:
//!
//! 1. Edges. Along each row (column), the places where brightness steps by more than [`STEP`], each placed to a
//!    fraction of a pixel by a parabola through the step and its neighbours.
//! 2. Pitch, per axis, for the whole sheet: the gaps between neighbouring edges of every frame are whole numbers of
//!    model pixels, so the pitch is the period that lines the gaps up best (the mean of `cos(2π·gap/p)`). The dense
//!    frames carry the sparse ones. A period's half lines the gaps up as well, so the longest period within
//!    [`HARMONIC`] of the best is taken.
//! 3. Size: every frame gets the same number of model pixels per axis, `round(cell / pitch)`, so pixel `(x, y)` of
//!    one frame is the same place as `(x, y)` of the next.
//! 4. Cuts, per frame: dynamic programming over exactly that many cells, each between 0.65 and 1.35 pitches wide,
//!    rewarded for cutting where the edge energy stands above its median. A model's pixels are not all one width
//!    (2 and 3 sheet pixels mixed), and its grid drifts across the sheet, so each frame finds its own cuts.
//! 5. Strength, per frame and axis: how much more edge energy falls on the cuts than between them,
//!    `(on - off) / (on + off)`, 0 for no grid, 1 for a perfect one. Empty space counts on neither side.
//!
//! Brightness is luma of the colour times alpha: a glow fading into a transparent background is an edge where it
//! fades, as it is over black. Deterministic: sums in index order, ties to the first value met.

use crate::RgbaImage;
use crate::recover::Grid;

/// A brightness step (luma, 0..255) at least this big is an edge.
pub const STEP: f64 = 40.0;
/// A gap between two edges longer than this many sheet pixels says nothing about the pitch.
const LONGEST_GAP: f64 = 16.0;
/// Pitches tried: from this...
const PITCH_LO: f64 = 1.5;
/// ...to this...
const PITCH_HI: f64 = 8.0;
/// ...in steps of this.
const PITCH_STEP: f64 = 0.005;
/// A longer period whose score is at least this share of the best one is the pitch (the best may be its half).
pub const HARMONIC: f64 = 0.85;
/// A gap counts toward refining the pitch when it is within this share of a pitch of a whole number of them...
const NEAR: f64 = 0.35;
/// ...and that number is at most this.
const LONGEST_RUN: f64 = 4.0;
/// How hard a cell width is pulled toward the pitch, in units of the mean edge energy.
const SMOOTH: f64 = 2.0;
/// Luma weights.
const LUM: [f64; 3] = [0.299, 0.587, 0.114];

/// One frame's cuts and how strongly its edges follow them, across and down.
#[derive(Clone, Debug, PartialEq)]
pub struct FrameGrid {
    pub grid: Grid,
    pub strength: (f64, f64),
}

/// The pixel grid of a sheet's frames: the pitch (sheet pixels a model pixel) and how well the edges' gaps fit it
/// (the mean cosine, 0 for noise, 1 for gaps all whole pitches), the size every frame has in model pixels, and each
/// frame's cuts, in the order given.
#[derive(Clone, Debug, PartialEq)]
pub struct SheetGrid {
    pub pitch: (f64, f64),
    pub fit: (f64, f64),
    pub size: (u32, u32),
    pub frames: Vec<FrameGrid>,
}

/// Brightness per pixel, row by row: luma of colour times alpha.
fn brightness(img: &RgbaImage) -> Vec<f64> {
    img.rgba
        .as_chunks::<4>()
        .0
        .iter()
        .map(|p| {
            let a = f64::from(p[3]) / 255.0;
            (f64::from(p[0]) * LUM[0] + f64::from(p[1]) * LUM[1] + f64::from(p[2]) * LUM[2]) * a
        })
        .collect()
}

/// A picture's brightness read along one axis: `lines` lines of `len` values; across reads rows, down reads columns.
struct Lines<'a> {
    v: &'a [f64],
    w: usize,
    across: bool,
    lines: usize,
    len: usize,
}

impl<'a> Lines<'a> {
    fn new(v: &'a [f64], w: usize, h: usize, across: bool) -> Self {
        let (lines, len) = if across { (h, w) } else { (w, h) };
        Lines {
            v,
            w,
            across,
            lines,
            len,
        }
    }

    fn at(&self, line: usize, i: usize) -> f64 {
        if self.across {
            self.v[line * self.w + i]
        } else {
            self.v[i * self.w + line]
        }
    }

    /// The step between value `i - 1` and `i` of a line, for `i` in `1..len`.
    fn step(&self, line: usize, i: usize) -> f64 {
        (self.at(line, i) - self.at(line, i - 1)).abs()
    }

    /// Edge energy at each cut position `0..=len`: position `i` sums the steps between `i - 1` and `i`; the borders
    /// hold 0.
    fn profile(&self) -> Vec<f64> {
        let mut out = vec![0.0; self.len + 1];
        for line in 0..self.lines {
            for (i, e) in out.iter_mut().enumerate().take(self.len).skip(1) {
                *e += self.step(line, i);
            }
        }
        out
    }

    /// Gaps between neighbouring edges along every line, edges placed to a fraction of a pixel.
    fn gaps(&self, out: &mut Vec<f64>) {
        for line in 0..self.lines {
            let mut last: Option<f64> = None;
            // a step at position i (between i - 1 and i) is an edge when it peaks over its neighbours
            for i in 2..self.len.saturating_sub(1) {
                let (a, b, c) = (
                    self.step(line, i - 1),
                    self.step(line, i),
                    self.step(line, i + 1),
                );
                if b <= STEP || b < a || b <= c {
                    continue;
                }
                let den = a - 2.0 * b + c;
                let at = i as f64
                    + if den == 0.0 {
                        0.0
                    } else {
                        (a - c) / (2.0 * den)
                    };
                if let Some(prev) = last {
                    let g = at - prev;
                    if g < LONGEST_GAP {
                        out.push(g);
                    }
                }
                last = Some(at);
            }
        }
    }
}

/// The pitch the gaps fit best, and how well (the mean cosine); `None` with no gaps.
fn pitch_of(gaps: &[f64]) -> Option<(f64, f64)> {
    if gaps.is_empty() {
        return None;
    }
    let n = gaps.len() as f64;
    let steps = libm::rint((PITCH_HI - PITCH_LO) / PITCH_STEP) as usize;
    let score: Vec<(f64, f64)> = (0..=steps)
        .map(|k| {
            let p = PITCH_LO + k as f64 * PITCH_STEP;
            let s: f64 = gaps
                .iter()
                .map(|g| libm::cos(2.0 * core::f64::consts::PI * g / p))
                .sum();
            (p, s / n)
        })
        .collect();
    let peaks: Vec<(f64, f64)> = score
        .windows(3)
        .filter(|w| w[1].1 >= w[0].1 && w[1].1 > w[2].1)
        .map(|w| w[1])
        .collect();
    let best = peaks.iter().map(|p| p.1).fold(f64::NEG_INFINITY, f64::max);
    if best <= 0.0 {
        return None;
    }
    let (coarse, fit) = peaks.into_iter().rfind(|p| p.1 >= HARMONIC * best)?;
    Some((refine(gaps, coarse), fit))
}

/// The comb's peak sits a little off the true pitch when the gaps are whole sheet pixels (2s and 3s for 2.7), so
/// the pitch is refined by least squares: each gap within [`NEAR`] of a whole number `k` (1 to [`LONGEST_RUN`]) of
/// pitches counts as `k` pixels, and the pitch is `Σ k·gap / Σ k²`, a few rounds.
fn refine(gaps: &[f64], mut p: f64) -> f64 {
    for _ in 0..4 {
        let (mut num, mut den) = (0.0, 0.0);
        for &g in gaps {
            let k = libm::rint(g / p);
            if (1.0..=LONGEST_RUN).contains(&k) && (g / p - k).abs() <= NEAR {
                num += k * g;
                den += k * k;
            }
        }
        if den == 0.0 {
            break;
        }
        p = num / den;
    }
    p
}

/// Cut positions `0..=len` for exactly `n` cells about `p` wide that best follow `profile` (cut positions `0..=len`,
/// borders 0), and the score of the fit; `None` when `n` such cells cannot span `len`. The first and last cell may
/// be cut short by the border, to half a pitch.
fn cuts(profile: &[f64], n: usize, p: f64) -> Option<(Vec<u32>, f64)> {
    let len = profile.len() - 1;
    if n == 0 || len == 0 {
        return None;
    }
    let inner = &profile[1..len];
    let mean = if inner.is_empty() {
        0.0
    } else {
        inner.iter().sum::<f64>() / inner.len() as f64
    };
    let mut sorted = inner.to_vec();
    sorted.sort_by(f64::total_cmp);
    let floor = sorted.get(sorted.len() / 2).copied().unwrap_or(0.0);
    let lo = (libm::floor(p * 0.65) as usize).max(1);
    let hi = (libm::ceil(p * 1.35) as usize).max(lo);
    let edge_lo = (libm::ceil(p * 0.5) as usize).max(1);
    let cost = |w: usize, edge: bool| {
        let dev = (w as f64 - p) / p;
        if edge && dev <= 0.0 {
            0.0
        } else {
            SMOOTH * mean * dev * dev
        }
    };
    let stride = len + 1;
    let mut best = vec![f64::NEG_INFINITY; (n + 1) * stride];
    let mut from = vec![0usize; (n + 1) * stride];
    best[0] = 0.0;
    for k in 1..=n {
        for j in 1..=len {
            let reward = if j < len { profile[j] - floor } else { 0.0 };
            // narrow first: on a tie the earlier cut stays
            for w in (1..=hi.min(j)).rev() {
                let i = j - w;
                let edge = i == 0 || j == len;
                if w < if edge { edge_lo } else { lo } {
                    continue;
                }
                let prev = best[(k - 1) * stride + i];
                if prev == f64::NEG_INFINITY {
                    continue;
                }
                let s = prev + reward - cost(w, edge);
                if s > best[k * stride + j] {
                    best[k * stride + j] = s;
                    from[k * stride + j] = i;
                }
            }
        }
    }
    let score = best[n * stride + len];
    if score == f64::NEG_INFINITY {
        return None;
    }
    let mut out = vec![len as u32];
    let mut j = len;
    for k in (1..=n).rev() {
        j = from[k * stride + j];
        out.push(j as u32);
    }
    out.reverse();
    Some((out, score))
}

/// The cuts of every frame along one axis for the count, of `round(len / p)` and one either side, whose cuts fit
/// all frames best (the summed score); the count and the cuts per frame.
fn axis_cuts(profiles: &[Vec<f64>], p: f64) -> (usize, Vec<Vec<u32>>) {
    let len = profiles.first().map_or(0, |e| e.len() - 1);
    let r = (libm::rint(len as f64 / p) as usize).max(1);
    let mut chosen: Option<(f64, usize, Vec<Vec<u32>>)> = None;
    for n in r.saturating_sub(1).max(1)..=r + 1 {
        let fits: Option<Vec<(Vec<u32>, f64)>> = profiles.iter().map(|e| cuts(e, n, p)).collect();
        let Some(fits) = fits else { continue };
        let score: f64 = fits.iter().map(|f| f.1).sum();
        if chosen.as_ref().is_none_or(|c| score > c.0) {
            chosen = Some((score, n, fits.into_iter().map(|f| f.0).collect()));
        }
    }
    match chosen {
        Some((_, n, c)) => (n, c),
        // no count fits the width bounds: even cells
        None => (
            r,
            profiles
                .iter()
                .map(|_| (0..=r).map(|k| ((k * len) / r) as u32).collect())
                .collect(),
        ),
    }
}

/// How much more edge energy lies on the inner cuts than between them: `(on - off) / (on + off)`, means over the
/// positions on each side.
fn strength(profile: &[f64], cuts: &[u32]) -> f64 {
    let len = profile.len() - 1;
    let mut on_cut = vec![false; len + 1];
    for &c in cuts {
        on_cut[c as usize] = true;
    }
    let (mut on, mut on_n, mut off, mut off_n) = (0.0, 0usize, 0.0, 0usize);
    for i in 1..len {
        if on_cut[i] {
            on += profile[i];
            on_n += 1;
        } else {
            off += profile[i];
            off_n += 1;
        }
    }
    let (on, off) = (on / on_n.max(1) as f64, off / off_n.max(1) as f64);
    if on + off == 0.0 {
        0.0
    } else {
        (on - off) / (on + off)
    }
}

/// The shared pixel grid of `frames`, cells of one sheet, all the same size. Fails when they differ in size, or
/// when no axis has edges enough to show a pitch (an empty or blurred sheet). An axis with no pitch of its own
/// takes the other's.
pub fn recover_frames(frames: &[RgbaImage]) -> Result<SheetGrid, String> {
    let Some(first) = frames.first() else {
        return Err("no frames".to_string());
    };
    let (w, h) = (first.w as usize, first.h as usize);
    if let Some(f) = frames
        .iter()
        .find(|f| (f.w as usize, f.h as usize) != (w, h))
    {
        return Err(format!(
            "frames differ in size: {w}x{h} and {}x{}",
            f.w, f.h
        ));
    }
    let lum: Vec<Vec<f64>> = frames.iter().map(brightness).collect();
    let (mut gx, mut gy) = (Vec::new(), Vec::new());
    for v in &lum {
        Lines::new(v, w, h, true).gaps(&mut gx);
        Lines::new(v, w, h, false).gaps(&mut gy);
    }
    let ((px, fx), (py, fy)) = match (pitch_of(&gx), pitch_of(&gy)) {
        (Some(x), Some(y)) => (x, y),
        (Some(x), None) => (x, (x.0, 0.0)),
        (None, Some(y)) => ((y.0, 0.0), y),
        (None, None) => return Err("no edges show a pixel grid".to_string()),
    };
    let ex: Vec<Vec<f64>> = lum
        .iter()
        .map(|v| Lines::new(v, w, h, true).profile())
        .collect();
    let ey: Vec<Vec<f64>> = lum
        .iter()
        .map(|v| Lines::new(v, w, h, false).profile())
        .collect();
    let (nx, cols) = axis_cuts(&ex, px);
    let (ny, rows) = axis_cuts(&ey, py);
    let grids = cols
        .into_iter()
        .zip(rows)
        .enumerate()
        .map(|(i, (cols, rows))| FrameGrid {
            strength: (strength(&ex[i], &cols), strength(&ey[i], &rows)),
            grid: Grid { cols, rows },
        })
        .collect();
    Ok(SheetGrid {
        pitch: (px, py),
        fit: (fx, fy),
        size: (nx as u32, ny as u32),
        frames: grids,
    })
}

/// `img` blown up `scale` times inside `[x0, y0, x1, y1]` (source pixels), with `grid`'s cuts drawn in `colour` on
/// the left (top) edge of each cell, for checking cuts by eye.
pub fn grid_view(
    img: &RgbaImage,
    grid: &Grid,
    [x0, y0, x1, y1]: [u32; 4],
    scale: u32,
    colour: [u8; 4],
) -> RgbaImage {
    let (x1, y1) = (x1.min(img.w), y1.min(img.h));
    let (w, h) = (x1.saturating_sub(x0), y1.saturating_sub(y0));
    let s = scale.max(1);
    let mut out = RgbaImage::new(w * s, h * s);
    for y in 0..h * s {
        for x in 0..w * s {
            out.set(x, y, img.get(x0 + x / s, y0 + y / s).unwrap_or([0; 4]));
        }
    }
    for &c in grid.cols.iter().filter(|&&c| c >= x0 && c < x1) {
        for y in 0..h * s {
            out.set((c - x0) * s, y, colour);
        }
    }
    for &r in grid.rows.iter().filter(|&&r| r >= y0 && r < y1) {
        for x in 0..w * s {
            out.set(x, (r - y0) * s, colour);
        }
    }
    out
}

/// The `side`-pixel square of `img` with the most edge energy (brightness steps across and down), searched in
/// steps of a quarter side: where a [`grid_view`] shows the most cuts at work. The first such square on a tie.
pub fn busiest(img: &RgbaImage, side: u32) -> [u32; 4] {
    let (w, h) = (img.w as usize, img.h as usize);
    let s = (side as usize).min(w).min(h);
    if s == 0 {
        return [0, 0, 0, 0];
    }
    let v = brightness(img);
    let mut energy = vec![0.0; w * h];
    for y in 0..h {
        for x in 0..w {
            let i = y * w + x;
            let across = if x > 0 { (v[i] - v[i - 1]).abs() } else { 0.0 };
            let down = if y > 0 { (v[i] - v[i - w]).abs() } else { 0.0 };
            energy[i] = across + down;
        }
    }
    let step = (s / 4).max(1);
    let mut best = (f64::NEG_INFINITY, 0usize, 0usize);
    for y0 in (0..=h - s).step_by(step) {
        for x0 in (0..=w - s).step_by(step) {
            let e: f64 = (y0..y0 + s)
                .map(|y| energy[y * w + x0..y * w + x0 + s].iter().sum::<f64>())
                .sum();
            if e > best.0 {
                best = (e, x0, y0);
            }
        }
    }
    let (x0, y0) = (best.1 as u32, best.2 as u32);
    [x0, y0, x0 + s as u32, y0 + s as u32]
}
