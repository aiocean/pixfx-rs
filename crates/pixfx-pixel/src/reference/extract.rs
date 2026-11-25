//! The effect's pixels cut back out of a sheet the model drew on a transparent background.
//!
//! The model returns the effect with an alpha channel and no backdrop, so there is no background to estimate and no
//! character to remove (the prompt asks for none). In order:
//!
//! 1. Cells. The model redraws the page, so the template's coordinates do not hold, but it keeps the labels: the
//!    only opaque, neutral, near-black marks. Each label line gives a row; the label starts, evenly spaced, give the
//!    columns. A cell is the square under its label, as wide as the template's cell is within its column pitch.
//!    The model often keeps the template's cyan cell frames; near a cell's edge that cyan is taken out, as
//!    `crate::sheet::cut` takes out the marks it drew.
//! 2. Grid. The model draws its own pixels, not the template's: about 2 to 4 sheet pixels each, 2s and 3s mixed,
//!    and not square. [`crate::frames::recover_frames`] finds the pitch from every cell at once and gives
//!    every frame the same number of model pixels, each frame its own cuts. A sheet with no edges to read a grid
//!    from (blurred, or blocks too big) falls back to the template's sprite pixels, even cells.
//! 3. Pixels. One sample per model pixel: the median of the middle of its block (half its width and height, at
//!    least one sheet pixel), colour and alpha.
//! 4. Origin. The model draws the subject a few pixels off in some frames, often a whole row at once; each frame
//!    is moved by whole model pixels to line up with the others ([`crate::register`], on the edges of the
//!    cell over black), so pixel `(x, y)` is the same place in every frame.
//! 5. Layers, as pixfx shows a surface (colour drawn, then light added on top), over black. Colour: the drawn colour
//!    where the pixel is nearly opaque, exactly as the model drew it. Light: colour times alpha where it is
//!    see-through (the glow), which is what that pixel adds over black. Nothing is snapped to a palette and no
//!    light is taken out of a colour pixel, so hues and edges stay as drawn.
//! 6. Check: colour or light against the cell over black, as a mean error per channel.
//! 7. Diff: the pixels that change between neighbouring frames of a row.
//!
//! A spec with a `grid` (the template showed every sprite pixel) skips step 2: the model drew on the template's
//! grid, so a cell is `cell` squares across, each as tall as it is wide. The model may squash rows of cells (the
//! lotus came back with cells 0.81 to 0.94 as tall as wide), so a cell ends above the next label line and its squares
//! are counted up from its bottom, where the figure stands; rows it has no room for come back empty at the top.
//! Step 4 is skipped too: the grid holds every frame in place.
//! The parts row (`spec.parts`) is cut like any row, its cells named `parts-<part>`, and left out of steps 4 and 7:
//! its cells are different things, not frames of one.
//!
//! Halves round to even (`libm::rint`), as the first version of this step did (in Python), so the cells it found
//! on the shields are found again.

use crate::frames::{SheetGrid, recover_frames};
use crate::recover::Grid;
use crate::register::register;
use crate::sheet::Layout;

use super::Spec;
use crate::RgbaImage;

/// A row's name as the first part of its cells' names: lower case, spaces as dashes (`HIT RIGHT` is `hit-right`).
pub fn name_of(row: &str) -> String {
    row.split_whitespace()
        .collect::<Vec<_>>()
        .join("-")
        .to_lowercase()
}

/// Alpha (0..1) from which a pixel is drawn colour; below it, it is glow.
pub const SOLID: f32 = 0.8;
/// Light dimmer than this (0..255 luma) is nothing.
const EMPTY: f32 = 4.0;
/// Luma weights.
const LUM: [f32; 3] = [0.299, 0.587, 0.114];

/// One frame of the effect.
#[derive(Clone, Debug, PartialEq)]
pub struct Cell {
    /// `init-1`: the row's name in lower case and the frame from 1.
    pub name: String,
    /// Where the cell was found on the sheet: `[x0, y0, x1, y1]`, end exclusive.
    pub rect: [u32; 4],
    /// Sheet pixels a model pixel, across and down: the cell's size over the sheet's pixel count.
    pub pitch: (f64, f64),
    /// The cuts between the model's pixels, in sheet pixels from the cell's corner.
    pub grid: Grid,
    /// How strongly the cell's edges follow its cuts, across and down: 0 for no grid, 1 for a perfect one.
    pub strength: (f64, f64),
    /// Model pixels the frame was moved (right, down) to line up with the others; its layers are moved already.
    pub shift: (i32, i32),
    /// The colour layer, a sprite pixel a pixel; transparent where there is no colour.
    pub colour: RgbaImage,
    /// The light layer, opaque, black where there is none.
    pub light: RgbaImage,
    /// The model's alpha per sprite pixel, as grey.
    pub alpha: RgbaImage,
    /// The cell over black, a sprite pixel a pixel: what colour and light must add up to.
    pub target: RgbaImage,
    /// Colour and light put back together over black.
    pub rebuilt: RgbaImage,
    /// Pixels in the colour layer.
    pub solid: usize,
    /// Pixels with any light.
    pub lit: usize,
    /// Mean difference per channel (0..255) between `rebuilt` and `target`.
    pub mean_err: f64,
    /// Where a part's [`crate::reference::spec::PIVOT`] marks were, in sprite pixels from the cell's corner, one per mark (the
    /// middle of its pixels), top to bottom: a cell of several pieces (a ring of swords) has one per piece. The marks
    /// themselves are painted over from the pixels round them. Empty for none.
    pub pivots: Vec<(f64, f64)>,
}

/// What changes between two neighbouring frames of a row.
#[derive(Clone, Debug, PartialEq)]
pub struct Diff {
    /// `init-1-2`.
    pub name: String,
    /// Green where the second frame is brighter, red where it is darker, black where they match.
    pub image: RgbaImage,
    pub changed: usize,
}

/// A sheet cut into its frames.
#[derive(Clone, Debug, PartialEq)]
pub struct Extracted {
    /// Sheet pixels a model pixel for the whole sheet, across and down.
    pub pitch: (f64, f64),
    /// How well the gaps between edges fit that pitch (0 noise, 1 every gap a whole number of pitches).
    pub fit: (f64, f64),
    /// Model pixels per frame, across and down: the size of every layer.
    pub size: (u32, u32),
    /// Whether the model's grid was found; when not, the template's sprite pixels were used.
    pub found: bool,
    /// The cells in reading order.
    pub cells: Vec<Cell>,
    pub diffs: Vec<Diff>,
}

/// A float picture, channels interleaved.
struct Plane {
    w: usize,
    h: usize,
    ch: usize,
    v: Vec<f32>,
}

impl Plane {
    fn at(&self, x: usize, y: usize, c: usize) -> f32 {
        self.v[(y * self.w + x) * self.ch + c]
    }
}

fn lum(p: [f32; 3]) -> f32 {
    p[0] * LUM[0] + p[1] * LUM[1] + p[2] * LUM[2]
}

/// Runs of `true` at least `min_len` long; gaps of up to `join` `false`s inside a run do not end it.
fn runs(mask: &[bool], min_len: usize, join: usize) -> Vec<(usize, usize)> {
    let mut out = Vec::new();
    let (mut start, mut gap) = (None::<usize>, 0usize);
    for i in 0..mask.len() + join + 1 {
        if mask.get(i).copied().unwrap_or(false) {
            start.get_or_insert(i);
            gap = 0;
        } else if let Some(s) = start {
            gap += 1;
            if gap > join {
                let end = i + 1 - gap;
                if end - s >= min_len {
                    out.push((s, end));
                }
                start = None;
                gap = 0;
            }
        }
    }
    out
}

fn median(v: &mut [f32]) -> f32 {
    v.sort_by(f32::total_cmp);
    let n = v.len();
    if n == 0 {
        0.0
    } else if n % 2 == 1 {
        v[n / 2]
    } else {
        (v[n / 2 - 1] + v[n / 2]) / 2.0
    }
}

/// Near-black, neutral, opaque: the ink labels are written in.
fn ink_of(img: &Plane) -> Vec<bool> {
    let (w, h) = (img.w, img.h);
    (0..w * h)
        .map(|i| {
            let (x, y) = (i % w, i / w);
            let [r, g, b, a] = [
                img.at(x, y, 0),
                img.at(x, y, 1),
                img.at(x, y, 2),
                img.at(x, y, 3),
            ];
            let (hi, lo) = (r.max(g).max(b), r.min(g).min(b));
            a > 0.7 * 255.0 && hi < 70.0 && hi - lo < 20.0
        })
        .collect()
}

/// The cells' rectangles, a list per row, from the label lines: row `r` is the first label line below row `r - 1`
/// with `counts[r]` labels spaced evenly from the left edge.
fn find_cells(
    img: &Plane,
    layout: &Layout,
    counts: &[usize],
) -> Result<Vec<Vec<[usize; 4]>>, String> {
    let (w, h) = (img.w, img.h);
    let ink = ink_of(img);
    let row_ink: Vec<bool> = (0..h)
        .map(|y| ink[y * w..(y + 1) * w].iter().filter(|&&v| v).count() > 3)
        .collect();
    let cols = layout.cols as usize;
    // a label fits in the gap above its cell: the gap's share of the sheet's height, with some slack
    let band = h as f64 * f64::from(layout.gap)
        / f64::from(layout.rows * layout.cell_h + (layout.rows + 1) * layout.gap);
    let tall = ((band * 1.15) as usize).max(8);
    let short = (h / 192).max(4);
    // the `n` label starts of the line at rows y0..y1: one near the left edge, the rest a pitch apart
    let starts_of = |y0: usize, y1: usize, n: usize| -> Option<Vec<f64>> {
        let col_ink: Vec<bool> = (0..w).map(|x| (y0..y1).any(|y| ink[y * w + x])).collect();
        let xs: Vec<usize> = runs(&col_ink, 3, w / 50).into_iter().map(|r| r.0).collect();
        let tol = w as f64 * 0.025;
        let pitch_lo = w as f64 / cols as f64 * 0.75;
        let pitch_hi = w as f64 / cols as f64 * 1.05;
        for &s0 in xs.iter().filter(|&&x| (x as f64) < w as f64 * 0.1) {
            if n == 1 {
                return Some(vec![s0 as f64]);
            }
            for &s1 in xs.iter().filter(|&&x| x > s0) {
                let p = (s1 - s0) as f64;
                if p < pitch_lo || p > pitch_hi {
                    continue;
                }
                let mut found = vec![s0 as f64, s1 as f64];
                for k in 2..n {
                    let want = s0 as f64 + k as f64 * p;
                    match xs.iter().find(|&&x| (x as f64 - want).abs() <= tol) {
                        Some(&x) => found.push(x as f64),
                        None => break,
                    }
                }
                if found.len() == n {
                    return Some(found);
                }
            }
        }
        None
    };
    let candidates: Vec<(usize, usize)> = runs(&row_ink, short, 0)
        .into_iter()
        .filter(|r| r.1 - r.0 < tall)
        .collect();
    let mut lines: Vec<(usize, Vec<f64>)> = Vec::with_capacity(counts.len());
    let mut next = 0;
    for (r, &n) in counts.iter().enumerate() {
        let hit = candidates
            .iter()
            .enumerate()
            .skip(next)
            .find_map(|(i, c)| starts_of(c.0, c.1, n).map(|s| (i, c.1, s)));
        let Some((i, bottom, starts)) = hit else {
            let at: Vec<usize> = lines.iter().map(|l| l.0).collect();
            return Err(format!(
                "found the label lines of {r} of {} rows (ending at y {at:?}); no line of {n} labels below them: the \
model may have dropped or moved a label",
                counts.len()
            ));
        };
        lines.push((bottom, starts));
        next = i + 1;
    }
    let col_x: Vec<f64> = (0..cols)
        .map(|c| {
            median(
                &mut lines
                    .iter()
                    .filter_map(|l| l.1.get(c).map(|&x| x as f32))
                    .collect::<Vec<_>>(),
            ) as f64
        })
        .collect();
    let pitch = median(
        &mut col_x
            .windows(2)
            .map(|p| (p[1] - p[0]) as f32)
            .collect::<Vec<_>>(),
    ) as f64;
    let size = libm::rint(pitch * f64::from(layout.cell_w) / f64::from(layout.cell_w + layout.gap))
        as usize;
    let mut out = Vec::with_capacity(lines.len());
    for ((y1, _), &n) in lines.iter().zip(counts) {
        let top = y1 + libm::rint(size as f64 * 0.03) as usize;
        let row = col_x
            .iter()
            .take(n)
            .map(|&x| {
                let x0 = libm::rint(x) as usize;
                [x0, top, (x0 + size).min(w), (top + size).min(h)]
            })
            .collect();
        out.push(row);
    }
    Ok(out)
}

/// A row's run of ink at least this long is a piece of a line (a frame, a box), not of a letter.
const STROKE: usize = 24;

/// The cells of a sheet drawn on a template with a grid. The model keeps the template's columns (the lotus came
/// back scaled as a whole across, every column within a pixel of the template's), so a cell's left edge and width
/// are the template's, scaled to the sheet's width. It may squash rows by different amounts, so a row's cells start
/// under its label line and end above the next one (no taller than wide), leaving between the text and the cell
/// what the template leaves: half the gap's height the text does not use; the last row is as tall as the others. A label line is a run of rows holding
/// text: at least 6 ink pixels a label outside long runs, so the dark frames and label boxes models like to draw
/// are not text; lines a little apart (a label wrapped onto two lines) are one line.
fn grid_cells(
    img: &Plane,
    layout: &Layout,
    counts: &[usize],
) -> Result<Vec<Vec<[usize; 4]>>, String> {
    let (w, h) = (img.w, img.h);
    let ink = ink_of(img);
    let (tw, _) = crate::sheet::size(layout);
    let sx = w as f64 / f64::from(tw.max(1));
    let band = f64::from(layout.gap * layout.scale.max(1)) * sx;
    let widest = counts.iter().copied().max().unwrap_or(1);
    let text_rows: Vec<bool> = (0..h)
        .map(|y| {
            let row = &ink[y * w..(y + 1) * w];
            let all = row.iter().filter(|&&v| v).count();
            let lines: usize = runs(row, STROKE, 0).iter().map(|r| r.1 - r.0).sum();
            all - lines >= 6 * widest
        })
        .collect();
    let lines: Vec<(usize, usize)> = runs(&text_rows, 2, (band * 0.3) as usize)
        .into_iter()
        .filter(|r| (r.1 - r.0) as f64 <= band * 1.15)
        .collect();
    if lines.len() < counts.len() {
        return Err(format!(
            "found {} label lines (at y {:?}) for {} rows: the model may have dropped or moved a label",
            lines.len(),
            lines.iter().map(|l| l.0).collect::<Vec<_>>(),
            counts.len()
        ));
    }
    let size = (f64::from(layout.cell_w * layout.scale.max(1)) * sx) as usize;
    let pad = |l: (usize, usize)| ((band - (l.1 - l.0) as f64) / 2.0).max(0.0) as usize;
    let tops: Vec<usize> = (0..counts.len())
        .map(|r| lines[r].1 + pad(lines[r]))
        .collect();
    let bottoms: Vec<usize> = (0..counts.len())
        .filter_map(|r| {
            lines.get(r + 1).map(|&l| {
                l.0.saturating_sub(pad(l))
                    .min(tops[r] + size)
                    .max(tops[r] + 1)
            })
        })
        .collect();
    // the last row has no label line under it: it is as tall as the rows above (a sheet of one row, as tall as wide)
    let mut tall_as: Vec<f32> = bottoms
        .iter()
        .zip(&tops)
        .map(|(b, t)| (b - t) as f32)
        .collect();
    let last = if tall_as.is_empty() {
        size
    } else {
        median(&mut tall_as) as usize
    };
    Ok(counts
        .iter()
        .enumerate()
        .map(|(r, &n)| {
            let top = tops[r];
            let bottom = bottoms.get(r).copied().unwrap_or(top + last).min(h);
            (0..n as u32)
                .map(|c| {
                    let x0 = (f64::from(crate::sheet::cell_rect(layout, c, 0)[0]) * sx) as usize;
                    [x0, top, (x0 + size).min(w), bottom]
                })
                .collect()
        })
        .collect())
}

/// The span of a block's middle half: at least one pixel.
fn middle(a: usize, b: usize) -> (usize, usize) {
    let w = b - a;
    let m = (w / 2).max(1);
    let start = a + (w - m) / 2;
    (start, (start + m).min(b.max(start + 1)))
}

/// One value per model pixel of the cell at `rect`, its blocks cut by `grid`: the median of the block's middle.
fn sample(p: &Plane, rect: [usize; 4], grid: &Grid) -> Plane {
    let [x0, y0, _, _] = rect;
    let (gw, gh) = (
        grid.cols.len().saturating_sub(1),
        grid.rows.len().saturating_sub(1),
    );
    let mut v = Vec::with_capacity(gw * gh * p.ch);
    let mut buf = Vec::new();
    for r in grid.rows.windows(2) {
        let (ya, yb) = middle(r[0] as usize, r[1] as usize);
        for c in grid.cols.windows(2) {
            let (xa, xb) = middle(c[0] as usize, c[1] as usize);
            for ch in 0..p.ch {
                buf.clear();
                for y in ya..yb {
                    for x in xa..xb {
                        buf.push(p.at(x0 + x, y0 + y, ch));
                    }
                }
                v.push(median(&mut buf));
            }
        }
    }
    Plane {
        w: gw,
        h: gh,
        ch: p.ch,
        v,
    }
}

/// `p` moved `dx` right and `dy` down; zero (empty, black) where nothing moved in.
fn moved(p: &Plane, dx: i32, dy: i32) -> Plane {
    let mut v = vec![0.0; p.v.len()];
    for y in 0..p.h {
        for x in 0..p.w {
            let (sx, sy) = (x as i64 - i64::from(dx), y as i64 - i64::from(dy));
            if sx < 0 || sy < 0 || sx >= p.w as i64 || sy >= p.h as i64 {
                continue;
            }
            for c in 0..p.ch {
                v[(y * p.w + x) * p.ch + c] = p.at(sx as usize, sy as usize, c);
            }
        }
    }
    Plane {
        w: p.w,
        h: p.h,
        ch: p.ch,
        v,
    }
}

fn crop(img: &RgbaImage, [x0, y0, x1, y1]: [usize; 4]) -> RgbaImage {
    let mut out = RgbaImage::new((x1 - x0) as u32, (y1 - y0) as u32);
    for y in y0..y1 {
        for x in x0..x1 {
            out.set(
                (x - x0) as u32,
                (y - y0) as u32,
                img.get(x as u32, y as u32).unwrap_or([0; 4]),
            );
        }
    }
    out
}

/// The cells' shared grid and `true`, or the template's even cells (`gw` by `gh`) and `false` when no grid shows.
fn grid_of(cells: &[RgbaImage], gw: usize, gh: usize) -> (SheetGrid, bool) {
    match recover_frames(cells) {
        Ok(g) => (g, true),
        Err(_) => {
            let even = |len: u32, n: usize| {
                (0..=n)
                    .map(|k| (k as u64 * u64::from(len) / n as u64) as u32)
                    .collect()
            };
            let (w, h) = cells.first().map_or((0, 0), |c| (c.w, c.h));
            let g = SheetGrid {
                pitch: (f64::from(w) / gw as f64, f64::from(h) / gh as f64),
                fit: (0.0, 0.0),
                size: (gw as u32, gh as u32),
                frames: cells
                    .iter()
                    .map(|_| crate::frames::FrameGrid {
                        grid: Grid {
                            cols: even(w, gw),
                            rows: even(h, gh),
                        },
                        strength: (0.0, 0.0),
                    })
                    .collect(),
            };
            (g, false)
        }
    }
}

fn to_image(w: usize, h: usize, px: impl Fn(usize, usize) -> [f32; 4]) -> RgbaImage {
    let mut img = RgbaImage::new(w as u32, h as u32);
    for y in 0..h {
        for x in 0..w {
            let p = px(x, y);
            img.set(x as u32, y as u32, p.map(|c| c.clamp(0.0, 255.0) as u8));
        }
    }
    img
}

/// The template's grid over the cells at `rects`, each `cell` squares across: square pitch from the width, the
/// squares counted up from the bottom; a cell too short for all of them gets as many as fit, and with `taller` a
/// cell drawn taller than wide keeps its extra rows (the size is then the tallest cell's).
fn known_grid(rects: &[[usize; 4]], cell: usize, taller: bool) -> SheetGrid {
    let frames: Vec<crate::frames::FrameGrid> = rects
        .iter()
        .map(|&[x0, y0, x1, y1]| {
            let (w, h) = ((x1 - x0) as f64, (y1 - y0) as f64);
            let p = w / cell as f64;
            let most = if taller { cell * 2 } else { cell };
            let n = (libm::rint(h / p) as usize).clamp(1, most);
            let cols = (0..=cell)
                .map(|k| libm::rint(k as f64 * p) as u32)
                .collect();
            let rows = (0..=n)
                .map(|k| libm::rint((h - (n - k) as f64 * p).max(0.0)) as u32)
                .collect();
            crate::frames::FrameGrid {
                grid: Grid { cols, rows },
                strength: (1.0, 1.0),
            }
        })
        .collect();
    let pitch = rects
        .iter()
        .map(|r| (r[2] - r[0]) as f64 / cell as f64)
        .sum::<f64>()
        / rects.len().max(1) as f64;
    let tall = frames
        .iter()
        .map(|f| f.grid.rows.len().saturating_sub(1))
        .max()
        .unwrap_or(cell)
        .max(cell);
    SheetGrid {
        pitch: (pitch, pitch),
        fit: (1.0, 1.0),
        size: (cell as u32, tall as u32),
        frames,
    }
}

/// `p` padded with empty pixels to `w` x `h`: rows added on top, columns on both sides alike (the extra one on the
/// right).
fn padded(p: Plane, w: usize, h: usize) -> Plane {
    if p.h >= h && p.w >= w {
        return p;
    }
    let (w, h) = (w.max(p.w), h.max(p.h));
    let (left, top) = ((w - p.w) / 2, h - p.h);
    let mut v = vec![0.0; w * h * p.ch];
    for y in 0..p.h {
        let from = y * p.w * p.ch;
        let to = ((y + top) * w + left) * p.ch;
        v[to..to + p.w * p.ch].copy_from_slice(&p.v[from..from + p.w * p.ch]);
    }
    Plane { w, h, ch: p.ch, v }
}

/// The model's own pixel grid over cells of different sizes (a page it re-laid): its pitch found on every cell
/// trimmed to the smallest one (centred across, from the bottom up), then each cell cut on that square pitch,
/// centred across and counted up from its bottom, as many pixels as fit; the size is the largest cell's. None when
/// no pitch shows.
fn pitch_grid(crops: &[RgbaImage], rects: &[[usize; 4]]) -> Option<SheetGrid> {
    let mw = crops.iter().map(|c| c.w).min()?;
    let mh = crops.iter().map(|c| c.h).min()?;
    let trimmed: Vec<RgbaImage> = crops
        .iter()
        .map(|c| {
            let (x0, y0) = ((c.w - mw) / 2, c.h - mh);
            let mut t = RgbaImage::new(mw, mh);
            for y in 0..mh {
                for x in 0..mw {
                    t.set(x, y, c.get(x0 + x, y0 + y).unwrap_or([0; 4]));
                }
            }
            t
        })
        .collect();
    let g = recover_frames(&trimmed).ok()?;
    let p = f64::midpoint(g.pitch.0, g.pitch.1);
    if p < 1.0 {
        return None;
    }
    let frames: Vec<crate::frames::FrameGrid> = rects
        .iter()
        .map(|&[x0, y0, x1, y1]| {
            let (w, h) = ((x1 - x0) as f64, (y1 - y0) as f64);
            let (nc, nr) = (libm::floor(w / p) as usize, libm::floor(h / p) as usize);
            let left = (w - nc as f64 * p) / 2.0;
            let top = h - nr as f64 * p;
            crate::frames::FrameGrid {
                grid: Grid {
                    cols: (0..=nc)
                        .map(|k| libm::rint(left + k as f64 * p) as u32)
                        .collect(),
                    rows: (0..=nr)
                        .map(|k| libm::rint(top + k as f64 * p) as u32)
                        .collect(),
                },
                strength: g.fit,
            }
        })
        .collect();
    let size = frames.iter().fold((0u32, 0u32), |(w, h), f| {
        (
            w.max(f.grid.cols.len().saturating_sub(1) as u32),
            h.max(f.grid.rows.len().saturating_sub(1) as u32),
        )
    });
    Some(SheetGrid {
        pitch: (p, p),
        fit: g.fit,
        size,
        frames,
    })
}

/// A found pixel grid fitting its cuts less than this is doubtful.
const DOUBTFUL: f64 = 0.15;

/// Model pixels a frame may be moved, each way, to line up with the others.
const REGISTER_RADIUS: u32 = 8;

/// Sprite pixels from a cell's edge within which the model's copy of the template's cell frame may fall.
const FRAME_BAND: usize = 4;

/// Whether straight colour `p` is the template's cell-frame cyan (`crate::sheet::BORDER`) as a model redraws
/// it, its soft edge too: green and blue alike, bright, well above red.
fn frame_cyan(p: [u8; 4]) -> bool {
    let [r, g, b, a] = p.map(i32::from);
    a > 0 && g > 120 && g - r > 30 && b - r > 30 && (g - b).abs() < 40
}

/// The cells inside the cyan frames the model kept, row by row: a vertical frame line is a column of opaque frame
/// cyan at least a twelfth of the sheet tall, two of them with the same top and bottom are a cell's sides, and the
/// cells whose sides share a top make a row. None unless every row has the cells `counts` asks for: the model may
/// re-lay the page (cells stretched across, not square), but it keeps the frames.
fn framed_cells(img: &Plane, counts: &[usize]) -> Option<Vec<Vec<[usize; 4]>>> {
    let (w, h) = (img.w, img.h);
    let cyan = |x: usize, y: usize| {
        let p = [0, 1, 2, 3].map(|c| img.at(x, y, c) as u8);
        p[3] >= 160 && frame_cyan(p)
    };
    let tall = h / 12;
    // vertical runs per column; runs in neighbouring columns that overlap are one line, as long as its longest run
    // (a frame line breaks up here and there, and glow next to it leaves short pieces)
    let mut lines: Vec<(usize, usize, usize, usize)> = Vec::new(); // x0, x1 (end), y0, y1 (end)
    let overlap = |a0: usize, a1: usize, b0: usize, b1: usize| {
        a1.min(b1).saturating_sub(a0.max(b0)) * 2 > (a1 - a0).min(b1 - b0)
    };
    for x in 0..w {
        let col: Vec<bool> = (0..h).map(|y| cyan(x, y)).collect();
        for (y0, y1) in runs(&col, tall, 2) {
            match lines
                .iter_mut()
                .find(|l| l.1 + 2 >= x && overlap(l.2, l.3, y0, y1))
            {
                Some(l) => {
                    l.1 = x + 1;
                    if y1 - y0 > l.3 - l.2 {
                        (l.2, l.3) = (y0, y1);
                    }
                }
                None => lines.push((x, x + 1, y0, y1)),
            }
        }
    }
    lines.retain(|l| l.1 - l.0 <= 12);
    // rows: the longest lines first, each line joins the row it overlaps; then a row keeps its full-height lines
    lines.sort_by_key(|l| std::cmp::Reverse(l.3 - l.2));
    let mut rows: Vec<Vec<(usize, usize, usize, usize)>> = Vec::new();
    for l in lines {
        match rows
            .iter_mut()
            .find(|r| r.first().is_some_and(|f| overlap(f.2, f.3, l.2, l.3)))
        {
            Some(r) => r.push(l),
            None => rows.push(vec![l]),
        }
    }
    // each row's top and bottom come from its longest line; within them, a column at least 40% frame cyan is part of
    // a cell's side (a side broken by glow still counts), and neighbouring such columns are one side
    rows.sort_by_key(|r| r.first().map_or(0, |f| f.2));
    let rows: Vec<Vec<(usize, usize, usize, usize)>> = rows
        .iter()
        .filter_map(|r| r.first().map(|f| (f.2, f.3)))
        .map(|(y0, y1)| {
            let side: Vec<bool> = (0..w)
                .map(|x| (y0..y1).filter(|&y| cyan(x, y)).count() * 5 >= (y1 - y0) * 2)
                .collect();
            runs(&side, 1, 2)
                .into_iter()
                .filter(|(x0, x1)| x1 - x0 <= 12)
                .map(|(x0, x1)| (x0, x1, y0, y1))
                .collect()
        })
        .collect();
    if rows.len() != counts.len() {
        return None;
    }
    let cells: Vec<Vec<[usize; 4]>> = rows
        .iter()
        .zip(counts)
        .map(|(r, &n)| {
            let mut r = r.clone();
            r.sort_by_key(|l| l.0);
            paired(&r, n)
                .into_iter()
                .map(|(a, b)| {
                    let t = a.1 - a.0;
                    [a.1, a.2.max(b.2) + t, b.0, a.3.min(b.3).saturating_sub(t)]
                })
                .collect()
        })
        .collect();
    let fits = cells.len() == counts.len()
        && cells.iter().zip(counts).all(|(c, &n)| c.len() == n)
        && cells
            .iter()
            .flatten()
            .all(|r| r[2] > r[0] + 8 && r[3] > r[1] + 8);
    fits.then_some(cells)
}

/// A frame line: its columns `[x0, x1)` and its rows `[y0, y1)`.
type Line = (usize, usize, usize, usize);

/// The `n` cells of a row from its frame lines, sorted left to right: each cell is two lines one cell width apart,
/// every cell of a row as wide. Each distance between two lines is tried as that width; scanning left to right a line
/// pairs with the first line that far on (within 4%), and the width giving exactly `n` cells with the least error
/// wins. A stray line (an effect's own cyan stroke inside a cell) finds no partner and drops out. Empty when no width
/// gives `n` cells.
fn paired(lines: &[Line], n: usize) -> Vec<(Line, Line)> {
    let mut best: Option<(f64, Vec<(Line, Line)>)> = None;
    for (i, a) in lines.iter().enumerate() {
        for b in &lines[i + 1..] {
            let width = b.0.saturating_sub(a.1) as f64;
            if width < 16.0 {
                continue;
            }
            let tol = width * 0.04 + 2.0;
            let (mut pairs, mut err, mut k) = (Vec::new(), 0.0, 0);
            while k < lines.len() {
                let l = lines[k];
                let partner = lines[k + 1..]
                    .iter()
                    .enumerate()
                    .find(|(_, m)| (m.0.saturating_sub(l.1) as f64 - width).abs() <= tol);
                match partner {
                    Some((j, &m)) => {
                        err += (m.0.saturating_sub(l.1) as f64 - width).abs();
                        pairs.push((l, m));
                        k += j + 2;
                    }
                    None => k += 1,
                }
            }
            if pairs.len() == n && best.as_ref().is_none_or(|(e, _)| err < *e) {
                best = Some((err, pairs));
            }
        }
    }
    best.map(|b| b.1).unwrap_or_default()
}

/// `sheet` with the cell frames the model kept taken out: in each cell's outer [`FRAME_BAND`] sprite pixels, and on
/// any column or row of the cell that is frame cyan along three fifths of it or more, frame cyan becomes empty. An effect's own cyan that close to a cell's edge goes with it.
fn without_frames(
    sheet: &RgbaImage,
    rects: impl Iterator<Item = [usize; 4]>,
    grid: usize,
) -> RgbaImage {
    let mut out = sheet.clone();
    for [x0, y0, x1, y1] in rects {
        let band = ((x1 - x0) * FRAME_BAND).div_ceil(grid.max(1));
        let lo_x = x0.saturating_sub(band);
        let lo_y = y0.saturating_sub(band);
        for y in lo_y..(y1 + band).min(sheet.h as usize) {
            for x in lo_x..(x1 + band).min(sheet.w as usize) {
                let inner = x >= x0 + band && x + band < x1 && y >= y0 + band && y + band < y1;
                let (xu, yu) = (x as u32, y as u32);
                if !inner && sheet.get(xu, yu).is_some_and(frame_cyan) {
                    out.set(xu, yu, [0; 4]);
                }
            }
        }
        // a frame line the model drew further in: a column or row frame cyan along most of the cell
        let cyan = |x: usize, y: usize| sheet.get(x as u32, y as u32).is_some_and(frame_cyan);
        for x in x0..x1 {
            if (y0..y1).filter(|&y| cyan(x, y)).count() * 5 >= (y1 - y0) * 3 {
                (y0..y1)
                    .filter(|&y| cyan(x, y))
                    .for_each(|y| out.set(x as u32, y as u32, [0; 4]));
            }
        }
        for y in y0..y1 {
            if (x0..x1).filter(|&x| cyan(x, y)).count() * 5 >= (x1 - x0) * 3 {
                (x0..x1)
                    .filter(|&x| cyan(x, y))
                    .for_each(|x| out.set(x as u32, y as u32, [0; 4]));
            }
        }
    }
    out
}

/// Cut `sheet` (drawn by the model from `spec`'s template) into frames, the parts first when the spec has them.
/// Fails when the labels cannot be found: a label line missing, or one with the wrong number of labels.
pub fn extract(sheet: &RgbaImage, spec: &Spec) -> Result<Extracted, String> {
    let layout = &crate::reference::template::layout(spec);
    let (w, h) = (sheet.w as usize, sheet.h as usize);
    let sheet_rows = spec.sheet_rows();
    let counts: Vec<usize> = sheet_rows.iter().map(|r| r.1 as usize).collect();
    let part_rows = spec.part_rows().len();
    let raw = Plane {
        w,
        h,
        ch: 4,
        v: sheet.rgba.iter().map(|&b| f32::from(b)).collect(),
    };
    let framed = framed_cells(&raw, &counts);
    let rows = match (&framed, spec.grid) {
        (Some(cells), _) => cells.clone(),
        (None, true) => grid_cells(&raw, layout, &counts)?,
        (None, false) => find_cells(&raw, layout, &counts)?,
    };
    // the template's square grid holds only when the model kept the cells square; else its own pixels are found
    let square = rows.iter().flatten().all(|r| {
        let (cw, ch) = ((r[2] - r[0]) as f64, (r[3] - r[1]) as f64);
        ch <= cw * 1.06
    });
    let known = spec.grid && (framed.is_none() || square);
    let rects: Vec<(usize, usize, [usize; 4])> = rows
        .iter()
        .enumerate()
        .flat_map(|(r, row)| row.iter().enumerate().map(move |(f, &rect)| (r, f, rect)))
        .collect();
    let sheet = &without_frames(sheet, rects.iter().map(|t| t.2), layout.cell_w as usize);
    let rgba = Plane {
        w,
        h,
        ch: 4,
        v: sheet.rgba.iter().map(|&b| f32::from(b)).collect(),
    };
    let over = Plane {
        w,
        h,
        ch: 3,
        v: sheet
            .rgba
            .as_chunks::<4>()
            .0
            .iter()
            .flat_map(|p| {
                let a = f32::from(p[3]) / 255.0;
                [
                    f32::from(p[0]) * a,
                    f32::from(p[1]) * a,
                    f32::from(p[2]) * a,
                ]
            })
            .collect(),
    };
    let crops: Vec<RgbaImage> = rects.iter().map(|t| crop(sheet, t.2)).collect();
    let rect_list: Vec<[usize; 4]> = rects.iter().map(|t| t.2).collect();
    // the model's own pixels, when they show clearly; on soft glowing art they often do not, and the width it keeps
    // (the template's cells across) is the better guess
    let refit = if known || square {
        None
    } else {
        Some(
            pitch_grid(&crops, &rect_list)
                .filter(|g| g.fit.0.min(g.fit.1) >= DOUBTFUL)
                .unwrap_or_else(|| known_grid(&rect_list, spec.cell as usize, true)),
        )
    };
    let (sg, found) = if let Some(g) = refit {
        (g, true)
    } else if known {
        (
            known_grid(
                &rects.iter().map(|t| t.2).collect::<Vec<_>>(),
                spec.cell as usize,
                false,
            ),
            true,
        )
    } else {
        grid_of(&crops, layout.cell_w as usize, layout.cell_h as usize)
    };
    let (gw, gh) = (sg.size.0 as usize, sg.size.1 as usize);
    let sampled: Vec<(Plane, Plane)> = rects
        .iter()
        .zip(&sg.frames)
        .map(|(t, fg)| {
            (
                padded(sample(&rgba, t.2, &fg.grid), gw, gh),
                padded(sample(&over, t.2, &fg.grid), gw, gh),
            )
        })
        .collect();
    // the parts are lined up with nothing: only the moments are frames of one effect
    let is_part = |r: usize| r < part_rows;
    let seen: Vec<RgbaImage> = sampled
        .iter()
        .zip(&rects)
        .filter(|(_, t)| !is_part(t.0))
        .map(|((_, tg), _)| {
            to_image(gw, gh, |x, y| {
                [tg.at(x, y, 0), tg.at(x, y, 1), tg.at(x, y, 2), 255.0]
            })
        })
        .collect();
    // on a grid sheet the grid already holds every frame in place
    let found_shifts = if spec.grid {
        vec![(0, 0); seen.len()]
    } else {
        register(&seen, REGISTER_RADIUS)?
            .into_iter()
            .map(|s| (s.dx, s.dy))
            .collect()
    };
    let mut moment_shifts = found_shifts.into_iter();
    let shifts: Vec<(i32, i32)> = rects
        .iter()
        .map(|t| {
            if is_part(t.0) {
                (0, 0)
            } else {
                moment_shifts.next().unwrap_or((0, 0))
            }
        })
        .collect();
    let mut cells = Vec::with_capacity(rects.len());
    let mut values: Vec<Vec<[f32; 3]>> = Vec::with_capacity(rects.len());
    for (((&(r, f, rect), fg), (px, tg)), sh) in
        rects.iter().zip(&sg.frames).zip(&sampled).zip(&shifts)
    {
        let row = sheet_rows
            .get(r)
            .map_or_else(|| format!("row{}", r + 1), |row| name_of(&row.0));
        let name = match (r == 0 && part_rows > 0, spec.parts.get(f)) {
            (true, Some(part)) => format!("{row}-{}", name_of(&part.name)),
            _ => format!("{row}-{}", f + 1),
        };
        let (px, tg) = (moved(px, sh.0, sh.1), moved(tg, sh.0, sh.1));
        let raw = |x: usize, y: usize| {
            (
                [px.at(x, y, 0), px.at(x, y, 1), px.at(x, y, 2)],
                px.at(x, y, 3),
            )
        };
        // the model copies the parts' pivot marks into the moments too: painted over everywhere, kept on parts only
        let (marks, cover) = pivot_mark(gw, gh, raw);
        let pivots = if is_part(r) { marks } else { Vec::new() };
        let at = |x: usize, y: usize| {
            let (rgb, a) = cover
                .iter()
                .find(|c| c.0 == (x, y))
                .map_or_else(|| raw(x, y), |c| c.1);
            let a = a / 255.0;
            let solid = a >= SOLID;
            let glow = rgb.map(|c| c * a);
            let light = if !solid && lum(glow) >= EMPTY {
                glow
            } else {
                [0.0; 3]
            };
            (rgb, a, solid, light)
        };
        let (mut solid_n, mut lit_n, mut err) = (0usize, 0usize, 0.0f64);
        let mut value = Vec::with_capacity(gw * gh);
        for y in 0..gh {
            for x in 0..gw {
                let (rgb, _, solid, light) = at(x, y);
                solid_n += usize::from(solid);
                lit_n += usize::from(lum(light) >= 1.0);
                let back = if solid { rgb } else { light };
                value.push(back);
                for (c, v) in back.iter().enumerate() {
                    err += f64::from((v - tg.at(x, y, c)).abs());
                }
            }
        }
        cells.push(Cell {
            name,
            rect: rect.map(|v| v as u32),
            pitch: (
                (rect[2] - rect[0]) as f64 / gw as f64,
                (rect[3] - rect[1]) as f64 / gh as f64,
            ),
            grid: fg.grid.clone(),
            strength: fg.strength,
            shift: *sh,
            colour: to_image(gw, gh, |x, y| {
                let (rgb, _, solid, _) = at(x, y);
                if solid {
                    [rgb[0], rgb[1], rgb[2], 255.0]
                } else {
                    [0.0; 4]
                }
            }),
            light: to_image(gw, gh, |x, y| {
                let l = at(x, y).3;
                [l[0], l[1], l[2], 255.0]
            }),
            alpha: to_image(gw, gh, |x, y| {
                let a = at(x, y).1 * 255.0;
                [a, a, a, 255.0]
            }),
            target: to_image(gw, gh, |x, y| {
                [tg.at(x, y, 0), tg.at(x, y, 1), tg.at(x, y, 2), 255.0]
            }),
            rebuilt: to_image(gw, gh, |x, y| {
                let (rgb, _, solid, light) = at(x, y);
                let b = if solid { rgb } else { light };
                [b[0], b[1], b[2], 255.0]
            }),
            solid: solid_n,
            lit: lit_n,
            mean_err: err / (gw * gh * 3) as f64,
            pivots,
        });
        values.push(value);
    }
    let skip = part_rows;
    let first = counts.iter().take(skip).sum::<usize>();
    let diffs = diffs(&cells[first..], &values[first..], &counts[skip..], gw, gh);
    // a moment drawn in layers is put back together, its whole frames first after the parts, so the player plays them
    let whole = composed(&cells, spec);
    let cells: Vec<Cell> = cells[..first]
        .iter()
        .cloned()
        .chain(whole)
        .chain(cells[first..].iter().cloned())
        .collect();
    Ok(Extracted {
        pitch: sg.pitch,
        fit: sg.fit,
        size: sg.size,
        found,
        cells,
        diffs,
    })
}

/// Each moment drawn in layers put back together as its whole frames, `<moment>-<k>`: the layers that are not light
/// laid over each other in the spec's order (the last on top, a layer's opaque colour covering what is under it, its
/// light added), then the light layers added over everything as pixfx adds light over colour: where nothing opaque is;
/// over the shapes they add nothing (the model draws the glow as a bright copy of the shapes, which would wash them
/// out). None for a spec without layers.
fn composed(cells: &[Cell], spec: &Spec) -> Vec<Cell> {
    let mut out = Vec::new();
    for r in &spec.rows {
        for k in 1..=spec.frames_of(r) as usize {
            let layer = |l: &crate::reference::spec::Layer| {
                let name = format!("{}-{k}", name_of(&format!("{}-{}", r.name, l.name)));
                cells.iter().find(|c| c.name == name).map(|c| (l.light, c))
            };
            let parts: Vec<(bool, &Cell)> = spec.layers.iter().filter_map(layer).collect();
            let Some(&(_, first)) = parts.first() else {
                continue;
            };
            let (w, h) = (first.rebuilt.w, first.rebuilt.h);
            let mut colour = RgbaImage::new(w, h);
            let mut light = RgbaImage::new(w, h);
            for p in light.rgba.as_chunks_mut::<4>().0 {
                *p = [0, 0, 0, 255];
            }
            let add = |light: &mut RgbaImage, x: u32, y: u32, v: [u8; 4]| {
                let o = light.get(x, y).unwrap_or([0, 0, 0, 255]);
                light.set(
                    x,
                    y,
                    [
                        o[0].saturating_add(v[0]),
                        o[1].saturating_add(v[1]),
                        o[2].saturating_add(v[2]),
                        255,
                    ],
                );
            };
            for &(_, c) in parts.iter().filter(|p| !p.0) {
                for y in 0..h {
                    for x in 0..w {
                        if let Some(px) = c.colour.get(x, y).filter(|p| p[3] > 0) {
                            colour.set(x, y, px);
                            light.set(x, y, [0, 0, 0, 255]);
                        }
                        add(&mut light, x, y, c.light.get(x, y).unwrap_or([0; 4]));
                    }
                }
            }
            for &(_, c) in parts.iter().filter(|p| p.0) {
                for y in 0..h {
                    for x in 0..w {
                        let v = c.rebuilt.get(x, y).unwrap_or([0; 4]);
                        if colour.get(x, y).is_none_or(|p| p[3] == 0) {
                            add(&mut light, x, y, v);
                        }
                    }
                }
            }
            let rebuilt = {
                let mut img = light.clone();
                for y in 0..h {
                    for x in 0..w {
                        if let Some(px) = colour.get(x, y).filter(|p| p[3] > 0) {
                            img.set(x, y, px);
                        }
                    }
                }
                img
            };
            let mut alpha = RgbaImage::new(w, h);
            for y in 0..h {
                for x in 0..w {
                    let a = parts
                        .iter()
                        .map(|p| p.1.alpha.get(x, y).map_or(0, |v| v[0]))
                        .max()
                        .unwrap_or(0);
                    alpha.set(x, y, [a, a, a, 255]);
                }
            }
            let count = |img: &RgbaImage, f: fn([u8; 4]) -> bool| {
                img.rgba
                    .as_chunks::<4>()
                    .0
                    .iter()
                    .filter(|p| f(**p))
                    .count()
            };
            out.push(Cell {
                name: format!("{}-{k}", name_of(&r.name)),
                rect: first.rect,
                pitch: first.pitch,
                grid: first.grid.clone(),
                strength: first.strength,
                shift: (0, 0),
                solid: count(&colour, |p| p[3] > 0),
                lit: count(&light, |p| p[0] > 0 || p[1] > 0 || p[2] > 0),
                colour,
                light,
                alpha,
                target: rebuilt.clone(),
                rebuilt,
                mean_err: 0.0,
                pivots: Vec::new(),
            });
        }
    }
    out
}

/// A pixel of a pivot mark and the colour and alpha it is painted over with.
type Cover = ((usize, usize), ([f32; 3], f32));
/// A part's pivot mark: the pixels of a `w` x `h` cell (`px` gives colour and alpha 0..255) that are
/// [`crate::reference::spec::PIVOT`] green, the middle of each mark (pixels touching, diagonals too, are one mark), and for each
/// of them the colour to paint it over with: the most
/// common opaque colour of the unmarked pixels within 2 when at least 4 of them are opaque, else the most common
/// of them all (transparent when there are none).
fn pivot_mark(
    w: usize,
    h: usize,
    px: impl Fn(usize, usize) -> ([f32; 3], f32),
) -> (Vec<(f64, f64)>, Vec<Cover>) {
    let green = |x: usize, y: usize| {
        let (c, a) = px(x, y);
        // green well above red and blue: the mark's soft or darker edge too, a mark on cyan picking up some blue;
        // an effect's jade or cyan stays far below
        a >= 128.0 && c[1] >= 120.0 && c[1] - c[0].max(c[2]) >= 90.0
    };
    let marked: Vec<(usize, usize)> = (0..h)
        .flat_map(|y| (0..w).map(move |x| (x, y)))
        .filter(|&(x, y)| green(x, y))
        .collect();
    // marks: flood each untouched marked pixel's neighbours
    let mut mark_of = vec![usize::MAX; marked.len()];
    let mut mids: Vec<(f64, f64)> = Vec::new();
    for start in 0..marked.len() {
        if mark_of[start] != usize::MAX {
            continue;
        }
        let id = mids.len();
        mark_of[start] = id;
        let (mut todo, mut sum, mut n) = (vec![start], (0.0, 0.0), 0.0);
        while let Some(i) = todo.pop() {
            let (x, y) = marked[i];
            sum = (sum.0 + x as f64, sum.1 + y as f64);
            n += 1.0;
            for (j, &(ox, oy)) in marked.iter().enumerate() {
                if mark_of[j] == usize::MAX && ox.abs_diff(x) <= 1 && oy.abs_diff(y) <= 1 {
                    mark_of[j] = id;
                    todo.push(j);
                }
            }
        }
        mids.push((sum.0 / n + 0.5, sum.1 / n + 0.5));
    }
    let cover = marked
        .iter()
        .map(|&(x, y)| {
            let mut seen: Vec<(([f32; 3], f32), usize)> = Vec::new();
            for ny in y.saturating_sub(2)..(y + 3).min(h) {
                for nx in x.saturating_sub(2)..(x + 3).min(w) {
                    if green(nx, ny) {
                        continue;
                    }
                    let v = px(nx, ny);
                    match seen.iter_mut().find(|s| s.0 == v) {
                        Some(s) => s.1 += 1,
                        None => seen.push((v, 1)),
                    }
                }
            }
            // the mark sits on the part's edge: fill with its colour when enough of it is round, so no notch is left
            let opaque: usize = seen.iter().filter(|s| (s.0).1 >= 128.0).map(|s| s.1).sum();
            let best = seen
                .iter()
                .filter(|s| opaque < 4 || (s.0).1 >= 128.0)
                .max_by_key(|s| s.1)
                .map_or(([0.0; 3], 0.0), |s| s.0);
            ((x, y), best)
        })
        .collect();
    (mids, cover)
}

/// Neighbouring frames of each row compared on colour or light (`values`, a cell's rebuilt pixels before rounding):
/// a pixel changes when its channels move by more than 40 in sum.
fn diffs(
    cells: &[Cell],
    values: &[Vec<[f32; 3]>],
    counts: &[usize],
    w: usize,
    h: usize,
) -> Vec<Diff> {
    let mut out = Vec::new();
    let mut at = 0;
    for &n in counts {
        let (row_cells, row_values) = (&cells[at..at + n], &values[at..at + n]);
        at += n;
        for (pair, vals) in row_cells.windows(2).zip(row_values.windows(2)) {
            let (a, b) = (&pair[0], &pair[1]);
            let mut img = RgbaImage::new(w as u32, h as u32);
            let mut changed = 0;
            for i in 0..w * h {
                let (va, vb) = (vals[0][i], vals[1][i]);
                let d: f32 = (0..3).map(|c| (va[c] - vb[c]).abs()).sum();
                let px = if d > 40.0 {
                    changed += 1;
                    if vb.iter().sum::<f32>() > va.iter().sum::<f32>() {
                        [80, 255, 80, 255]
                    } else {
                        [255, 70, 70, 255]
                    }
                } else {
                    [0, 0, 0, 255]
                };
                img.set((i % w) as u32, (i / w) as u32, px);
            }
            let stem = a.name.rsplit_once('-').map_or(a.name.as_str(), |s| s.0);
            let fa = a.name.rsplit('-').next().unwrap_or("");
            let fb = b.name.rsplit('-').next().unwrap_or("");
            out.push(Diff {
                name: format!("{stem}-{fa}-{fb}"),
                image: img,
                changed,
            });
        }
    }
    out
}
