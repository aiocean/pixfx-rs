//! Draw a text grid (as the `read` script writes it, edited or not) back into a sprite: `<out>/<text>.png`.
//!
//!   cargo run -p pixfx-pixel --example draw -- <sprite.txt> [--out DIR]

mod common;

use common::{Args, exit, read_text, stem, write_image};
use pixfx_pixel::tool::draw;

const USAGE: &str = "usage: draw <sprite.txt> [--out DIR]";

fn main() -> std::process::ExitCode {
    exit(run())
}

fn run() -> Result<bool, String> {
    let mut args = Args::new(USAGE);
    let out = args.out()?;
    let [text] = args.exactly()?;
    write_image(
        &out.join(format!("{}.png", stem(&text))),
        &draw(&read_text(&text)?)?,
    )?;
    Ok(true)
}
