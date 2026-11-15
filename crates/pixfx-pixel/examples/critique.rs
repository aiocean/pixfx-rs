//! Critique sprites: every issue with its cells and a fix, then a score; with `--frames`, drift across them as
//! frames of one animation. `--intended` lists cells meant to stand alone (an eye, a buckle). Exits 1 when a sprite
//! has an error.
//!
//!   cargo run -p pixfx-pixel --example critique -- <sprite.png>... [--frames] [--intended X,Y;X,Y...]

mod common;

use common::{Args, exit, read_image};
use pixfx_pixel::critique::{Severity, critique, critique_frames};
use pixfx_pixel::tool::report_text;

const USAGE: &str = "usage: critique <sprite.png>... [--frames] [--intended X,Y;X,Y...]";

fn main() -> std::process::ExitCode {
    exit(run())
}

fn run() -> Result<bool, String> {
    let mut args = Args::new(USAGE);
    let frames = args.flag("--frames");
    let intended = match args.value("--intended")? {
        Some(v) => cells(&v)?,
        None => Vec::new(),
    };
    let sprites = args.files()?;
    if sprites.is_empty() {
        return Err(USAGE.to_string());
    }
    let imgs = sprites
        .iter()
        .map(|p| read_image(p))
        .collect::<Result<Vec<_>, _>>()?;
    let mut reports: Vec<(String, _)> = sprites
        .iter()
        .zip(&imgs)
        .map(|(p, img)| (p.display().to_string(), critique(img, &intended)))
        .collect();
    if frames {
        reports.push(("frames".to_string(), critique_frames(&imgs)));
    }
    let mut ok = true;
    for (name, r) in &reports {
        println!("{name}");
        print!("{}", report_text(r));
        ok &= !r.issues.iter().any(|i| i.severity == Severity::Error);
    }
    Ok(ok)
}

/// `X,Y;X,Y;...`: cells by their column and row.
fn cells(v: &str) -> Result<Vec<(u32, u32)>, String> {
    v.split(';')
        .filter(|c| !c.trim().is_empty())
        .map(|c| {
            c.split_once(',')
                .and_then(|(x, y)| Some((x.trim().parse().ok()?, y.trim().parse().ok()?)))
                .ok_or(format!("--intended expects X,Y;X,Y: {c}"))
        })
        .collect()
}
