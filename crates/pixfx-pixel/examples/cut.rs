//! Cut the cells an image model drew into a sheet (laid out by the `sheet` script's `sheet.json`) back out, each down
//! to one pixel a cell: `<out>/cell-<col>-<row>.png`.
//!
//!   cargo run -p pixfx-pixel --example cut -- <drawn.png> <sheet.json> [--out DIR]

mod common;

use common::{Args, exit, read_image, read_text, write_image};
use pixfx_pixel::sheet::parse_layout;
use pixfx_pixel::tool::cut;

const USAGE: &str = "usage: cut <drawn.png> <sheet.json> [--out DIR]";

fn main() -> std::process::ExitCode {
    exit(run())
}

fn run() -> Result<bool, String> {
    let mut args = Args::new(USAGE);
    let out = args.out()?;
    let [drawn, layout] = args.exactly()?;
    let layout = parse_layout(&read_text(&layout)?)?;
    let cols = layout.cols.max(1);
    for (i, cell) in cut(&read_image(&drawn)?, &layout).iter().enumerate() {
        let (c, r) = (i as u32 % cols, i as u32 / cols);
        write_image(&out.join(format!("cell-{c}-{r}.png")), cell)?;
    }
    Ok(true)
}
