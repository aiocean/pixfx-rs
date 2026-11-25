//! Every catalog clip as a sheet, into `target/body/<view>/<clip>/` (see `clip_sheet`).
//!
//!   cargo run -p pixfx-core --features body --example catalog -- [--camera side|iso|topdown]

mod common;

use std::path::PathBuf;
use std::process::ExitCode;

use pixfx_core::body::{Body, View, catalog};

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("catalog: {e}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), String> {
    let mut args = std::env::args().skip(1);
    let mut view = View::Side;
    while let Some(a) = args.next() {
        match a.as_str() {
            "--camera" => view = common::view(&args.next().ok_or("--camera needs a value")?)?,
            _ => return Err("usage: catalog [--camera side|iso|topdown]".into()),
        }
    }
    let root = PathBuf::from("target/body").join(view.name());
    for name in catalog::NAMES {
        let Some(clip) = catalog::by_name(name) else {
            continue;
        };
        common::write_clip(
            &Body::default(),
            &clip,
            view,
            &root.join(name.replace('/', "_")),
        )?;
    }
    println!("{} clips in {}", catalog::NAMES.len(), root.display());
    Ok(())
}
