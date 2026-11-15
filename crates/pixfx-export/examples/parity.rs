//! The parity gate for one effect: render it with its source and with its pixfx port, print how many pixels differ
//! and whether that passes, and write `<out>/<id>.side.gif` (source left, port right). Exits 0 on PASS, 1 on FAIL,
//! 2 when the check could not run.
//!
//!   cargo run -p pixfx-export --example parity -- <id> [--opt key=value]... [--actor PNG,PNG... [--actor-every N]]
//!       [--pixelfx DIR] [--out DIR]
//!
//! The source is pixelfx, rendered through `tools/pixelfx-ref.mjs` under node; the pixelfx dir is `--pixelfx`, else
//! `$PIXELFX`, else `./pixelfx`. An effect ported from Sprite Fusion has its source vendored at
//! `tools/spritefusion/<id>.ts` instead and is checked against it (through `tools/spritefusion-ref.mjs`). Each
//! `--opt key=value` sets an option on both sides: `seed` (a whole number), `flip` (0 or 1), or a numeric option of
//! the effect (`hits=2`). `--actor` plays the effect on a character, for an effect that wraps one: the PNGs are the
//! sprite's frames, each standing with the middle of its bottom row on the effect's origin and shown for
//! `--actor-every` render frames (default 6) in turn; only a Sprite Fusion source takes one. `--out` defaults to
//! `target/parity`.

mod common;

use std::path::{Path, PathBuf};
use std::process::Command;

use common::{Args, effect, exit};
use pixfx_core::{Effect, render_frames, render_frames_with_actor};
use pixfx_export::parity::{Actors, ParityOpts, compare, read_reference, write_side_gif};

const USAGE: &str = "usage: parity <id> [--opt key=value]... [--actor PNG,PNG... [--actor-every N]] [--pixelfx DIR] [--out DIR]";

/// The reference scripts and the vendored Sprite Fusion sources, found from this crate so the script runs from any
/// directory of the repo.
const REF_SCRIPT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../tools/pixelfx-ref.mjs");
const SF_REF_SCRIPT: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../tools/spritefusion-ref.mjs"
);
const SF_SOURCES: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../tools/spritefusion");

struct Check {
    id: String,
    pixelfx: PathBuf,
    out: PathBuf,
    opts: ParityOpts,
    actor: Option<Actors>,
}

fn main() -> std::process::ExitCode {
    exit(parse().and_then(|c| run(&c)))
}

fn parse() -> Result<Check, String> {
    let mut args = Args::new(USAGE);
    let opts = ParityOpts::parse(&args.values("--opt")?)?;
    let every = args.count("--actor-every", 6)?;
    let actor = args
        .value("--actor")?
        .map(|list| Actors::read(&list.split(',').collect::<Vec<_>>(), u64::from(every)))
        .transpose()?;
    let pixelfx = match args.value("--pixelfx")? {
        Some(dir) => PathBuf::from(dir),
        None => std::env::var_os("PIXELFX").map_or_else(|| PathBuf::from("pixelfx"), PathBuf::from),
    };
    let out = args.out("target/parity")?;
    Ok(Check {
        id: args.id()?,
        pixelfx,
        out,
        opts,
        actor,
    })
}

/// Run the check; `Ok(pass)` once it ran.
fn run(c: &Check) -> Result<bool, String> {
    let id = c.id.as_str();
    let fx = effect(id)?;
    reference_render(c, fx)?;
    let port = match &c.actor {
        Some(actors) => render_frames_with_actor(fx, &c.opts.options(), 0, |k| actors.at(k)),
        None => render_frames(fx, &c.opts.options(), 0),
    };
    let reference = read_reference(&c.out, id, port.origin)
        .map_err(|e| format!("reading the {id} reference: {e}"))?;
    let report = compare(&reference, &port);
    if (reference.w, reference.h, reference.frames.len()) != (port.w, port.h, port.frames.len()) {
        eprintln!(
            "{id}: the source renders {} frames of {}x{}, pixfx {} of {}x{}",
            reference.frames.len(),
            reference.w,
            reference.h,
            port.frames.len(),
            port.w,
            port.h
        );
    }
    let gif = c.out.join(format!("{id}.side.gif"));
    if let Some(dir) = gif.parent() {
        std::fs::create_dir_all(dir).map_err(|e| format!("creating {}: {e}", dir.display()))?;
    }
    write_side_gif(&gif, &reference, &port)
        .map_err(|e| format!("writing {}: {e}", gif.display()))?;
    println!(
        "{id}: colour {:.2}% differ, light {:.2}% differ → {}",
        report.colour_diff_frac * 100.0,
        report.light_diff_frac * 100.0,
        if report.pass() { "PASS" } else { "FAIL" }
    );
    Ok(report.pass())
}

/// Write the reference render of `c.id` into `c.out` (its progress line is dropped): Sprite Fusion's source when one
/// is vendored for it, else pixelfx's. A Sprite Fusion source has no meta, so it is handed the port's canvas, origin,
/// fps, frame count and default seed.
fn reference_render(c: &Check, fx: &'static dyn Effect) -> Result<(), String> {
    let sf_source = Path::new(SF_SOURCES).join(format!("{}.ts", c.id));
    let mut cmd = Command::new("node");
    if sf_source.is_file() {
        let meta = fx.meta();
        let canvas = fx.canvas(&c.opts.options());
        cmd.arg(Path::new(SF_REF_SCRIPT))
            .arg(&c.id)
            .arg("--canvas")
            .arg(format!(
                r#"{{"w":{},"h":{},"ox":{},"oy":{},"fps":{},"n":{},"seed":{}}}"#,
                canvas.size.0,
                canvas.size.1,
                canvas.origin.0,
                canvas.origin.1,
                meta.fps,
                meta.frames(),
                meta.seed
            ));
        if let Some(actors) = &c.actor {
            let file = c.out.join(format!("{}.actor.json", c.id.replace('/', "_")));
            std::fs::write(&file, actors.to_json())
                .map_err(|e| format!("writing {}: {e}", file.display()))?;
            cmd.arg("--actor").arg(file);
        }
    } else if c.actor.is_some() {
        return Err(format!(
            "--actor: {} is checked against pixelfx, which plays no actor",
            c.id
        ));
    } else {
        cmd.arg(Path::new(REF_SCRIPT))
            .arg(&c.id)
            .arg("--pixelfx")
            .arg(&c.pixelfx);
    }
    let run = cmd
        .arg("--out")
        .arg(&c.out)
        .arg("--opts")
        .arg(c.opts.to_json())
        .output()
        .map_err(|e| format!("running node: {e}"))?;
    if run.status.success() {
        Ok(())
    } else {
        Err(format!(
            "reference render of {} failed ({}):\n{}",
            c.id,
            run.status,
            String::from_utf8_lossy(&run.stderr).trim_end()
        ))
    }
}
