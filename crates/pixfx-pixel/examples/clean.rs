//! Clean an image model's picture down to one pixel a cell: crop, mirror, cut it from its background, recover its
//! pixel grid, fit it to a height, then snap it to a palette or merge its colours down to a count. Writes
//! `<out>/<picture>.clean.png`.
//!
//!   cargo run -p pixfx-pixel --example clean -- <picture.png> [--crop X,Y,W,H] [--flip] [--cut TOLERANCE]
//!       [--height N] [--palette P] [--max-colors N] [--out DIR]
//!
//! P is a fighter look (thunder, frost, alchemist) or hex colours split by commas; `--out` defaults to `target/pixel`.

mod common;

use common::{Args, exit, read_image, stem, write_image};
use pixfx_pixel::tool::{CleanOptions, clean};

const USAGE: &str = "usage: clean <picture.png> [--crop X,Y,W,H] [--flip] [--cut TOLERANCE] [--height N] [--palette P] [--max-colors N] [--out DIR]";

fn main() -> std::process::ExitCode {
    exit(run())
}

fn run() -> Result<bool, String> {
    let mut args = Args::new(USAGE);
    let defaults = CleanOptions::default();
    let opts = CleanOptions {
        crop: args.rect("--crop")?,
        flip: args.flag("--flip"),
        cut: args
            .value("--cut")?
            .map(|v| v.parse())
            .transpose()
            .map_err(|_| USAGE)?,
        height: args
            .value("--height")?
            .map(|v| v.parse())
            .transpose()
            .map_err(|_| USAGE)?,
        palette: args.palette()?,
        max_colors: args.number("--max-colors", defaults.max_colors)?,
    };
    let out = args.out()?;
    let [picture] = args.exactly()?;
    let c = clean(&read_image(&picture)?, &opts)?;
    println!(
        "pitch {:.2}, cells {}x{}, confidence {:.2}",
        c.pitch, c.sprite.w, c.sprite.h, c.confidence
    );
    write_image(
        &out.join(format!("{}.clean.png", stem(&picture))),
        &c.sprite,
    )?;
    Ok(true)
}
