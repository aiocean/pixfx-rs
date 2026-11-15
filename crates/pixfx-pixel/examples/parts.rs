//! Split a sprite into named parts by a spec the agent writes (its colour letters are those the `read` script shows
//! with the same palette). Prints a line per part (`name parent x,y,w,h pivot`) and writes `<out>/<part>.png` and
//! `<out>/<part>.txt` for each.
//!
//!   cargo run -p pixfx-pixel --example parts -- <sprite.png> <spec.txt> [--palette P] [--out DIR]

mod common;

use common::{Args, exit, read_image, read_text, write_image, write_text};
use pixfx_pixel::text_named;
use pixfx_pixel::tool::{part_line, parts};

const USAGE: &str = "usage: parts <sprite.png> <spec.txt> [--palette P] [--out DIR]";

fn main() -> std::process::ExitCode {
    exit(run())
}

fn run() -> Result<bool, String> {
    let mut args = Args::new(USAGE);
    let palette = args.palette()?;
    let out = args.out()?;
    let [sprite, spec] = args.exactly()?;
    let (parts, palette) = parts(&read_image(&sprite)?, &read_text(&spec)?, palette.as_ref())?;
    if let Some(p) = parts.iter().find(|p| {
        !p.name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
    }) {
        return Err(format!(
            "part name {:?}: letters, digits, _ and - only (it names files)",
            p.name
        ));
    }
    let names: Vec<&str> = palette.names.iter().map(String::as_str).collect();
    for p in &parts {
        println!("{}", part_line(p));
        write_image(
            &out.join(format!("{}.png", p.name)),
            &p.map.paint(&palette.colors),
        )?;
        let listing = text_named(&p.map, &palette.colors, &names).map_err(|e| e.to_string())?;
        write_text(&out.join(format!("{}.txt", p.name)), &listing)?;
    }
    Ok(true)
}
