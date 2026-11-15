//! Render an effect as a looping gif, each pixel blown up `--scale` times (default 4): `<name>.gif`, `<name>` being
//! the id with `/` as `_`.
//!
//!   cargo run -p pixfx-export --example gif -- <id> [--scale N] [--seed N] [--palette WORD]
//!       [--actor PNG,PNG... [--actor-every N]] [--out DIR]
//!
//! `--actor` plays the effect on a character, for an effect that reads the sprite it plays on: the PNGs are the
//! sprite's frames, each standing with the middle of its bottom row on the effect's origin and shown for
//! `--actor-every` render frames (default 6) in turn. The gif shows what the effect draws, not the sprite: an effect
//! that draws round the sprite shows a hole where it stands. `--out` defaults to `target/out`. For anything the flags do not cover (an effect's own option, a glow margin),
//! change the `Options` below or copy this file.

mod common;

use common::{Args, effect, exit, file_name};
use pixfx_core::render_frames_with_actor;
use pixfx_export::parity::Actors;
use pixfx_export::write_gif;

const USAGE: &str = "usage: gif <id> [--scale N] [--seed N] [--palette WORD] [--actor PNG,PNG... [--actor-every N]] [--out DIR]";

fn main() -> std::process::ExitCode {
    exit(run())
}

fn run() -> Result<bool, String> {
    let mut args = Args::new(USAGE);
    let scale = args.count("--scale", 4)?;
    let every = args.count("--actor-every", 6)?;
    let actor = args
        .value("--actor")?
        .map(|list| Actors::read(&list.split(',').collect::<Vec<_>>(), u64::from(every)))
        .transpose()?;
    let opts = args.look()?;
    let out = args.out("target/out")?;
    let id = args.id()?;
    let path = out.join(format!("{}.gif", file_name(&id)));
    let rendered = render_frames_with_actor(effect(&id)?, &opts, 0, |k| {
        actor.as_ref().and_then(|a| a.at(k))
    });
    write_gif(&path, &rendered, scale).map_err(|e| format!("writing {}: {e}", path.display()))?;
    println!("{id}: {}", path.display());
    Ok(true)
}
