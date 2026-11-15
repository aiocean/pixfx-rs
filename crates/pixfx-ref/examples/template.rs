//! Step 1: the template sheet, its grid, the prompt and the style reference for a spec.
//!
//!   cargo run -p pixfx-ref --example template -- <spec.json> [--out DIR] [--style ID] [--style-frame N]
//!
//! Writes into `--out` (default `target/ref/<slug>`): `template.png` (the labelled cells with the guide figure),
//! `sheet.json` (its grid), `prompt.txt`, and `style.png`: frame `--style-frame` of effect `--style` (default
//! frame 46 of `arcane/hex-shield`, a square round its origin over a night background, drawn 3x), which shows the
//! model pixfx's pixel size and look. Read `prompt.txt` before spending money on it.

mod common;

use std::path::PathBuf;

use common::{Args, exit, home, read_spec, write_png, write_text};
use pixfx_core::{Options, render_frames};
use pixfx_draw::composite;
use pixfx_pixel::sheet::layout_json;
use pixfx_ref::{RgbaImage, prompt::prompt, template};

const USAGE: &str = "usage: template <spec.json> [--out DIR] [--style ID] [--style-frame N]";

/// The background the style frame is shown on, and how big it is drawn.
const NIGHT: [u8; 3] = [18, 18, 30];
const STYLE_SCALE: u32 = 3;

fn main() -> std::process::ExitCode {
    exit("template", run())
}

fn run() -> Result<(), String> {
    let mut args = Args::new(USAGE);
    let out = args.value("--out")?.map(PathBuf::from);
    let style = args.value("--style")?.unwrap_or_else(|| "arcane/hex-shield".into());
    let frame: usize = match args.value("--style-frame")? {
        Some(v) => v.parse().map_err(|_| format!("--style-frame {v}: not a whole number"))?,
        None => 46,
    };
    let rest = args.rest(1..=1)?;
    let spec = read_spec(&PathBuf::from(&rest[0]))?;
    let out = out.unwrap_or_else(|| home(&spec));
    write_png(&out.join("template.png"), &template::render(&spec)?)?;
    write_text(&out.join("sheet.json"), &layout_json(&template::layout(&spec)))?;
    write_text(&out.join("prompt.txt"), &prompt(&spec))?;
    write_png(&out.join("style.png"), &style_frame(&style, frame)?)?;
    println!("{}: {}", spec.id, out.display());
    Ok(())
}

/// A square of frame `k` of effect `id` round its origin, over [`NIGHT`], drawn [`STYLE_SCALE`]x.
fn style_frame(id: &str, k: usize) -> Result<RgbaImage, String> {
    let fx = pixfx_effects::effect(id).ok_or_else(|| format!("no effect {id}"))?;
    let r = render_frames(fx, &Options::default(), 0);
    let f = r.frames.get(k).ok_or_else(|| format!("{id} has {} frames, not {}", r.frames.len(), k + 1))?;
    let bg: Vec<u8> = (0..f.w * f.h).flat_map(|_| [NIGHT[0], NIGHT[1], NIGHT[2], 255]).collect();
    let mut px = vec![0u8; (f.w * f.h * 4) as usize];
    composite(f, &mut px, Some(&bg));
    let side = f.w.min(f.h);
    let x0 = (r.origin.0 - side as i32 / 2).clamp(0, (f.w - side) as i32) as u32;
    let y0 = (r.origin.1 - side as i32 / 2).clamp(0, (f.h - side) as i32) as u32;
    let mut img = RgbaImage::new(side * STYLE_SCALE, side * STYLE_SCALE);
    for y in 0..img.h {
        for x in 0..img.w {
            let i = (((y0 + y / STYLE_SCALE) * f.w + x0 + x / STYLE_SCALE) * 4) as usize;
            img.set(x, y, [px[i], px[i + 1], px[i + 2], 255]);
        }
    }
    Ok(img)
}
