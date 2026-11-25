//! Every pose of a fighter look on one sheet, painted as the duel paints it, 8 to a row in the order of
//! `pixfx_fighter::poses::POSES`: `<out>/<look>-poses.png`. `--blank` leaves the named poses' cells empty (a
//! reference sheet for an artist or an image model to fill). `--pose P` writes `<out>/<look>-<P>.png` instead: the
//! pose at three moments, and with `--states` one row per state (out, facing, flash, ice, shock), labelled in the
//! output; each cell is also written alone, `<out>/<look>-<P>/r<row>-m<moment>.png`.
//!
//!   cargo run -p pixfx-fighter --example poses -- <thunder|frost|alchemist> [--blank POSE,...] [--pose POSE [--states]]
//!       [--pixel-density 1|2] [--out DIR]
//!
//! `--out` defaults to `out`.

use std::path::PathBuf;
use std::process::ExitCode;

use pixfx_draw::PixelDensity;
use pixfx_export::{Grid, pack_cells, write_png_rgba};
use pixfx_fighter::look::Look;
use pixfx_fighter::paint::SPRITE;
use pixfx_fighter::pose::Pose;
use pixfx_fighter::poses::{
    MOMENTS, POSES, State, Tint, pose_cells_at_density, pose_named, state_cells_at_density, states,
};

const USAGE: &str = "usage: poses <thunder|frost|alchemist> [--blank POSE,...] [--pose POSE [--states]] [--pixel-density 1|2] [--out DIR]";

fn main() -> ExitCode {
    match run(std::env::args().skip(1).collect()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("{e}");
            ExitCode::from(2)
        }
    }
}

fn named(name: &str) -> Result<Pose, String> {
    let names = POSES.map(|(n, _)| n).join(", ");
    pose_named(name).ok_or(format!("no pose {name}: one of {names}"))
}

fn run(args: Vec<String>) -> Result<(), String> {
    let mut look = None;
    let mut blank = Vec::new();
    let (mut one, mut with_states) = (None, false);
    let mut out = PathBuf::from("out");
    let mut density = PixelDensity::Two;
    let mut it = args.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--blank" => {
                for name in it.next().ok_or(USAGE)?.split(',') {
                    blank.push(named(name)?);
                }
            }
            "--pose" => {
                let name = it.next().ok_or(USAGE)?;
                one = Some((name.clone(), named(name)?));
            }
            "--states" => with_states = true,
            "--pixel-density" => {
                density = match it.next().map(String::as_str) {
                    Some("1") => PixelDensity::One,
                    Some("2") => PixelDensity::Two,
                    _ => return Err("--pixel-density must be 1 or 2".into()),
                };
            }
            "--out" => out = it.next().ok_or(USAGE)?.into(),
            flag if flag.starts_with("--") => return Err(format!("unknown flag {flag}\n{USAGE}")),
            _ if look.is_none() => look = Some(a.clone()),
            _ => return Err(USAGE.to_string()),
        }
    }
    let name = look.ok_or(USAGE)?;
    let look = Look::preset(&name).ok_or(format!(
        "no fighter look {name}: thunder, frost or alchemist"
    ))?;
    std::fs::create_dir_all(&out).map_err(|e| format!("creating {}: {e}", out.display()))?;
    let side = SPRITE as u32 * density.factor();
    let (cells, grid, path) = match &one {
        Some((pose_name, pose)) => {
            let rows = if with_states {
                states()
            } else {
                vec![State {
                    out: false,
                    face: 1.0,
                    tint: Tint::Own,
                }]
            };
            for (i, s) in rows.iter().enumerate() {
                println!("row {i}: {}", s.label());
            }
            let cells = state_cells_at_density(&look, *pose, &rows, density);
            let grid = Grid {
                frames: cells.len() as u32,
                columns: MOMENTS.len() as u32,
                rows: rows.len() as u32,
                cell_w: side,
                cell_h: side,
            };
            (cells, grid, out.join(format!("{name}-{pose_name}.png")))
        }
        None if with_states => return Err(format!("--states needs --pose\n{USAGE}")),
        None => {
            let cells = pose_cells_at_density(&look, &blank, density);
            let grid = Grid::new(cells.len() as u32, side, side);
            (cells, grid, out.join(format!("{name}-poses.png")))
        }
    };
    write_png_rgba(
        &path,
        grid.width(),
        grid.height(),
        &pack_cells(&grid, &cells, 4),
    )
    .map_err(|e| format!("writing {}: {e}", path.display()))?;
    println!("{name}: {}", path.display());
    if let Some((pose_name, _)) = one {
        // Each cell on its own too, a sprite the pixfx-pixel `diff` and `critique` scripts take as it is.
        let dir = out.join(format!("{name}-{pose_name}"));
        std::fs::create_dir_all(&dir).map_err(|e| format!("creating {}: {e}", dir.display()))?;
        for (i, cell) in cells.iter().enumerate() {
            let (row, moment) = (i / MOMENTS.len(), i % MOMENTS.len());
            let cell_path = dir.join(format!("r{row}-m{moment}.png"));
            write_png_rgba(&cell_path, side, side, cell)
                .map_err(|e| format!("writing {}: {e}", cell_path.display()))?;
        }
        println!("cells: {}/r<row>-m<moment>.png", dir.display());
    }
    Ok(())
}
