//! Bake an effect for hosts to play without drawing it: `<name>.bake.json` and its pages `<name>.bake.<n>.png`, every
//! frame as live hosts draw it (`loadBaked` in `pixfx-web` takes them). `<name>` is the id with `/` as `_`.
//!
//!   cargo run -p pixfx-export --example bake -- <id> [--seed N] [--palette WORD] [--num NAME=V] [--out DIR]
//!
//! `--out` defaults to `target/out`.

mod common;

use common::{Args, effect, exit, file_name};
use pixfx_export::{bake, write_bake};

const USAGE: &str = "usage: bake <id> [--seed N] [--palette WORD] [--num NAME=V] [--out DIR]";

fn main() -> std::process::ExitCode {
    exit(run())
}

fn run() -> Result<bool, String> {
    let mut args = Args::new(USAGE);
    let opts = args.look()?;
    let out = args.out("target/out")?;
    let id = args.id()?;
    let clip = bake(effect(&id)?, opts)?;
    let files = write_bake(&clip, &out, &file_name(&id))
        .map_err(|e| format!("writing the {id} bake: {e}"))?;
    println!(
        "{id}: {} ({} pages)",
        files.json.display(),
        files.pages.len()
    );
    Ok(true)
}
