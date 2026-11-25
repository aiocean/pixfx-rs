//! Sprite sheets: the grid layout and pixel packing are pure ([`Grid`], [`pack_cells`]); [`write_png_rgba`]
//! writes any RGBA image, and [`write_sheet`] an effect's sheets.

use std::fs;
use std::io::BufWriter;
use std::path::{Path, PathBuf};

use pixfx_core::{Meta, Rendered};
use pixfx_draw::{Frame, composite};

use crate::ExportError;
use crate::json::{FileNames, sheet_json};

/// Frames per sheet row, at most.
const MAX_COLUMNS: u32 = 8;

/// Where frames sit on a sheet: `columns` per row, one row after another, every cell `cell_w` x `cell_h`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Grid {
    pub frames: u32,
    pub columns: u32,
    pub rows: u32,
    pub cell_w: u32,
    pub cell_h: u32,
}

impl Grid {
    pub fn new(frames: u32, cell_w: u32, cell_h: u32) -> Self {
        let columns = frames.clamp(1, MAX_COLUMNS);
        Self {
            frames,
            columns,
            rows: frames.div_ceil(columns),
            cell_w,
            cell_h,
        }
    }

    pub fn width(&self) -> u32 {
        self.columns * self.cell_w
    }

    pub fn height(&self) -> u32 {
        self.rows * self.cell_h
    }

    /// Top-left pixel of cell `i`.
    pub fn cell_origin(&self, i: u32) -> (u32, u32) {
        (
            (i % self.columns) * self.cell_w,
            (i / self.columns) * self.cell_h,
        )
    }
}

/// Paths of the files [`write_sheet`] wrote.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SheetFiles {
    pub sheet: PathBuf,
    pub color: PathBuf,
    pub light: PathBuf,
    pub json: PathBuf,
}

/// Lay equally sized `cells` (each `grid.cell_w` x `grid.cell_h`, `bpp` bytes a pixel) out on one image, in
/// order; what no cell covers stays zero (transparent).
pub fn pack_cells(grid: &Grid, cells: &[Vec<u8>], bpp: usize) -> Vec<u8> {
    let (sheet_w, cell_w) = (grid.width() as usize, grid.cell_w as usize);
    let mut out = vec![0; sheet_w * grid.height() as usize * bpp];
    for (i, cell) in (0..).zip(cells) {
        let (cx, cy) = grid.cell_origin(i);
        for (y, row) in cell.chunks(cell_w * bpp).enumerate() {
            let start = ((cy as usize + y) * sheet_w + cx as usize) * bpp;
            if let Some(dst) = out.get_mut(start..start + row.len()) {
                dst.copy_from_slice(row);
            }
        }
    }
    out
}

/// The frame's colour layer as RGBA8 (colour words are ABGR, so their little-endian bytes are RGBA).
fn colour_rgba(f: &Frame) -> Vec<u8> {
    f.color.iter().flat_map(|c| c.to_le_bytes()).collect()
}

/// The frame's light layer as RGB8 (light words are xBGR).
fn light_rgb(f: &Frame) -> Vec<u8> {
    f.light
        .iter()
        .flat_map(|l| {
            let [r, g, b, _] = l.to_le_bytes();
            [r, g, b]
        })
        .collect()
}

/// The frame as it looks over transparency: colour with the light added.
fn composite_rgba(f: &Frame) -> Vec<u8> {
    let mut out = vec![0; f.w as usize * f.h as usize * 4];
    composite(f, &mut out, None);
    out
}

/// Write a `w` x `h` image of `color` pixels (8 bits a channel, rows top to bottom) to `path` as a PNG: the
/// one PNG encoder every export goes through.
fn write_png(
    path: &Path,
    w: u32,
    h: u32,
    data: &[u8],
    color: png::ColorType,
) -> Result<(), ExportError> {
    let file = BufWriter::new(fs::File::create(path)?);
    let mut enc = png::Encoder::new(file, w, h);
    enc.set_color(color);
    enc.set_depth(png::BitDepth::Eight);
    enc.write_header()?.write_image_data(data)?;
    Ok(())
}

/// Write a `w` x `h` RGBA image (8 bits a channel, straight alpha, rows top to bottom) to `path` as a PNG.
pub fn write_png_rgba(path: &Path, w: u32, h: u32, rgba: &[u8]) -> Result<(), ExportError> {
    write_png(path, w, h, rgba, png::ColorType::Rgba)
}

/// Write `<name>.png` (colour with light added), `<name>_color.png`, `<name>_light.png` (RGB, to add over
/// the colour sheet) and `<name>.json` into `out_dir`, creating it.
pub fn write_sheet(
    r: &Rendered,
    meta: &Meta,
    out_dir: &Path,
    name: &str,
) -> Result<SheetFiles, ExportError> {
    fs::create_dir_all(out_dir)?;
    let grid = Grid::new(r.frames.len() as u32, r.w, r.h);
    let names = FileNames::new(name);
    let files = SheetFiles {
        sheet: out_dir.join(&names.sheet),
        color: out_dir.join(&names.color),
        light: out_dir.join(&names.light),
        json: out_dir.join(&names.json),
    };
    let layers = |f: fn(&Frame) -> Vec<u8>, bpp| {
        let cells: Vec<Vec<u8>> = r.frames.iter().map(f).collect();
        pack_cells(&grid, &cells, bpp)
    };
    let (w, h) = (grid.width(), grid.height());
    write_png_rgba(&files.sheet, w, h, &layers(composite_rgba, 4))?;
    write_png_rgba(&files.color, w, h, &layers(colour_rgba, 4))?;
    write_png(
        &files.light,
        w,
        h,
        &layers(light_rgb, 3),
        png::ColorType::Rgb,
    )?;
    let json = sheet_json(r, meta, &grid, name, &names);
    fs::write(&files.json, serde_json::to_string_pretty(&json)?)?;
    Ok(files)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grid_wraps_at_eight_columns() {
        let g = Grid::new(10, 4, 3);
        assert_eq!((g.columns, g.rows, g.width(), g.height()), (8, 2, 32, 6));
        assert_eq!(g.cell_origin(9), (4, 3));
        let few = Grid::new(3, 4, 3);
        assert_eq!((few.columns, few.rows), (3, 1));
    }
}
