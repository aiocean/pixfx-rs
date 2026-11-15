//! Render an effect as an mp4, each pixel blown up `--scale` times (default 4), the loop played `--loops` times
//! (default 3): `<name>.mp4`, `<name>` being the id with `/` as `_`. Needs ffmpeg on `PATH`.
//!
//!   cargo run -p pixfx-export --example mp4 -- <id> [--scale N] [--loops N] [--seed N] [--palette WORD] [--out DIR]
//!
//! `--out` defaults to `target/out`.

mod common;

use common::{Args, effect, exit, file_name};
use pixfx_core::render_frames;
use pixfx_export::write_video;

const USAGE: &str =
    "usage: mp4 <id> [--scale N] [--loops N] [--seed N] [--palette WORD] [--out DIR]";

fn main() -> std::process::ExitCode {
    exit(run())
}

fn run() -> Result<bool, String> {
    let mut args = Args::new(USAGE);
    let scale = args.count("--scale", 4)?;
    let loops = args.count("--loops", 3)?;
    let opts = args.look()?;
    let out = args.out("target/out")?;
    let id = args.id()?;
    let path = out.join(format!("{}.mp4", file_name(&id)));
    write_video(&path, &render_frames(effect(&id)?, &opts, 0), scale, loops)
        .map_err(|e| format!("writing {}: {e}", path.display()))?;
    println!("{id}: {}", path.display());
    Ok(true)
}
