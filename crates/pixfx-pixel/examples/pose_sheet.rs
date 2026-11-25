//! A pose sheet from a JSON spec (see `pixfx_pixel::pose::spec`), or every preset with `--presets`. Writes into `--out`
//! (default `target/pose`): `sheet.png` (the template for the image model), `sheet.json` (the grid that
//! `pixfx_pixel::sheet::cut` takes back), `skeletons.json` (each cell's joints in sprite pixels) and `legend.txt`
//! (the colour key to paste into the prompt).
//!
//!   cargo run -p pixfx-pixel --example pose_sheet -- <spec.json> [--out DIR]
//!   cargo run -p pixfx-pixel --example pose_sheet -- --presets [--facing DEG] [--out DIR]

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use pixfx_pixel::pose::{Cell, PoseSheet, legend, presets, spec};

const USAGE: &str = "usage: pose_sheet (<spec.json> | --presets [--facing DEG]) [--out DIR]";

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("pose_sheet: {e}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), String> {
    let mut args = std::env::args().skip(1);
    let (mut file, mut all, mut facing, mut out) = (None, false, 0.0, PathBuf::from("target/pose"));
    while let Some(a) = args.next() {
        match a.as_str() {
            "--presets" => all = true,
            "--out" => out = args.next().ok_or(USAGE)?.into(),
            "--facing" => {
                let v = args.next().ok_or(USAGE)?;
                facing = v
                    .parse()
                    .map_err(|_| format!("--facing {v}: not a number"))?;
            }
            f if !f.starts_with("--") && file.is_none() => file = Some(PathBuf::from(f)),
            _ => return Err(USAGE.into()),
        }
    }
    let sheet = match (file, all) {
        (Some(f), false) => {
            let text = std::fs::read_to_string(&f).map_err(|e| format!("{}: {e}", f.display()))?;
            spec::parse(&text)?
        }
        (None, true) => PoseSheet {
            cells: presets::NAMES
                .iter()
                .filter_map(|&n| {
                    let phase = 0.25;
                    presets::by_name(n, phase).map(|pose| Cell {
                        caption: n.to_uppercase(),
                        pose,
                        facing,
                    })
                })
                .collect(),
            ..PoseSheet::default()
        },
        _ => return Err(USAGE.into()),
    };
    let drawn = sheet.render()?;
    std::fs::create_dir_all(&out).map_err(|e| format!("{}: {e}", out.display()))?;
    let img = &drawn.image;
    write(&out.join("sheet.png"), |p| {
        pixfx_export::write_png_rgba(p, img.w, img.h, &img.rgba).map_err(|e| format!("{e:?}"))
    })?;
    let layout = pixfx_pixel::sheet::layout_json(&drawn.layout);
    write(&out.join("sheet.json"), |p| text(p, &layout))?;
    let joints = serde_json::to_string_pretty(&drawn.skeletons).map_err(|e| e.to_string())?;
    write(&out.join("skeletons.json"), |p| text(p, &joints))?;
    write(&out.join("legend.txt"), |p| text(p, &(legend() + "\n")))?;
    Ok(())
}

fn text(p: &Path, s: &str) -> Result<(), String> {
    std::fs::write(p, s).map_err(|e| e.to_string())
}

fn write(p: &Path, f: impl FnOnce(&Path) -> Result<(), String>) -> Result<(), String> {
    f(p).map_err(|e| format!("{}: {e}", p.display()))?;
    println!("{}", p.display());
    Ok(())
}
