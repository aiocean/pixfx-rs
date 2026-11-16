//! An effect beside the sheet it was drawn from: one row of the sheet per colourway, the effect's frames at the
//! sheet's frame times under it, and where they differ, zoomed; prints each frame's overlap and colour error.
//!
//!   cargo run -p pixfx-export --example sheet_compare -- <id> --sheet <png> [--rows 0,5] [--cell 64]
//!       [--frame-ms 60] [--zoom 4] [--option variant] [--anchor X,Y] [--num NAME=V] [--out PNG]
//!
//! The sheet holds frames left to right, `--cell` pixels square, one row per colourway; row r is rendered with the
//! effect's `--option` (default `variant`) set to r. The cell's point `--anchor` (default its centre; the feet of
//! art that stands on the ground) sits on the effect's origin. Only the colour
//! layer is compared (the sheet has no light). Per frame: `iou`, painted pixels both share over painted pixels
//! either has; `err`, the mean channel difference over the cell (transparent counts as black), 0 to 1. Strips:
//! the sheet, the effect, and the difference (grey both painted, red the sheet only, green the effect only).

mod common;

use common::{Args, effect, exit};
use pixfx_core::{OptValue, render_frames};
use pixfx_draw::color::Rgba;
use pixfx_export::{decode_png_rgba, write_png_rgba};

const USAGE: &str = "usage: sheet_compare <id> --sheet <png> [--rows 0,5] [--cell 64] [--frame-ms 60] [--zoom 4] [--option variant] [--anchor X,Y] [--num NAME=V] [--out PNG]";
const BG: [u8; 4] = [24, 24, 24, 255];

fn main() -> std::process::ExitCode {
    exit(run())
}

fn run() -> Result<bool, String> {
    let mut args = Args::new(USAGE);
    let rows: Vec<u32> = match args.value("--rows")? {
        Some(v) => v
            .split(',')
            .map(|r| {
                r.trim()
                    .parse()
                    .map_err(|_| format!("--rows expects numbers: {v}"))
            })
            .collect::<Result<_, _>>()?,
        None => vec![0, 5],
    };
    let cell = args.count("--cell", 64)?;
    let frame_ms = args.count("--frame-ms", 60)?;
    let zoom = args.count("--zoom", 4)?;
    let option = args
        .value("--option")?
        .unwrap_or_else(|| "variant".to_string());
    let anchor: Option<(i32, i32)> = match args.value("--anchor")? {
        Some(v) => {
            let (x, y) = v
                .split_once(',')
                .ok_or(format!("--anchor expects X,Y: {v}"))?;
            Some((
                x.trim()
                    .parse()
                    .map_err(|_| format!("--anchor expects X,Y: {v}"))?,
                y.trim()
                    .parse()
                    .map_err(|_| format!("--anchor expects X,Y: {v}"))?,
            ))
        }
        None => None,
    };
    let out = args.value("--out")?;
    let base = args.look()?;
    let sheet_path = args
        .value("--sheet")?
        .ok_or_else(|| format!("--sheet is required\n{USAGE}"))?;
    let id = args.id()?;
    let fx = effect(&id)?;
    let bytes = std::fs::read(&sheet_path).map_err(|e| format!("reading {sheet_path}: {e}"))?;
    let sheet = decode_png_rgba(&bytes)?;
    let n = sheet.w / cell;
    let (w, h) = (n * cell * zoom, rows.len() as u32 * 3 * cell * zoom);
    let mut img = vec![0u8; (w * h * 4) as usize];
    let mut total = (0.0, 0.0, 0u32);
    for (ri, &row) in rows.iter().enumerate() {
        let mut opts = base.clone();
        opts.extra
            .insert(option.clone(), OptValue::Num(f64::from(row)));
        let r = render_frames(fx, &opts, 0);
        let (ax, ay) = anchor.unwrap_or((cell as i32 / 2, cell as i32 / 2));
        let (cx, cy) = (r.origin.0 - ax, r.origin.1 - ay);
        let mut line = String::new();
        for k in 0..n {
            let fi = (u64::from(k) * u64::from(frame_ms) * u64::from(r.fps) + 500) / 1000;
            let frame = r.frames.get(fi as usize);
            let (mut both, mut either, mut err) = (0u32, 0u32, 0.0f64);
            for y in 0..cell {
                for x in 0..cell {
                    let s = sheet.get(k * cell + x, row * cell + y).unwrap_or([0; 4]);
                    let (ex, ey) = (cx + x as i32, cy + y as i32);
                    let e = frame
                        .filter(|f| ex >= 0 && ey >= 0 && (ex as u32) < f.w && (ey as u32) < f.h)
                        .and_then(|f| f.color.get((ey as u32 * f.w + ex as u32) as usize))
                        .map(|&c| {
                            let c = Rgba::unpack(c);
                            [c.0, c.1, c.2, c.3]
                        })
                        .unwrap_or([0; 4]);
                    let (sa, ea) = (s[3] > 0, e[3] > 0);
                    both += u32::from(sa && ea);
                    either += u32::from(sa || ea);
                    let ch = |p: [u8; 4], i: usize| if p[3] > 0 { f64::from(p[i]) } else { 0.0 };
                    err += (0..3).map(|i| (ch(s, i) - ch(e, i)).abs()).sum::<f64>() / (3.0 * 255.0);
                    let d = match (sa, ea) {
                        (true, true) => [140, 140, 140, 255],
                        (true, false) => [220, 50, 50, 255],
                        (false, true) => [50, 200, 70, 255],
                        _ => BG,
                    };
                    let over = |p: [u8; 4]| if p[3] > 0 { p } else { BG };
                    for (strip, px) in [(0, over(s)), (1, over(e)), (2, d)] {
                        let oy = (ri as u32 * 3 + strip) * cell + y;
                        for zy in 0..zoom {
                            for zx in 0..zoom {
                                let i = (((oy * zoom + zy) * w + (k * cell + x) * zoom + zx) * 4)
                                    as usize;
                                if let Some(dst) = img.get_mut(i..i + 4) {
                                    dst.copy_from_slice(&px);
                                }
                            }
                        }
                    }
                }
            }
            let iou = if either > 0 {
                f64::from(both) / f64::from(either)
            } else {
                1.0
            };
            let err = err / f64::from(cell * cell);
            total = (total.0 + iou, total.1 + err, total.2 + 1);
            line.push_str(&format!(" {k}:{iou:.2}/{err:.3}"));
        }
        println!("row {row} iou/err{line}");
    }
    if total.2 > 0 {
        let c = f64::from(total.2);
        println!("mean iou {:.3} err {:.4}", total.0 / c, total.1 / c);
    }
    if let Some(path) = out {
        // a 1-pixel frame line between cells, so frames read apart when zoomed
        for k in 1..n {
            for y in 0..h {
                let i = ((y * w + k * cell * zoom) * 4) as usize;
                if let Some(dst) = img.get_mut(i..i + 4) {
                    dst.copy_from_slice(&[70, 70, 90, 255]);
                }
            }
        }
        write_png_rgba(std::path::Path::new(&path), w, h, &img)
            .map_err(|e| format!("writing {path}: {e}"))?;
        println!("side by side: {path}");
    }
    Ok(true)
}
