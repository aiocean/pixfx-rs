//! Render an effect as sprite sheets plus beanfx's Aseprite-style JSON: `<name>.png`, `<name>_color.png`,
//! `<name>_light.png` and `<name>.json`, `<name>` being the id with `/` as `_`.
//!
//!   cargo run -p pixfx-export --example sprite_sheet -- <id> [--seed N] [--palette WORD] [--out DIR]
//!
//! `--out` defaults to `target/out`. For anything the flags do not cover (an effect's own option, a glow margin),
//! change the `Options` below or copy this file.

mod common;

use common::{Args, effect, exit, file_name};
use pixfx_core::render_frames;
use pixfx_export::write_sheet;

const USAGE: &str = "usage: sprite_sheet <id> [--seed N] [--palette WORD] [--out DIR]";

fn main() -> std::process::ExitCode {
    exit(run())
}

fn run() -> Result<bool, String> {
    let mut args = Args::new(USAGE);
    let opts = args.look()?;
    let out = args.out("target/out")?;
    let id = args.id()?;
    let fx = effect(&id)?;
    let files = write_sheet(
        &render_frames(fx, &opts, 0),
        fx.meta(),
        &out,
        &file_name(&id),
    )
    .map_err(|e| format!("writing the {id} sheet: {e}"))?;
    println!("{id}: {}", files.json.display());
    Ok(true)
}
