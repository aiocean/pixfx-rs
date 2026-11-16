//! The template sheet sent to the model: a grid of cells, a label over each; the parts row (when the spec has parts)
//! holds empty cells for the parts drawn alone, a row per part with forms one cell per form, every moment cell holds the guide figure; with `grid`, every cell
//! shows its pixel grid.

use pixfx_pixel::sheet::{
    Bone, Guide, Layout, blank, bones_of, cell_rect, check, guide, parse_bones,
};

use crate::RgbaImage;
use crate::spec::{PARTS, Spec};

/// The grid's lines: thin round every sprite pixel, darker every [`MAJOR`] pixels. Grey, so the model reads them as
/// a guide and the cut never takes them for the effect.
pub const MINOR: [u8; 4] = [216, 216, 226, 255];
pub const DARK: [u8; 4] = [150, 150, 166, 255];
pub const MAJOR: u32 = 8;

/// The cell size the figure's coordinates are given in.
const FIGURE_CELL: u32 = 96;

/// The sheet's grid: the parts row, then a row per moment, as many columns as the longest row.
pub fn layout(spec: &Spec) -> Layout {
    Layout {
        cell_w: spec.cell,
        cell_h: spec.cell,
        cols: spec.cols(),
        rows: u32::try_from(spec.sheet_rows().len()).unwrap_or(u32::MAX),
        gap: spec.gap,
        scale: spec.scale,
    }
}

/// Each cell's label in reading order: `PARTS 1/4 - MAIN` over each part, `INIT 1/3 - shield forms` over a row's
/// first cell (with layers, `INIT-SOLID 1/3 - shield forms` on the first layer's row only), `INIT 2/3` over the rest, an empty label past the end of a row shorter than the sheet.
pub fn labels(spec: &Spec) -> Vec<String> {
    let cols = spec.cols();
    let parts = spec.parts.len();
    let mut out: Vec<String> = Vec::new();
    if parts > 0 {
        out.extend((1..=cols as usize).map(|f| match spec.parts.get(f - 1) {
            Some(p) => format!("{PARTS} {f}/{parts} - {}", p.name),
            None => String::new(),
        }));
    }
    for p in spec.parts.iter().filter(|p| !p.forms.is_empty()) {
        let n = p.forms.len();
        out.extend((1..=cols as usize).map(|f| {
            if f > n {
                String::new()
            } else {
                format!("{} {f}/{n}", p.name)
            }
        }));
    }
    let first_layer = spec.layers.first();
    for (name, r, layer) in spec.moment_rows() {
        let n = spec.frames_of(r);
        let captioned = !r.caption.trim().is_empty() && layer == first_layer;
        out.extend((1..=cols).map(|f| {
            if f > n {
                String::new()
            } else if f == 1 && captioned {
                format!("{name} {f}/{n} - {}", r.caption)
            } else {
                format!("{name} {f}/{n}")
            }
        }));
    }
    out
}

/// The template: white paper, framed cells, the guide figure in every moment cell (scaled from a 96-pixel cell to
/// the spec's), the pixel grid in every cell with `grid`, the labels in the gaps; the cells past the end of a short
/// row are left out (plain paper). Fails on a grid too big to draw, a figure that does not parse, or a label too
/// long for its gap (raise `gap`).
pub fn render(spec: &Spec) -> Result<RgbaImage, String> {
    let layout = layout(spec);
    check(&layout)?;
    let mut sheet = blank(&layout, false);
    let bones = parse_bones(&spec.figure)?;
    let first_moment = u32::try_from(spec.part_rows().len()).unwrap_or(u32::MAX);
    for (row, (_, cells)) in (0..layout.rows).zip(spec.sheet_rows()) {
        for col in 0..layout.cols {
            if col >= cells {
                erase(&mut sheet, &layout, col, row);
                continue;
            }
            if spec.grid {
                pixel_grid(&mut sheet, &layout, col, row);
            }
            if row < first_moment {
                continue;
            }
            let b: Vec<Bone> = bones_of(&bones, col, row - first_moment)
                .iter()
                .map(|b| scaled(*b, spec.cell))
                .collect();
            if !b.is_empty() {
                guide(&mut sheet, &layout, col, row, &Guide::Bones(b));
            }
        }
    }
    let labels = labels(spec);
    let texts: Vec<&str> = labels.iter().map(String::as_str).collect();
    pixfx_pose::captions(&mut sheet, &layout, &texts)?;
    Ok(sheet)
}

/// A bone given in a 96-pixel cell, in a cell of `cell` pixels.
fn scaled(b: Bone, cell: u32) -> Bone {
    let s = |v: u32| (u64::from(v) * u64::from(cell) / u64::from(FIGURE_CELL)) as u32;
    b.map(|(x, y)| {
        (
            s(x).min(cell.saturating_sub(1)),
            s(y).min(cell.saturating_sub(1)),
        )
    })
}

/// The lines round every sprite pixel of cell (`col`, `row`): [`MINOR`], and [`DARK`] every [`MAJOR`] pixels.
fn pixel_grid(sheet: &mut RgbaImage, layout: &Layout, col: u32, row: u32) {
    let [x, y, w, h] = cell_rect(layout, col, row);
    let s = layout.scale.max(1);
    if s < 2 {
        return;
    }
    for k in 1..layout.cell_w {
        let c = if k % MAJOR == 0 { DARK } else { MINOR };
        for py in y..y + h {
            sheet.set(x + k * s, py, c);
        }
    }
    for k in 1..layout.cell_h {
        let c = if k % MAJOR == 0 { DARK } else { MINOR };
        for px in x..x + w {
            sheet.set(px, y + k * s, c);
        }
    }
}

/// Paper over cell (`col`, `row`) and the frame round it.
fn erase(sheet: &mut RgbaImage, layout: &Layout, col: u32, row: u32) {
    let [x, y, w, h] = cell_rect(layout, col, row);
    for py in y.saturating_sub(1)..(y + h + 1).min(sheet.h) {
        for px in x.saturating_sub(1)..(x + w + 1).min(sheet.w) {
            sheet.set(px, py, [255, 255, 255, 255]);
        }
    }
}
