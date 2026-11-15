//! The effect's pixels cut back out of a sheet the model drew on a transparent background.
//!
//! The model returns the effect with an alpha channel and no backdrop, so there is no background to estimate and no
//! character to remove (the prompt asks for none). In order:
//!
//! 1. Cells. The model redraws the page, so the template's coordinates do not hold, but it keeps the labels: the
//!    only opaque, neutral, near-black marks. Each label line gives a row; the label starts, evenly spaced, give the
//!    columns. A cell is the square under its label, as wide as the template's cell is within its column pitch.
//!    The model often keeps the template's cyan cell frames; near a cell's edge that cyan is taken out, as
//!    `pixfx_pixel::sheet::cut` takes out the marks it drew.
//! 2. Grid. The model draws its own pixels, not the template's: about 2 to 4 sheet pixels each, 2s and 3s mixed,
//!    and not square. [`pixfx_pixel::frames::recover_frames`] finds the pitch from every cell at once and gives
//!    every frame the same number of model pixels, each frame its own cuts. A sheet with no edges to read a grid
//!    from (blurred, or blocks too big) falls back to the template's sprite pixels, even cells.
//! 3. Pixels. One sample per model pixel: the median of the middle of its block (half its width and height, at
//!    least one sheet pixel), colour and alpha.
//! 4. Origin. The model draws the subject a few pixels off in some frames, often a whole row at once; each frame
//!    is moved by whole model pixels to line up with the others ([`pixfx_pixel::register`], on the edges of the
//!    cell over black), so pixel `(x, y)` is the same place in every frame.
//! 5. Layers, as pixfx shows a surface (colour drawn, then light added on top), over black. Colour: the drawn colour
//!    where the pixel is nearly opaque, exactly as the model drew it. Light: colour times alpha where it is
//!    see-through (the glow), which is what that pixel adds over black. Nothing is snapped to a palette and no
//!    light is taken out of a colour pixel, so hues and edges stay as drawn.
//! 6. Check: colour or light against the cell over black, as a mean error per channel.
//! 7. Diff: the pixels that change between neighbouring frames of a row.
//!
//! Halves round to even (`libm::rint`), as the first version of this step did (in Python), so the cells it found
//! on the shields are found again.

use pixfx_pixel::frames::{SheetGrid, recover_frames};
use pixfx_pixel::recover::Grid;
use pixfx_pixel::register::register;
use pixfx_pixel::sheet::Layout;

use crate::{RgbaImage, Spec};

/// A row's name as the first part of its cells' names: lower case, spaces as dashes (`HIT RIGHT` is `hit-right`).
pub fn name_of(row: &str) -> String {
    row.split_whitespace().collect::<Vec<_>>().join("-").to_lowercase()
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

/// The cells' rectangles, a list per row, from the label lines: row `r` is the first label line below row `r - 1`
/// with `counts[r]` labels spaced evenly from the left edge.
fn find_cells(img: &Plane, layout: &Layout, counts: &[usize]) -> Result<Vec<Vec<[usize; 4]>>, String> {
    let (w, h) = (img.w, img.h);
    let ink: Vec<bool> = (0..w * h)
        .map(|i| {
            let (x, y) = (i % w, i / w);
            let [r, g, b, a] = [img.at(x, y, 0), img.at(x, y, 1), img.at(x, y, 2), img.at(x, y, 3)];
            let (hi, lo) = (r.max(g).max(b), r.min(g).min(b));
            a > 0.7 * 255.0 && hi < 70.0 && hi - lo < 20.0
        })
        .collect();
    let row_ink: Vec<bool> = (0..h).map(|y| ink[y * w..(y + 1) * w].iter().filter(|&&v| v).count() > 3).collect();
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
    let candidates: Vec<(usize, usize)> = runs(&row_ink, short, 0).into_iter().filter(|r| r.1 - r.0 < tall).collect();
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
        .map(|c| median(&mut lines.iter().filter_map(|l| l.1.get(c).map(|&x| x as f32)).collect::<Vec<_>>()) as f64)
        .collect();
    let pitch = median(&mut col_x.windows(2).map(|p| (p[1] - p[0]) as f32).collect::<Vec<_>>()) as f64;
    let size = libm::rint(pitch * f64::from(layout.cell_w) / f64::from(layout.cell_w + layout.gap)) as usize;
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
    let (gw, gh) = (grid.cols.len().saturating_sub(1), grid.rows.len().saturating_sub(1));
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
    Plane { w: gw, h: gh, ch: p.ch, v }
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
    Plane { w: p.w, h: p.h, ch: p.ch, v }
}

fn crop(img: &RgbaImage, [x0, y0, x1, y1]: [usize; 4]) -> RgbaImage {
    let mut out = RgbaImage::new((x1 - x0) as u32, (y1 - y0) as u32);
    for y in y0..y1 {
        for x in x0..x1 {
            out.set((x - x0) as u32, (y - y0) as u32, img.get(x as u32, y as u32).unwrap_or([0; 4]));
        }
    }
    out
}

/// The cells' shared grid and `true`, or the template's even cells (`gw` by `gh`) and `false` when no grid shows.
fn grid_of(cells: &[RgbaImage], gw: usize, gh: usize) -> (SheetGrid, bool) {
    match recover_frames(cells) {
        Ok(g) => (g, true),
        Err(_) => {
            let even = |len: u32, n: usize| (0..=n).map(|k| (k as u64 * u64::from(len) / n as u64) as u32).collect();
            let (w, h) = cells.first().map_or((0, 0), |c| (c.w, c.h));
            let g = SheetGrid {
                pitch: (f64::from(w) / gw as f64, f64::from(h) / gh as f64),
                fit: (0.0, 0.0),
                size: (gw as u32, gh as u32),
                frames: cells
                    .iter()
                    .map(|_| pixfx_pixel::frames::FrameGrid {
                        grid: Grid { cols: even(w, gw), rows: even(h, gh) },
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

/// Model pixels a frame may be moved, each way, to line up with the others.
const REGISTER_RADIUS: u32 = 8;

/// Sprite pixels from a cell's edge within which the model's copy of the template's cell frame may fall.
const FRAME_BAND: usize = 4;

/// Whether straight colour `p` is the template's cell-frame cyan (`pixfx_pixel::sheet::BORDER`) as a model redraws
/// it, its soft edge too: green and blue alike, bright, well above red.
fn frame_cyan(p: [u8; 4]) -> bool {
    let [r, g, b, a] = p.map(i32::from);
    a > 0 && g > 120 && g - r > 30 && b - r > 30 && (g - b).abs() < 40
}

/// `sheet` with the cell frames the model kept taken out: in each cell's outer [`FRAME_BAND`] sprite pixels, frame
/// cyan becomes empty. An effect's own cyan that close to a cell's edge goes with it.
fn without_frames(sheet: &RgbaImage, rects: impl Iterator<Item = [usize; 4]>, grid: usize) -> RgbaImage {
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
    }
    out
}

/// Cut `sheet` (drawn by the model from `spec`'s template) into frames. Fails when the labels cannot be found: a
/// label line missing, or one with the wrong number of labels.
pub fn extract(sheet: &RgbaImage, spec: &Spec) -> Result<Extracted, String> {
    let layout = &crate::template::layout(spec);
    let (w, h) = (sheet.w as usize, sheet.h as usize);
    let counts: Vec<usize> = spec.rows.iter().map(|r| spec.frames_of(r) as usize).collect();
    let raw = Plane { w, h, ch: 4, v: sheet.rgba.iter().map(|&b| f32::from(b)).collect() };
    let rows = find_cells(&raw, layout, &counts)?;
    let rects: Vec<(usize, usize, [usize; 4])> = rows
        .iter()
        .enumerate()
        .flat_map(|(r, row)| row.iter().enumerate().map(move |(f, &rect)| (r, f, rect)))
        .collect();
    let sheet = &without_frames(sheet, rects.iter().map(|t| t.2), layout.cell_w as usize);
    let rgba = Plane { w, h, ch: 4, v: sheet.rgba.iter().map(|&b| f32::from(b)).collect() };
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
                [f32::from(p[0]) * a, f32::from(p[1]) * a, f32::from(p[2]) * a]
            })
            .collect(),
    };
    let crops: Vec<RgbaImage> = rects.iter().map(|t| crop(sheet, t.2)).collect();
    let (sg, found) = grid_of(&crops, layout.cell_w as usize, layout.cell_h as usize);
    let (gw, gh) = (sg.size.0 as usize, sg.size.1 as usize);
    let sampled: Vec<(Plane, Plane)> = rects
        .iter()
        .zip(&sg.frames)
        .map(|(t, fg)| (sample(&rgba, t.2, &fg.grid), sample(&over, t.2, &fg.grid)))
        .collect();
    let seen: Vec<RgbaImage> = sampled
        .iter()
        .map(|(_, tg)| to_image(gw, gh, |x, y| [tg.at(x, y, 0), tg.at(x, y, 1), tg.at(x, y, 2), 255.0]))
        .collect();
    let shifts = register(&seen, REGISTER_RADIUS)?;
    let mut cells = Vec::with_capacity(rects.len());
    let mut values: Vec<Vec<[f32; 3]>> = Vec::with_capacity(rects.len());
    for (((&(r, f, rect), fg), (px, tg)), sh) in rects.iter().zip(&sg.frames).zip(&sampled).zip(&shifts) {
        let row = spec.rows.get(r).map_or_else(|| format!("row{}", r + 1), |row| name_of(&row.name));
        let (px, tg) = (moved(px, sh.dx, sh.dy), moved(tg, sh.dx, sh.dy));
        let at = |x: usize, y: usize| {
            let rgb = [px.at(x, y, 0), px.at(x, y, 1), px.at(x, y, 2)];
            let a = px.at(x, y, 3) / 255.0;
            let solid = a >= SOLID;
            let glow = rgb.map(|c| c * a);
            let light = if !solid && lum(glow) >= EMPTY { glow } else { [0.0; 3] };
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
            name: format!("{row}-{}", f + 1),
            rect: rect.map(|v| v as u32),
            pitch: (
                (rect[2] - rect[0]) as f64 / gw as f64,
                (rect[3] - rect[1]) as f64 / gh as f64,
            ),
            grid: fg.grid.clone(),
            strength: fg.strength,
            shift: (sh.dx, sh.dy),
            colour: to_image(gw, gh, |x, y| {
                let (rgb, _, solid, _) = at(x, y);
                if solid { [rgb[0], rgb[1], rgb[2], 255.0] } else { [0.0; 4] }
            }),
            light: to_image(gw, gh, |x, y| {
                let l = at(x, y).3;
                [l[0], l[1], l[2], 255.0]
            }),
            alpha: to_image(gw, gh, |x, y| {
                let a = at(x, y).1 * 255.0;
                [a, a, a, 255.0]
            }),
            target: to_image(gw, gh, |x, y| [tg.at(x, y, 0), tg.at(x, y, 1), tg.at(x, y, 2), 255.0]),
            rebuilt: to_image(gw, gh, |x, y| {
                let (rgb, _, solid, light) = at(x, y);
                let b = if solid { rgb } else { light };
                [b[0], b[1], b[2], 255.0]
            }),
            solid: solid_n,
            lit: lit_n,
            mean_err: err / (gw * gh * 3) as f64,
        });
        values.push(value);
    }
    let diffs = diffs(&cells, &values, &counts, gw, gh);
    Ok(Extracted { pitch: sg.pitch, fit: sg.fit, size: sg.size, found, cells, diffs })
}

/// Neighbouring frames of each row compared on colour or light (`values`, a cell's rebuilt pixels before rounding):
/// a pixel changes when its channels move by more than 40 in sum.
fn diffs(cells: &[Cell], values: &[Vec<[f32; 3]>], counts: &[usize], w: usize, h: usize) -> Vec<Diff> {
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
                    if vb.iter().sum::<f32>() > va.iter().sum::<f32>() { [80, 255, 80, 255] } else { [255, 70, 70, 255] }
                } else {
                    [0, 0, 0, 255]
                };
                img.set((i % w) as u32, (i / w) as u32, px);
            }
            let stem = a.name.rsplit_once('-').map_or(a.name.as_str(), |s| s.0);
            let fa = a.name.rsplit('-').next().unwrap_or("");
            let fb = b.name.rsplit('-').next().unwrap_or("");
            out.push(Diff { name: format!("{stem}-{fa}-{fb}"), image: img, changed });
        }
    }
    out
}
