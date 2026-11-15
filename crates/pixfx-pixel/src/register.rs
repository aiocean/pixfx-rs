//! Frames of one drawing lined up on one origin. An image model asked to draw a subject "the same size and in the
//! same place in every frame" still draws it a few pixels off in some frames, often a whole row of a sheet at once.
//! Here each frame gets the whole-pixel shift that puts it back:
//!
//! 1. Edges: per pixel, the brightness gradient's length (central differences), brightness being luma times alpha.
//!    Edges, not brightness: a frame whose content changes (a bell dissolving from the bottom) keeps its outline
//!    where it was, while its brightness shifts its weight.
//! 2. Reference: per pixel, the median of every frame's edges, each frame shifted by its shift so far (none at
//!    first). The median keeps the edges most frames share, the subject's outline, and drops what one row adds (a
//!    spark, a beam).
//! 3. Shift: per frame, the one within `radius` pixels whose shifted edges correlate best with the reference
//!    (normalised cross-correlation). It is taken only when the match is clear: inside the window, not on its
//!    edge (a best match on the edge means the content changed rather than moved: petals flying apart), a
//!    correlation of at least [`MIN_SCORE`], better than no shift by more than [`MIN_GAIN`]. A frame with few
//!    pixels stays where it is. A true shift is up to `radius - 1` pixels.
//! 4. Steps 2 and 3 run [`ROUNDS`] times.
//!
//! A shift `(dx, dy)` moves the frame's content right and down: the frame's pixel `(x, y)` goes to `(x + dx,
//! y + dy)`. Deterministic: no shift is tried before the frame's own place, and ties keep the first met.

use crate::RgbaImage;

/// A shift is taken only when its correlation with the reference is at least this...
pub const MIN_SCORE: f64 = 0.3;
/// ...and better than leaving the frame where it is by more than this.
pub const MIN_GAIN: f64 = 0.02;
/// Rounds of reference and shifts.
const ROUNDS: usize = 3;
/// Luma weights.
const LUM: [f64; 3] = [0.299, 0.587, 0.114];

/// A frame's shift onto the shared origin, in whole pixels (right and down), the correlation of its edges with the
/// reference there, and at no shift.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Shift {
    pub dx: i32,
    pub dy: i32,
    pub score: f64,
    pub unshifted: f64,
}

/// Edge strength per pixel, row by row.
fn edges(img: &RgbaImage) -> Vec<f64> {
    let (w, h) = (img.w as usize, img.h as usize);
    let lum: Vec<f64> = img
        .rgba
        .as_chunks::<4>()
        .0
        .iter()
        .map(|p| {
            let a = f64::from(p[3]) / 255.0;
            (f64::from(p[0]) * LUM[0] + f64::from(p[1]) * LUM[1] + f64::from(p[2]) * LUM[2]) * a
        })
        .collect();
    let mut out = vec![0.0; w * h];
    for y in 1..h.saturating_sub(1) {
        for x in 1..w.saturating_sub(1) {
            let gx = lum[y * w + x + 1] - lum[y * w + x - 1];
            let gy = lum[(y + 1) * w + x] - lum[(y - 1) * w + x];
            out[y * w + x] = libm::sqrt(gx * gx + gy * gy);
        }
    }
    out
}

/// `v` (`w` by `h`) moved `dx` right and `dy` down; zero where nothing moved in.
fn shifted(v: &[f64], w: usize, h: usize, dx: i32, dy: i32) -> Vec<f64> {
    let mut out = vec![0.0; w * h];
    for y in 0..h {
        let sy = y as i64 - i64::from(dy);
        if sy < 0 || sy >= h as i64 {
            continue;
        }
        for x in 0..w {
            let sx = x as i64 - i64::from(dx);
            if sx >= 0 && sx < w as i64 {
                out[y * w + x] = v[sy as usize * w + sx as usize];
            }
        }
    }
    out
}

/// Normalised cross-correlation of two equal-length signals; 0 when either is flat.
fn ncc(a: &[f64], b: &[f64]) -> f64 {
    let n = a.len().max(1) as f64;
    let (ma, mb) = (a.iter().sum::<f64>() / n, b.iter().sum::<f64>() / n);
    let (mut ab, mut aa, mut bb) = (0.0, 0.0, 0.0);
    for (x, y) in a.iter().zip(b) {
        let (p, q) = (x - ma, y - mb);
        ab += p * q;
        aa += p * p;
        bb += q * q;
    }
    let d = libm::sqrt(aa * bb);
    if d == 0.0 { 0.0 } else { ab / d }
}

/// Per pixel, the median of the frames' values.
fn median(frames: &[Vec<f64>]) -> Vec<f64> {
    let len = frames.first().map_or(0, Vec::len);
    let mut col = Vec::with_capacity(frames.len());
    (0..len)
        .map(|i| {
            col.clear();
            col.extend(frames.iter().map(|f| f[i]));
            col.sort_by(f64::total_cmp);
            let n = col.len();
            if n % 2 == 1 { col[n / 2] } else { (col[n / 2 - 1] + col[n / 2]) / 2.0 }
        })
        .collect()
}

/// Each frame's shift onto the origin the frames share, searched within `radius` pixels. Fails when the frames
/// differ in size.
pub fn register(frames: &[RgbaImage], radius: u32) -> Result<Vec<Shift>, String> {
    let Some(first) = frames.first() else {
        return Ok(Vec::new());
    };
    let (w, h) = (first.w as usize, first.h as usize);
    if let Some(f) = frames.iter().find(|f| (f.w as usize, f.h as usize) != (w, h)) {
        return Err(format!("frames differ in size: {w}x{h} and {}x{}", f.w, f.h));
    }
    let e: Vec<Vec<f64>> = frames.iter().map(edges).collect();
    let r = radius as i32;
    // the frame's own place first, so a tie keeps it
    let mut tries = vec![(0, 0)];
    tries.extend((-r..=r).flat_map(|dy| (-r..=r).map(move |dx| (dx, dy))).filter(|&t| t != (0, 0)));
    let mut out: Vec<Shift> =
        e.iter().map(|_| Shift { dx: 0, dy: 0, score: 0.0, unshifted: 0.0 }).collect();
    for _ in 0..ROUNDS {
        let placed: Vec<Vec<f64>> = e.iter().zip(&out).map(|(f, s)| shifted(f, w, h, s.dx, s.dy)).collect();
        let reference = median(&placed);
        for (f, s) in e.iter().zip(out.iter_mut()) {
            let unshifted = ncc(f, &reference);
            let mut best = (0, 0, unshifted);
            for &(dx, dy) in &tries[1..] {
                let c = ncc(&shifted(f, w, h, dx, dy), &reference);
                if c > best.2 {
                    best = (dx, dy, c);
                }
            }
            // a best match on the window's edge is no peak: the content changed, it did not move
            let inside = best.0.abs() < r && best.1.abs() < r;
            let clear = inside && best.2 >= MIN_SCORE && best.2 - unshifted > MIN_GAIN;
            *s = if clear {
                Shift { dx: best.0, dy: best.1, score: best.2, unshifted }
            } else {
                Shift { dx: 0, dy: 0, score: unshifted, unshifted }
            };
        }
    }
    Ok(out)
}

/// `img` moved by `shift`: pixel `(x, y)` goes to `(x + dx, y + dy)`, transparent where nothing moved in.
pub fn apply(img: &RgbaImage, shift: Shift) -> RgbaImage {
    let mut out = RgbaImage::new(img.w, img.h);
    for y in 0..img.h {
        for x in 0..img.w {
            let (sx, sy) = (i64::from(x) - i64::from(shift.dx), i64::from(y) - i64::from(shift.dy));
            if sx >= 0
                && sy >= 0
                && let Some(p) = img.get(sx as u32, sy as u32)
            {
                out.set(x, y, p);
            }
        }
    }
    out
}
