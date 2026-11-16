//! Step 3: cut a drawn sheet into the effect's frames, as colour and light layers, with a review page.
//!
//!   cargo run -p pixfx-ref --example extract -- <spec.json> <sheet.png> [--parts-sheet | --moment NAME] [--out DIR] [--back HREF] [--between N]
//!
//! `--parts-sheet` and `--moment` read the sheets `template` made with them, into `<their folder>/review`.
//!
//! `<sheet.png>` is what the model drew on a transparent background (`generate` writes it as
//! `draw-<n>/image-01.png`). Writes into `--out` (default `target/ref/<slug>/review`): per frame
//! `<row>-<n>.{orig,grid,alpha,color,light,rebuilt,palette}.png`, the frame diffs, `report.json`, `matrix.json` (every frame as a pixel matrix with the shared palette), `entry.json` (for the
//! gallery), `sheet-black.png` and `index.html`. Open `index.html`: the rebuilt column must match the original.
//! `--between` is how many frames the page's player makes between each two drawn frames of a row (default
//! [`BETWEEN`]; 0 for none). `--back` is the page's link to the gallery (default `../../index.html#<key>`, the `gallery` script's default
//! place).

mod common;

use std::path::PathBuf;

use common::{Args, exit, key, read_png, read_spec, sheet_of, write_png, write_text};
use pixfx_pixel::frames::{busiest, grid_view};
use pixfx_ref::RgbaImage;
use pixfx_ref::extract::extract;
use pixfx_ref::matrix::{matrix, paletted, to_json};
use pixfx_ref::review::{diff_file, effect_page, file, report};
use pixfx_ref::tween::tweens;
use serde_json::json;

/// Sheet pixels across the busiest square of a cell, shown with its cuts 3 times bigger.
const VIEW: u32 = 64;
/// The colour the cuts are drawn in.
const CUT: [u8; 4] = [0, 255, 255, 255];

/// In-betweens per two drawn frames, by default: 7 lets the player show 1, 3 or 7.
const BETWEEN: usize = 7;

const USAGE: &str = "usage: extract <spec.json> <sheet.png> [--parts-sheet | --moment NAME] [--out DIR] [--back HREF] [--between N]";

fn main() -> std::process::ExitCode {
    exit("extract", run())
}

/// `img` over black.
fn over_black(img: &RgbaImage) -> RgbaImage {
    let mut out = img.clone();
    for p in out.rgba.as_chunks_mut::<4>().0 {
        let a = u32::from(p[3]);
        for c in &mut p[..3] {
            *c = (u32::from(*c) * a / 255) as u8;
        }
        p[3] = 255;
    }
    out
}

fn crop(img: &RgbaImage, [x0, y0, x1, y1]: [u32; 4]) -> RgbaImage {
    let mut out = RgbaImage::new(x1 - x0, y1 - y0);
    for y in y0..y1 {
        for x in x0..x1 {
            out.set(x - x0, y - y0, img.get(x, y).unwrap_or([0; 4]));
        }
    }
    out
}

fn run() -> Result<(), String> {
    let mut args = Args::new(USAGE);
    let out = args.value("--out")?.map(PathBuf::from);
    let back = args.value("--back")?;
    let moment = args.value("--moment")?;
    let parts_sheet = args.flag("--parts-sheet");
    let n = match args.value("--between")? {
        Some(v) => v
            .parse::<usize>()
            .map_err(|e| format!("--between {v}: {e}"))?,
        None => BETWEEN,
    };
    let rest = args.rest(2..=2)?;
    let (spec, dir) = sheet_of(
        read_spec(&PathBuf::from(&rest[0]))?,
        parts_sheet,
        moment.as_deref(),
    )?;
    let sheet = read_png(&PathBuf::from(&rest[1]))?;
    let out = out.unwrap_or_else(|| dir.join("review"));
    let k = key(&spec);
    let ex = extract(&sheet, &spec).map_err(|e| format!("{}: {e}", rest[1]))?;
    let black = over_black(&sheet);
    write_png(&out.join("sheet-black.png"), &black)?;
    for c in &ex.cells {
        let orig = crop(&black, c.rect);
        let view = grid_view(&orig, &c.grid, busiest(&orig, VIEW), 3, CUT);
        write_png(&out.join(file(&c.name, "grid")), &view)?;
        write_png(&out.join(file(&c.name, "orig")), &orig)?;
        write_png(&out.join(file(&c.name, "alpha")), &c.alpha)?;
        write_png(&out.join(file(&c.name, "color")), &c.colour)?;
        write_png(&out.join(file(&c.name, "light")), &c.light)?;
        write_png(&out.join(file(&c.name, "rebuilt")), &c.rebuilt)?;
    }
    let m = matrix(&ex);
    for f in &m.frames {
        write_png(&out.join(file(&f.name, "palette")), &paletted(&m, f))?;
    }
    write_text(&out.join("matrix.json"), &format!("{}\n", to_json(&m)))?;
    for d in &ex.diffs {
        write_png(&out.join(diff_file(&d.name)), &d.image)?;
    }
    let rep = report(&ex);
    write_text(&out.join("report.json"), &format!("{rep:#}\n"))?;
    let cells = ex.cells.len().max(1) as f64;
    let entry = json!({
        "key": k, "id": spec.id, "name": spec.name, "look": spec.look,
        "pitch": ex.pitch.0,
        "mean_err": ex.cells.iter().map(|c| c.mean_err).sum::<f64>() / cells,
    });
    write_text(&out.join("entry.json"), &format!("{entry:#}\n"))?;
    let back = back.unwrap_or_else(|| format!("../../index.html#{k}"));
    let between = tweens(&m, n);
    write_text(
        &out.join("index.html"),
        &effect_page(&spec.name, &back, &ex, &m, &between, n),
    )?;
    let (w, h) = ex.size;
    match ex.found {
        true => println!(
            "grid {w}x{h}, pitch {:.3} x {:.3}, fit {:.2} x {:.2}",
            ex.pitch.0, ex.pitch.1, ex.fit.0, ex.fit.1
        ),
        false => println!("no grid found: the template's {w}x{h}"),
    }
    println!(
        "palette {} colour + {} light, missing the exact values by {:.2} and {:.2} per channel",
        m.colours.len(),
        m.lights.len(),
        m.colour_err,
        m.light_err
    );
    for c in &ex.cells {
        println!(
            "{} rect {:?} grid {:.2} x {:.2} colour {} light {} err {:.2}",
            c.name, c.rect, c.strength.0, c.strength.1, c.solid, c.lit, c.mean_err
        );
    }
    println!("{}: {}", spec.id, out.join("index.html").display());
    Ok(())
}
