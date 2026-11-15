//! A blank sheet for an image model to draw into: `cell` x `grid` cells, or (`--directions`) a 3x3 round a sprite
//! for its eight directions; `--bones` draws skeletons into the cells (`X,Y X,Y`, `circle X,Y R`, `cell C,R`
//! starting a cell's own). Writes `<out>/sheet.png` and `<out>/sheet.json`, which the `cut` script takes back.
//!
//!   cargo run -p pixfx-pixel --example sheet -- (--cell WxH --grid CxR | --directions <sprite.png>) [--gap N]
//!       [--scale N] [--pixel-lines] [--bones FILE] [--out DIR]

mod common;

use std::path::PathBuf;

use common::{Args, exit, read_image, read_text, write_image, write_text};
use pixfx_pixel::sheet::{layout_json, parse_bones};
use pixfx_pixel::tool::{SheetOptions, sheet};

const USAGE: &str = "usage: sheet (--cell WxH --grid CxR | --directions <sprite.png>) [--gap N] [--scale N] [--pixel-lines] [--bones FILE] [--out DIR]";

fn main() -> std::process::ExitCode {
    exit(run())
}

fn run() -> Result<bool, String> {
    let mut args = Args::new(USAGE);
    let (cell, grid) = (args.pair("--cell")?, args.pair("--grid")?);
    let directions = args.value("--directions")?.map(PathBuf::from);
    let bones = args.value("--bones")?.map(PathBuf::from);
    let gap = args.number("--gap", 2)?;
    let scale = args.number("--scale", 8)?;
    let pixel_lines = args.flag("--pixel-lines");
    let out = args.out()?;
    if !args.files()?.is_empty() {
        return Err(format!("sheet takes no file but its flags\n{USAGE}"));
    }
    let (cell, grid) = match (cell, grid, &directions) {
        (Some(c), Some(g), None) => (c, g),
        (None, None, Some(_)) => ((0, 0), (3, 3)),
        _ => {
            return Err(format!(
                "sheet needs --cell WxH --grid CxR, or --directions\n{USAGE}"
            ));
        }
    };
    let opts = SheetOptions {
        cell,
        grid,
        gap,
        scale,
        pixel_lines,
    };
    let sprite = directions.as_deref().map(read_image).transpose()?;
    let bones = match &bones {
        Some(p) => parse_bones(&read_text(p)?)?,
        None => Vec::new(),
    };
    let (layout, img) = sheet(&opts, sprite.as_ref(), &bones)?;
    write_image(&out.join("sheet.png"), &img)?;
    write_text(&out.join("sheet.json"), &layout_json(&layout))?;
    Ok(true)
}
