//! Step 5: the effect you wrote beside the sheet it was drawn from, with how much each step changes in both.
//!
//!   cargo run -p pixfx-pixel --example compare -- <spec.json> <review-dir> <effect-id> --times "init=0.15,0.35,0.55;hit=1.55,1.64,1.7" [--out FILE] [--side N]
//!
//! `<review-dir>` is what `extract` wrote (it reads `<row>-<n>.rebuilt.png`, the sheet's frames over black).
//! `--times` gives, for each moment row to compare, the effect's time in seconds of each of its drawn frames, in
//! order; rows left out are skipped. Renders the effect, cuts the cell window round its origin at each time
//! (`pixfx_pixel::reference::compare`), and writes `--out` (default `<review-dir>/compare.png`: per row the sheet's cells over the
//! effect's frames, `--side` pixels each, default 192) and `compare.json`; prints each row's step changes
//! (`pixfx_pixel::reference::entropy`) for the sheet and the effect.

mod common;

use std::path::PathBuf;

use common::{Args, exit_reference, read_image, read_spec, write_png, write_reference_text};
use pixfx_core::{Options, render_frames};
use pixfx_draw::composite;
use pixfx_pixel::reference::RgbaImage;
use pixfx_pixel::reference::compare::{Pair, board, effect_cell};
use pixfx_pixel::reference::entropy::steps;
use pixfx_pixel::reference::extract::name_of;
use serde_json::json;

const USAGE: &str = "usage: compare <spec.json> <review-dir> <effect-id> --times \"row=t1,t2,..;row=..\" [--out FILE] [--side N]";

fn main() -> std::process::ExitCode {
    exit_reference("compare", run())
}

/// `row=t1,t2;row=t3` as (row in lower case with dashes, times).
fn parse_times(s: &str) -> Result<Vec<(String, Vec<f64>)>, String> {
    s.split(';')
        .filter(|p| !p.trim().is_empty())
        .map(|p| {
            let (row, ts) = p
                .split_once('=')
                .ok_or_else(|| format!("--times: `{p}` is not row=t1,t2,..."))?;
            let ts = ts
                .split(',')
                .map(|t| {
                    t.trim()
                        .parse::<f64>()
                        .map_err(|_| format!("--times: `{t}` is not a time in seconds"))
                })
                .collect::<Result<Vec<_>, _>>()?;
            Ok((name_of(row), ts))
        })
        .collect()
}

fn run() -> Result<(), String> {
    let mut args = Args::new(USAGE);
    let times = parse_times(
        &args
            .value("--times")?
            .ok_or("--times is needed: the effect's time of each drawn frame")?,
    )?;
    let out = args.value("--out")?.map(PathBuf::from);
    let side: u32 = match args.value("--side")? {
        Some(v) => v
            .parse()
            .map_err(|_| format!("--side {v}: not a whole number"))?,
        None => 192,
    };
    let rest = args.rest(3..=3)?;
    let spec = read_spec(&PathBuf::from(&rest[0]))?;
    let dir = PathBuf::from(&rest[1]);
    let fx = pixfx_effects::effect(&rest[2]).ok_or_else(|| format!("no effect {}", rest[2]))?;
    let r = render_frames(fx, &Options::default(), 0);
    let fps = f64::from(fx.meta().fps.max(1));
    let frame_at = |t: f64| -> Option<RgbaImage> {
        let k = (libm::rint(t * fps).max(0.0) as usize).min(r.frames.len().checked_sub(1)?);
        let f = r.frames.get(k)?;
        let bg: Vec<u8> = (0..f.w * f.h).flat_map(|_| [0, 0, 0, 255]).collect();
        let mut px = vec![0u8; (f.w * f.h * 4) as usize];
        composite(f, &mut px, Some(&bg));
        Some(RgbaImage {
            w: f.w,
            h: f.h,
            rgba: px,
        })
    };
    let mut pairs = Vec::new();
    let mut report = Vec::new();
    for row in &spec.rows {
        let key = name_of(&row.name);
        let Some((_, ts)) = times.iter().find(|(k, _)| *k == key) else {
            continue;
        };
        let n = spec.frames_of(row) as usize;
        if ts.len() != n {
            return Err(format!(
                "--times {key}: {} times for {n} drawn frames",
                ts.len()
            ));
        }
        let sheet = (1..=n)
            .map(|k| read_image(&dir.join(format!("{key}-{k}.rebuilt.png"))))
            .collect::<Result<Vec<_>, _>>()?;
        let effect = ts
            .iter()
            .zip(&sheet)
            .map(|(&t, s)| {
                frame_at(t)
                    .map(|f| effect_cell(&f, r.origin, s.w, s.h))
                    .ok_or_else(|| format!("{}: no frame at {t}s", rest[2]))
            })
            .collect::<Result<Vec<_>, _>>()?;
        let (a, b) = (steps(&sheet)?, steps(&effect)?);
        let fmt = |v: &[f64]| {
            v.iter()
                .map(|x| format!("{x:.3}"))
                .collect::<Vec<_>>()
                .join(" ")
        };
        println!("{key:<10} sheet {}  | effect {}", fmt(&a), fmt(&b));
        report.push(json!({"row": key, "times": ts, "sheet_steps": a, "effect_steps": b}));
        pairs.push(Pair { sheet, effect });
    }
    if pairs.is_empty() {
        return Err("no row of --times matches a moment row of the spec".into());
    }
    let out = out.unwrap_or_else(|| dir.join("compare.png"));
    write_png(&out, &board(&pairs, side, 8))?;
    write_reference_text(
        &out.with_extension("json"),
        &format!("{:#}\n", json!({"effect": rest[2], "rows": report})),
    )?;
    println!("{}", out.display());
    Ok(())
}
