//! One catalog clip as a sheet: a row per direction, a column per frame. Writes into `--out` (default
//! `target/body/<clip>`): `composite.png` (hull with bones), `hull.png`, `bones.png`, `parts.png` (part map in
//! guide colours), `<clip>.gif` (the first direction, 4x) and `<clip>.json` (cells, timing, root motion, joints).
//!
//!   cargo run -p pixfx-core --features body --example clip_sheet -- <clip> [--camera side|iso|topdown] [--out DIR]

mod common;

use std::path::PathBuf;
use std::process::ExitCode;

use pixfx_core::body::{Body, View, catalog};

const USAGE: &str = "usage: clip_sheet <clip> [--camera side|iso|topdown] [--out DIR]";

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("clip_sheet: {e}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), String> {
    let mut args = std::env::args().skip(1);
    let (mut name, mut view, mut out) = (None, View::Side, None);
    while let Some(a) = args.next() {
        match a.as_str() {
            "--camera" => view = common::view(&args.next().ok_or(USAGE)?)?,
            "--out" => out = Some(PathBuf::from(args.next().ok_or(USAGE)?)),
            n if !n.starts_with("--") && name.is_none() => name = Some(n.to_string()),
            _ => return Err(USAGE.into()),
        }
    }
    let name = name.ok_or(USAGE)?;
    let clip = catalog::by_name(&name)
        .ok_or_else(|| format!("{name}: not in the catalog ({})", catalog::NAMES.join(", ")))?;
    let out = out.unwrap_or_else(|| PathBuf::from("target/body").join(name.replace('/', "_")));
    common::write_clip(&Body::default(), &clip, view, &out)?;
    println!("{}", out.display());
    Ok(())
}
