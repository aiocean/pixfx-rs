//! The template sheet sent to the model: a grid of cells, the guide figure in each, a label over each cell.

use pixfx_pixel::sheet::{Guide, Layout, blank, bones_of, cell_rect, check, guide, parse_bones};

use crate::{RgbaImage, Spec};

/// The sheet's grid: a row per moment, as many columns as the longest row.
pub fn layout(spec: &Spec) -> Layout {
    Layout {
        cell_w: spec.cell,
        cell_h: spec.cell,
        cols: spec.cols(),
        rows: u32::try_from(spec.rows.len()).unwrap_or(u32::MAX),
        gap: spec.gap,
        scale: spec.scale,
    }
}

/// Each cell's label in reading order: `INIT 1/3 - shield forms` over a row's first cell, `INIT 2/3` over the rest,
/// an empty label past the end of a row shorter than the sheet.
pub fn labels(spec: &Spec) -> Vec<String> {
    let cols = spec.cols();
    spec.rows
        .iter()
        .flat_map(|r| {
            let n = spec.frames_of(r);
            (1..=cols).map(move |f| {
                if f > n {
                    String::new()
                } else if f == 1 && !r.caption.trim().is_empty() {
                    format!("{} {f}/{n} - {}", r.name, r.caption)
                } else {
                    format!("{} {f}/{n}", r.name)
                }
            })
        })
        .collect()
}

/// The template: white paper, framed cells, the guide figure in every cell, the labels in the gaps; the cells past
/// the end of a short row are left out (plain paper). Fails on a grid too big to draw, a figure that does not parse,
/// or a label too long for its gap (raise `gap`).
pub fn render(spec: &Spec) -> Result<RgbaImage, String> {
    let layout = layout(spec);
    check(&layout)?;
    let mut sheet = blank(&layout, false);
    let bones = parse_bones(&spec.figure)?;
    for (row, r) in (0..layout.rows).zip(&spec.rows) {
        for col in 0..layout.cols {
            if col >= spec.frames_of(r) {
                erase(&mut sheet, &layout, col, row);
                continue;
            }
            let b = bones_of(&bones, col, row);
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

/// Paper over cell (`col`, `row`) and the frame round it.
fn erase(sheet: &mut RgbaImage, layout: &Layout, col: u32, row: u32) {
    let [x, y, w, h] = cell_rect(layout, col, row);
    for py in y.saturating_sub(1)..(y + h + 1).min(sheet.h) {
        for px in x.saturating_sub(1)..(x + w + 1).min(sheet.w) {
            sheet.set(px, py, [255, 255, 255, 255]);
        }
    }
}
