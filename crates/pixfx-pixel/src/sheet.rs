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

/// The longest side a sheet may have, in sheet pixels; [`check`] refuses a bigger one.
pub const MAX_SIDE: u64 = 16384;

/// Whether a layout makes a sheet worth drawing: at least one cell of at least one pixel, and no side longer than
/// [`MAX_SIDE`]. Check a layout that came from outside (a flag, a file) before [`size`] or [`blank`], whose sums
/// would overflow on a huge one.
pub fn check(layout: &Layout) -> Result<(), String> {
    let l = layout;
    if l.cell_w == 0 || l.cell_h == 0 || l.cols == 0 || l.rows == 0 {
        return Err(format!(
            "a sheet needs cells: {}x{} cells of {}x{}",
            l.cols, l.rows, l.cell_w, l.cell_h
        ));
    }
    let side = |n: u32, cell: u32| {
        (u64::from(n) * u64::from(cell) + (u64::from(n) + 1) * u64::from(l.gap))
            * u64::from(l.scale.max(1))
    };
    let (w, h) = (side(l.cols, l.cell_w), side(l.rows, l.cell_h));
    if w > MAX_SIDE || h > MAX_SIDE {
        return Err(format!(
            "a {w}x{h} sheet is bigger than {MAX_SIDE} pixels a side: fewer cells or a smaller scale"
        ));
    }
    Ok(())
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

/// A bone: a straight line between two joints, in sprite pixels inside a cell.
pub type Bone = [(u32, u32); 2];

/// A bone and the cell (`C`, `R`) it belongs to; `None`: every cell.
pub type CellBone = (Option<(u32, u32)>, Bone);

/// Skeletons from their text form, a pose per cell. Blank lines and `#` lines are skipped. A line is a bone
/// `X,Y X,Y`, or `circle X,Y R` (a head: a ring of 12 bones of radius `R`). `cell C,R` starts the bones of cell
/// (`C`, `R`); bones before any `cell` line belong to every cell (`None`).
pub fn parse_bones(text: &str) -> Result<Vec<CellBone>, String> {
    let point = |p: &str| -> Option<(u32, u32)> {
        let (x, y) = p.split_once(',')?;
        Some((x.trim().parse().ok()?, y.trim().parse().ok()?))
    };
    let mut out = Vec::new();
    let mut cell = None;
    for (n, l) in text.lines().map(str::trim).enumerate() {
        if l.is_empty() || l.starts_with('#') {
            continue;
        }
        let bad = || {
            format!(
                "bones line {}: expected X,Y X,Y, circle X,Y R or cell C,R: {l}",
                n + 1
            )
        };
        match l.split_whitespace().collect::<Vec<_>>().as_slice() {
            ["cell", c] => cell = Some(point(c).ok_or_else(bad)?),
            ["circle", c, r] => {
                let (cx, cy) = point(c).ok_or_else(bad)?;
                let r: f64 = r.parse().map_err(|_| bad())?;
                let at = |k: u32| {
                    let a = f64::from(k) * core::f64::consts::TAU / 12.0;
                    let x = libm::round(f64::from(cx) + r * libm::cos(a)).max(0.0) as u32;
                    let y = libm::round(f64::from(cy) + r * libm::sin(a)).max(0.0) as u32;
                    (x, y)
                };
                for k in 0..12 {
                    out.push((cell, [at(k), at(k + 1)]));
                }
            }
            [a, b] => out.push((cell, [point(a).ok_or_else(bad)?, point(b).ok_or_else(bad)?])),
            _ => return Err(bad()),
        }
    }
    Ok(out)
}

/// The bones of cell (`col`, `row`): those given for every cell, then its own.
pub fn bones_of(bones: &[CellBone], col: u32, row: u32) -> Vec<Bone> {
    bones
        .iter()
        .filter(|(c, _)| c.is_none_or(|c| c == (col, row)))
        .map(|(_, b)| *b)
        .collect()
}

/// A layout as `sheet.json`.
pub fn layout_json(l: &Layout) -> String {
    format!(
        "{{\"cell_w\": {}, \"cell_h\": {}, \"cols\": {}, \"rows\": {}, \"gap\": {}, \"scale\": {}}}\n",
        l.cell_w, l.cell_h, l.cols, l.rows, l.gap, l.scale
    )
}

/// A layout back from `sheet.json` (a flat object of whole numbers, as [`layout_json`] writes it).
pub fn parse_layout(json: &str) -> Result<Layout, String> {
    let body = json.trim().trim_start_matches('{').trim_end_matches('}');
    let mut fields = std::collections::BTreeMap::new();
    for field in body.split(',').filter(|f| !f.trim().is_empty()) {
        let (k, v) = field
            .split_once(':')
            .ok_or(format!("sheet.json: expected \"key\": number, got {field}"))?;
        let v: u32 = v
            .trim()
            .parse()
            .map_err(|_| format!("sheet.json: {} is not a whole number", k.trim()))?;
        fields.insert(k.trim().trim_matches('"').to_string(), v);
    }
    let get = |k: &str| {
        fields
            .get(k)
            .copied()
            .ok_or(format!("sheet.json has no {k}"))
    };
    let layout = Layout {
        cell_w: get("cell_w")?,
        cell_h: get("cell_h")?,
        cols: get("cols")?,
        rows: get("rows")?,
        gap: get("gap")?,
        scale: get("scale")?,
    };
    check(&layout).map(|()| layout)
}

/// Caption ink.
const INK: [u8; 4] = [20, 20, 20, 255];

/// Write each cell's caption into the gap above it, cells in reading order (`texts[i]` over cell `i`, left to right,
/// top to bottom; an empty text draws nothing), all at one font size: the biggest that fits the longest. Fails
/// naming the first caption too long for the gap.
pub fn captions(img: &mut RgbaImage, layout: &Layout, texts: &[&str]) -> Result<(), String> {
    let k = caption_size(layout, texts)?;
    for (i, text) in texts.iter().enumerate() {
        let i = u32::try_from(i).unwrap_or(u32::MAX);
        let (col, row) = (i % layout.cols.max(1), i / layout.cols.max(1));
        if !text.trim().is_empty() && row < layout.rows {
            caption(img, layout, cell_rect(layout, col, row), text, k);
        }
    }
    Ok(())
}

/// The one font size every caption of a sheet is drawn at: the biggest that fits the longest.
pub(crate) fn caption_size(layout: &Layout, texts: &[&str]) -> Result<u32, String> {
    let band = layout.gap * layout.scale;
    let mut k = 4;
    for (i, text) in texts.iter().enumerate() {
        if text.trim().is_empty() {
            continue;
        }
        let fits = fit(text, layout.cell_w * layout.scale, band).ok_or_else(|| {
            let i = u32::try_from(i).unwrap_or(u32::MAX);
            format!(
                "cell {} ({},{}): caption \"{}\" does not fit the gap above the cell: shorten it, or raise the gap or the scale",
                i + 1,
                i % layout.cols.max(1),
                i / layout.cols.max(1),
                text
            )
        })?;
        k = k.min(fits);
    }
    Ok(k)
}

/// Words wrapped to lines of at most `width` characters; a word longer than a line is split.
fn wrap(text: &str, width: usize) -> Vec<String> {
    let width = width.max(1);
    let mut lines: Vec<String> = Vec::new();
    let mut line = String::new();
    for word in text.split_whitespace() {
        let mut word: Vec<char> = word.chars().collect();
        while word.len() > width {
            if !line.is_empty() {
                lines.push(std::mem::take(&mut line));
            }
            let rest = word.split_off(width);
            lines.push(word.into_iter().collect());
            word = rest;
        }
        let n = line.chars().count();
        if n > 0 && n + 1 + word.len() > width {
            lines.push(std::mem::take(&mut line));
        }
        if !line.is_empty() {
            line.push(' ');
        }
        line.extend(word);
    }
    if !line.is_empty() {
        lines.push(line);
    }
    lines
}

/// Font pixels a caption line takes: 7 of glyph, 3 of space.
const LINE: u32 = 10;

/// The biggest font size (1 to 4) at which `text` wraps into a band `band` pixels high over a cell `w` wide.
fn fit(text: &str, w: u32, band: u32) -> Option<u32> {
    (1..=4u32).rev().find(|&k| {
        let per_line = (w / (6 * k)) as usize;
        let n = u32::try_from(wrap(text, per_line).len()).unwrap_or(u32::MAX);
        per_line > 0 && n.saturating_mul(LINE * k).saturating_add(2) <= band
    })
}

/// Write `text` at font size `k` into the gap above the cell at `rect`.
pub(crate) fn caption(img: &mut RgbaImage, layout: &Layout, rect: [u32; 4], text: &str, k: u32) {
    let band = layout.gap * layout.scale;
    let [x0, y0, w, _] = rect;
    let lines = wrap(text, (w / (6 * k)) as usize);
    let need = u32::try_from(lines.len()).unwrap_or(0) * LINE * k;
    let top = y0 - band + band.saturating_sub(need) / 2 + k;
    for (li, line) in lines.iter().enumerate() {
        let ly = top + u32::try_from(li).unwrap_or(0) * LINE * k;
        for (ci, c) in line.chars().enumerate() {
            let cx = x0 + u32::try_from(ci).unwrap_or(0) * 6 * k;
            for gy in 0..crate::font::H {
                for gx in 0..crate::font::W {
                    if crate::font::ink(c, gx, gy) {
                        for dy in 0..k {
                            for dx in 0..k {
                                img.set(cx + gx * k + dx, ly + gy * k + dy, INK);
                            }
                        }
                    }
                }
            }
        }
    }
}
