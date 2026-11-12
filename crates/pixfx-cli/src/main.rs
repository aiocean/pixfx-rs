//! `pixfx`: tools for pixfx effects.
//!
//!   pixfx parity <id> [--pixelfx DIR] [--out DIR]
//!
//! Renders `<id>` with pixelfx (through `tools/pixelfx-ref.mjs`, under node) and with its pixfx port, prints
//! how many pixels differ and whether that passes, and writes `<out>/<id>.side.gif` (pixelfx left, pixfx
//! right). The pixelfx dir is `--pixelfx`, else `$PIXELFX`, else `./pixelfx`; `--out` defaults to
//! `target/parity`. Exits 0 on PASS, 1 on FAIL, 2 when the check could not run.
#![forbid(unsafe_code)]

use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

use pixfx_cli::parity::{compare, read_reference, write_side_gif};
use pixfx_core::{Options, render_frames};

const USAGE: &str = "usage: pixfx parity <id> [--pixelfx DIR] [--out DIR]";

/// The reference script, found from this crate's source so `pixfx` runs from any directory of the repo.
const REF_SCRIPT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../tools/pixelfx-ref.mjs");

struct ParityArgs {
    id: String,
    pixelfx: PathBuf,
    out: PathBuf,
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let parsed = match args.split_first() {
        Some((cmd, rest)) if cmd == "parity" => parse_parity(rest),
        _ => Err(USAGE.to_string()),
    };
    match parsed.and_then(|a| parity(&a)) {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::from(1),
        Err(e) => {
            eprintln!("pixfx: {e}");
            ExitCode::from(2)
        }
    }
}

fn parse_parity(args: &[String]) -> Result<ParityArgs, String> {
    let mut id = None;
    let mut pixelfx =
        std::env::var_os("PIXELFX").map_or_else(|| PathBuf::from("pixelfx"), PathBuf::from);
    let mut out = PathBuf::from("target/parity");
    let mut it = args.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--pixelfx" => pixelfx = it.next().ok_or(USAGE)?.into(),
            "--out" => out = it.next().ok_or(USAGE)?.into(),
            flag if flag.starts_with("--") => return Err(format!("unknown flag {flag}\n{USAGE}")),
            _ if id.is_none() => id = Some(a.clone()),
            _ => return Err(USAGE.to_string()),
        }
    }
    Ok(ParityArgs {
        id: id.ok_or(USAGE)?,
        pixelfx,
        out,
    })
}

/// Run the parity check; `Ok(pass)` once it ran.
fn parity(a: &ParityArgs) -> Result<bool, String> {
    let id = a.id.as_str();
    let effect = pixfx_effects::effect(id)
        .ok_or_else(|| format!("no pixfx effect {id} (not ported yet?)"))?;
    reference_render(a)?;
    let reference =
        read_reference(&a.out, id).map_err(|e| format!("reading the {id} reference: {e}"))?;
    let port = render_frames(effect, &Options::default(), 0);
    let report = compare(&reference, &port);
    if (reference.w, reference.h, reference.frames.len()) != (port.w, port.h, port.frames.len()) {
        eprintln!(
            "{id}: pixelfx renders {} frames of {}x{}, pixfx {} of {}x{}",
            reference.frames.len(),
            reference.w,
            reference.h,
            port.frames.len(),
            port.w,
            port.h
        );
    }
    let gif = a.out.join(format!("{id}.side.gif"));
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

/// Write pixelfx's render of `a.id` into `a.out` with the reference script (its progress line is dropped).
fn reference_render(a: &ParityArgs) -> Result<(), String> {
    let run = Command::new("node")
        .arg(Path::new(REF_SCRIPT))
        .arg(&a.id)
        .arg("--pixelfx")
        .arg(&a.pixelfx)
        .arg("--out")
        .arg(&a.out)
        .output()
        .map_err(|e| format!("running node: {e}"))?;
    if run.status.success() {
        Ok(())
    } else {
        Err(format!(
            "pixelfx reference render of {} failed ({}):\n{}",
            a.id,
            run.status,
            String::from_utf8_lossy(&run.stderr).trim_end()
        ))
    }
}
