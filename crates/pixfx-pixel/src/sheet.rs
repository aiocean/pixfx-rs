//! Sheets: a layout of cells for an image model to draw into, guides drawn into cells, and each cell's picture cut
//! back out of what the model drew.
//!
//! A sheet is drawn at `scale`: every sprite pixel is a `scale` x `scale` square of sheet pixels, so a model sees
//! the cells at the size it draws best. Cell size and gap are counted in sprite pixels; positions handed to guides
//! are sprite pixels inside the cell. The sheet's own marks are drawn in [`GUIDE`] and [`BORDER`], colours a
//! drawing does not use, and [`cut`] takes them out again.

use crate::oklab::{Lab, delta_e, lab};
use crate::{OPAQUE, Rgba, RgbaImage};

/// The colour of guides drawn into a cell: pivots, ground lines, bones.
pub const GUIDE: Rgba = Rgba(0xff, 0x00, 0xff, 0xff);
/// The colour of cell borders and pixel lines.
pub const BORDER: Rgba = Rgba(0x00, 0xff, 0xff, 0xff);
/// The colour of the sheet around and inside its cells.
const PAPER: [u8; 4] = [0xff, 0xff, 0xff, 0xff];
/// How close (OKLab ΔE) a drawn pixel must be to `GUIDE` or `BORDER` for `cut` to take it out.
const MARK_DELTA_E: f64 = 0.1;

/// Cells in `cols` x `rows`, each `cell_w` x `cell_h` sprite pixels, `gap` sprite pixels between cells and around
/// the edge, every sprite pixel drawn as a `scale` x `scale` square (a scale of 0 counts as 1).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Layout {
    pub cell_w: u32,
    pub cell_h: u32,
    pub cols: u32,
    pub rows: u32,
    pub gap: u32,
    pub scale: u32,
}

/// What a guide draws into a cell, in sprite pixels from the cell's top-left.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Guide {
    /// One pixel where the body turns.
    Pivot { x: u32, y: u32 },
    /// A line across the cell where the feet stand.
    Ground { y: u32 },
    /// A pose skeleton: each bone a straight line between two joints.
    Bones(Vec<[(u32, u32); 2]>),
    /// An existing sprite, centred across the cell and standing on its bottom; only its painted pixels are drawn.
    Sprite(RgbaImage),
}

fn scale(layout: &Layout) -> u32 {
    layout.scale.max(1)
}

/// The sheet's size in sheet pixels.
pub fn size(layout: &Layout) -> (u32, u32) {
    let s = scale(layout);
    (
        (layout.cols * layout.cell_w + (layout.cols + 1) * layout.gap) * s,
        (layout.rows * layout.cell_h + (layout.rows + 1) * layout.gap) * s,
    )
}

/// Cell (`col`, `row`) as `[x, y, w, h]` in sheet pixels.
pub fn cell_rect(layout: &Layout, col: u32, row: u32) -> [u32; 4] {
    let s = scale(layout);
    [
        (layout.gap + col * (layout.cell_w + layout.gap)) * s,
        (layout.gap + row * (layout.cell_h + layout.gap)) * s,
        layout.cell_w * s,
        layout.cell_h * s,
    ]
}

/// An empty sheet: white paper, each cell framed by a one-pixel `BORDER` ring just outside it (on the cell's own
/// outer pixels when there is no gap). With `pixel_lines`, a `BORDER` line also runs along the top and left of
/// every sprite pixel inside the cells, a plate to draw on (only at a scale of 2 or more).
pub fn blank(layout: &Layout, pixel_lines: bool) -> RgbaImage {
    let (w, h) = size(layout);
    let mut sheet = RgbaImage::new(w, h);
    for px in sheet.rgba.as_chunks_mut::<4>().0 {
        *px = PAPER;
    }
    let border = [BORDER.0, BORDER.1, BORDER.2, BORDER.3];
    let s = scale(layout);
    for row in 0..layout.rows {
        for col in 0..layout.cols {
            let [x, y, cw, ch] = cell_rect(layout, col, row);
            if cw == 0 || ch == 0 {
                continue;
            }
            // The ring's corners: outside the cell when the gap leaves room, else on its edge.
            let (x0, y0, x1, y1) = if layout.gap > 0 {
                (x - 1, y - 1, x + cw, y + ch)
            } else {
                (x, y, x + cw - 1, y + ch - 1)
            };
            for rx in x0..=x1 {
                sheet.set(rx, y0, border);
                sheet.set(rx, y1, border);
            }
            for ry in y0..=y1 {
                sheet.set(x0, ry, border);
                sheet.set(x1, ry, border);
            }
            if pixel_lines && s >= 2 {
                for cy in 0..ch {
                    for cx in 0..cw {
                        if cx % s == 0 || cy % s == 0 {
                            sheet.set(x + cx, y + cy, border);
                        }
                    }
                }
            }
        }
    }
    sheet
}

/// Fill sprite pixel (`px`, `py`) of cell (`col`, `row`) with `color`; nothing outside the cell.
fn fill(
    sheet: &mut RgbaImage,
    layout: &Layout,
    col: u32,
    row: u32,
    px: i64,
    py: i64,
    color: [u8; 4],
) {
    if px < 0 || py < 0 || px >= i64::from(layout.cell_w) || py >= i64::from(layout.cell_h) {
        return;
    }
    let s = scale(layout);
    let [x, y, _, _] = cell_rect(layout, col, row);
    let (sx, sy) = (x + px as u32 * s, y + py as u32 * s);
    for dy in 0..s {
        for dx in 0..s {
            sheet.set(sx + dx, sy + dy, color);
        }
    }
}

/// Draw a guide into cell (`col`, `row`) of a sheet laid out by `layout`; nothing reaches outside the cell.
pub fn guide(sheet: &mut RgbaImage, layout: &Layout, col: u32, row: u32, g: &Guide) {
    let mark = [GUIDE.0, GUIDE.1, GUIDE.2, GUIDE.3];
    match g {
        Guide::Pivot { x, y } => fill(sheet, layout, col, row, i64::from(*x), i64::from(*y), mark),
        Guide::Ground { y } => {
            for x in 0..layout.cell_w {
                fill(sheet, layout, col, row, i64::from(x), i64::from(*y), mark);
            }
        }
        Guide::Bones(bones) => {
            for [(ax, ay), (bx, by)] in bones {
                line(
                    i64::from(*ax),
                    i64::from(*ay),
                    i64::from(*bx),
                    i64::from(*by),
                    |x, y| {
                        fill(sheet, layout, col, row, x, y, mark);
                    },
                );
            }
        }
        Guide::Sprite(sprite) => {
            let ox = (i64::from(layout.cell_w) - i64::from(sprite.w)) / 2;
            let oy = i64::from(layout.cell_h) - i64::from(sprite.h);
            for sy in 0..sprite.h {
                for sx in 0..sprite.w {
                    if let Some(p) = sprite.get(sx, sy)
                        && p[3] >= OPAQUE
                    {
                        fill(
                            sheet,
                            layout,
                            col,
                            row,
                            ox + i64::from(sx),
                            oy + i64::from(sy),
                            p,
                        );
                    }
                }
            }
        }
    }
}

/// Every pixel of the straight line from (ax, ay) to (bx, by), both ends included (Bresenham).
fn line(ax: i64, ay: i64, bx: i64, by: i64, mut plot: impl FnMut(i64, i64)) {
    let (dx, dy) = ((bx - ax).abs(), -(by - ay).abs());
    let (stepx, stepy) = (if ax < bx { 1 } else { -1 }, if ay < by { 1 } else { -1 });
    let (mut x, mut y, mut err) = (ax, ay, dx + dy);
    loop {
        plot(x, y);
        if x == bx && y == by {
            return;
        }
        let e2 = 2 * err;
        if e2 >= dy {
            err += dy;
            x += stepx;
        }
        if e2 <= dx {
            err += dx;
            y += stepy;
        }
    }
}

/// A 3x3 sheet for a sprite's eight directions: the sprite in the centre cell (1, 1), N NE E SE S SW W NW around
/// it to be drawn. Cells are the sprite's size.
pub fn directions(sprite: &RgbaImage, scale: u32) -> (Layout, RgbaImage) {
    let layout = Layout {
        cell_w: sprite.w,
        cell_h: sprite.h,
        cols: 3,
        rows: 3,
        gap: 2,
        scale,
    };
    let mut sheet = blank(&layout, false);
    guide(&mut sheet, &layout, 1, 1, &Guide::Sprite(sprite.clone()));
    (layout, sheet)
}

/// A row of `n` frames of an animation.
pub fn frames(n: u32, cell_w: u32, cell_h: u32, scale: u32) -> Layout {
    Layout {
        cell_w,
        cell_h,
        cols: n,
        rows: 1,
        gap: 2,
        scale,
    }
}

/// Each cell's picture out of a drawn sheet, row by row. The drawn sheet may differ in size from the layout (a
/// model returns its own size): cell rectangles are scaled by `drawn.w / size.w` and `drawn.h / size.h`, edges to the nearest pixel. Pixels
/// within ΔE 0.1 of `GUIDE` or `BORDER` come back transparent. The pictures are at the drawn scale, ready for
/// `recover`.
pub fn cut(drawn: &RgbaImage, layout: &Layout) -> Vec<RgbaImage> {
    let (sw, sh) = size(layout);
    if sw == 0 || sh == 0 {
        return Vec::new();
    }
    // A sheet position on the drawn picture, to the nearest pixel (halves up), in integers.
    let along = |v: u32, drawn_side: u32, side: u32| -> u32 {
        ((2 * u64::from(v) * u64::from(drawn_side) + u64::from(side)) / (2 * u64::from(side)))
            as u32
    };
    let marks: [Lab; 2] = [lab(GUIDE), lab(BORDER)];
    let mut out = Vec::new();
    for row in 0..layout.rows {
        for col in 0..layout.cols {
            let [x, y, w, h] = cell_rect(layout, col, row);
            let (x0, y0) = (along(x, drawn.w, sw), along(y, drawn.h, sh));
            let (x1, y1) = (along(x + w, drawn.w, sw), along(y + h, drawn.h, sh));
            let mut cell = drawn.crop(x0, y0, x1 - x0, y1 - y0);
            for px in cell.rgba.as_chunks_mut::<4>().0 {
                let [r, g, b, a] = *px;
                if a >= OPAQUE
                    && marks
                        .iter()
                        .any(|m| delta_e(lab(Rgba(r, g, b, 255)), *m) <= MARK_DELTA_E)
                {
                    *px = [0, 0, 0, 0];
                }
            }
            out.push(cell);
        }
    }
    out
}
