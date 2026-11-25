//! Render an effect's clip for hosts to play without drawing it: `<name>.clip.json` and its pages `<name>.clip.<n>.png`, every
//! frame as live hosts draw it (`loadClip` in `pixfx-web` takes them). `<name>` is the id with `/` as `_`.
//!
//!   cargo run -p pixfx-export --example render_clip -- <id> [--seed N] [--palette WORD] [--num NAME=V] [--out DIR]
//!
//! `--out` defaults to `target/out`.

mod common;

use common::{Args, effect, exit, file_name};
use pixfx_export::{render_clip, write_clip};

const USAGE: &str =
    "usage: render_clip <id> [--seed N] [--palette WORD] [--num NAME=V] [--out DIR]";

fn main() -> std::process::ExitCode {
    exit(run())
}

fn run() -> Result<bool, String> {
    let mut args = Args::new(USAGE);
    let opts = args.look()?;
    let out = args.out("target/out")?;
    let id = args.id()?;
    let clip = render_clip(effect(&id)?, opts)?;
    let files = write_clip(&clip, &out, &file_name(&id))
        .map_err(|e| format!("writing the {id} clip: {e}"))?;
    println!(
        "{id}: {} ({} pages)",
        files.json.display(),
        files.pages.len()
    );
    Ok(true)
}
