//! Read a sprite (one pixel a cell) as text: a letter per cell under a colour legend, named when the palette is a
//! fighter look's, and with `--rects` the cells merged into rects. Prints it and writes `<out>/<sprite>.txt`, which
//! the `draw` script takes back.
//!
//!   cargo run -p pixfx-pixel --example read -- <sprite.png> [--palette P] [--region X,Y,W,H] [--rects] [--out DIR]

mod common;

use common::{Args, exit, read_image, stem, write_text};
use pixfx_pixel::tool::read;

const USAGE: &str =
    "usage: read <sprite.png> [--palette P] [--region X,Y,W,H] [--rects] [--out DIR]";

fn main() -> std::process::ExitCode {
    exit(run())
}

fn run() -> Result<bool, String> {
    let mut args = Args::new(USAGE);
    let palette = args.palette()?;
    let region = args.rect("--region")?;
    let rects = args.flag("--rects");
    let out = args.out()?;
    let [sprite] = args.exactly()?;
    let listing = read(&read_image(&sprite)?, palette.as_ref(), region, rects)?;
    print!("{listing}");
    write_text(&out.join(format!("{}.txt", stem(&sprite))), &listing)?;
    Ok(true)
}
