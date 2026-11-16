//! Critique sprites: every issue with its cells and a fix, then a score; with `--frames`, drift across them as
//! frames of one animation. `--intended` lists cells meant to stand alone (an eye, a buckle). Against the duel's
//! ruler (`pixfx_draw::RULER`): `--standing` checks a standing character is its height, `--against
//! PART*RATIO` checks a thing is that many times a body part (`eye*2` for glasses, `head*1.5` for a hat's brim,
//! `blade*1` for a sword). Exits 1 when a sprite has an error.
//!
//!   cargo run -p pixfx-pixel --example critique -- <sprite.png>... [--frames] [--intended X,Y;X,Y...]
//!       [--standing] [--against PART*RATIO]

mod common;

use common::{Args, exit, read_image};
use pixfx_draw::RULER;
use pixfx_pixel::critique::{Severity, critique, critique_frames};
use pixfx_pixel::scale;
use pixfx_pixel::tool::report_text;

const USAGE: &str = "usage: critique <sprite.png>... [--frames] [--intended X,Y;X,Y...] [--standing] [--against PART*RATIO]";

fn main() -> std::process::ExitCode {
    exit(run())
}

fn run() -> Result<bool, String> {
    let mut args = Args::new(USAGE);
    let frames = args.flag("--frames");
    let standing = args.flag("--standing");
    let against = match args.value("--against")? {
        Some(v) => Some(part_ratio(&v)?),
        None => None,
    };
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
        .map(|(p, img)| {
            let mut r = critique(img, &intended);
            if standing {
                r.issues.extend(scale::standing(img, &RULER));
            }
            if let Some((name, part, ratio)) = &against {
                r.issues.extend(scale::against(
                    img,
                    &format!("{name} x {ratio}"),
                    *part,
                    *ratio,
                ));
            }
            (p.display().to_string(), r)
        })
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

/// `PART*RATIO`: a ruler part by name and how many times it the thing is.
fn part_ratio(v: &str) -> Result<(String, u32, f64), String> {
    let bad = || format!("--against expects PART*RATIO (eye*2, head*1.5): {v}");
    let (name, ratio) = v.split_once('*').ok_or_else(bad)?;
    let part = scale::part(&RULER, name.trim()).ok_or_else(bad)?;
    let ratio: f64 = ratio.trim().parse().map_err(|_| bad())?;
    Ok((name.trim().to_string(), part, ratio))
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
