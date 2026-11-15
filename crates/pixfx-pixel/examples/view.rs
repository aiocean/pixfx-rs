//! View a sprite at a size worth looking at: `<out>/<sprite>.view.png`, grid lines and all. A sprite too big to show
//! cell by cell comes out as an overview; look closer with `--region`.
//!
//!   cargo run -p pixfx-pixel --example view -- <sprite.png> [--region X,Y,W,H] [--max-side N] [--out DIR]

mod common;

use common::{Args, exit, read_image, stem, write_image};
use pixfx_pixel::tool::view;
use pixfx_pixel::view::ViewOptions;

const USAGE: &str = "usage: view <sprite.png> [--region X,Y,W,H] [--max-side N] [--out DIR]";

fn main() -> std::process::ExitCode {
    exit(run())
}

fn run() -> Result<bool, String> {
    let mut args = Args::new(USAGE);
    let region = args.rect("--region")?;
    let max_side = args.number("--max-side", ViewOptions::default().max_side)?;
    let out = args.out()?;
    let [sprite] = args.exactly()?;
    let v = view(&read_image(&sprite)?, region, max_side);
    if v.overview {
        println!(
            "overview: a pixel is {0}x{0} cells; look closer with --region",
            v.scale
        );
    } else {
        println!("{} screen pixels a cell", v.scale);
    }
    write_image(&out.join(format!("{}.view.png", stem(&sprite))), &v.image)?;
    Ok(true)
}
