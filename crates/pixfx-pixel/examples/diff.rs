//! How many pixels two images differ by, each cropped first when asked; `--anchor feet` stands both on the same spot
//! (bottom painted row, middle of the painted columns) to compare a drawing with a sprite whatever their margins.
//! With `--out`, writes a, b and the difference side by side. Exits 0 when they match, 1 when they differ.
//!
//!   cargo run -p pixfx-pixel --example diff -- <a.png> <b.png> [--a-crop X,Y,W,H] [--b-crop X,Y,W,H]
//!       [--anchor feet] [--out PNG]

mod common;

use common::{Args, exit, read_image};
use pixfx_export::{Grid, pack_cells, write_png_rgba};
use pixfx_pixel::{RgbaImage, diff, feet};

const USAGE: &str =
    "usage: diff <a.png> <b.png> [--a-crop X,Y,W,H] [--b-crop X,Y,W,H] [--anchor feet] [--out PNG]";

fn main() -> std::process::ExitCode {
    exit(run())
}

fn cropped(img: RgbaImage, rect: Option<[u32; 4]>) -> RgbaImage {
    match rect {
        Some([x, y, w, h]) => img.crop(x, y, w, h),
        None => img,
    }
}

fn run() -> Result<bool, String> {
    let mut args = Args::new(USAGE);
    let (a_crop, b_crop) = (args.rect("--a-crop")?, args.rect("--b-crop")?);
    let anchor_feet = match args.value("--anchor")?.as_deref() {
        None => false,
        Some("feet") => true,
        Some(_) => return Err(format!("--anchor takes feet\n{USAGE}")),
    };
    let out = args.value("--out")?;
    let [a_path, b_path] = args.exactly()?;
    let a = cropped(read_image(&a_path)?, a_crop);
    let b = cropped(read_image(&b_path)?, b_crop);
    let (a, b) = if anchor_feet { feet(&a, &b) } else { (a, b) };
    let (d, mask) = diff(&a, &b);
    println!("{} of {} pixels differ", d.differing, d.total);
    if let Some(path) = out {
        if (a.w, a.h) != (b.w, b.h) {
            return Err(format!(
                "side by side needs one size: {}x{} and {}x{}",
                a.w, a.h, b.w, b.h
            ));
        }
        let grid = Grid::new(3, a.w, a.h);
        let sheet = pack_cells(&grid, &[a.rgba, b.rgba, mask.rgba], 4);
        write_png_rgba(
            std::path::Path::new(&path),
            grid.width(),
            grid.height(),
            &sheet,
        )
        .map_err(|e| format!("writing {path}: {e}"))?;
        println!("side by side: {path}");
    }
    Ok(d.differing == 0)
}
