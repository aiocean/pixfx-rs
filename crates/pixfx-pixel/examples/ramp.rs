//! Make ramps, one material's shades each, hue-shifted (dark and cool to light and warm), from a base colour per
//! material, and write them as one named palette (`<out>/<name>.gpl`) that `--palette` takes in every other step:
//! `clean` snaps an image model's picture onto it, `read` names each letter by its material and step.
//!
//!   cargo run -p pixfx-pixel --example ramp -- skin=#d99a6c cloth=#3355aa [--steps 5] [--name ramps] [--out DIR]

mod common;

use common::{Args, exit, write_text};
use pixfx_draw::color::to_hex;
use pixfx_draw::hex;
use pixfx_pixel::tool::Palette;

const USAGE: &str = "usage: ramp <material>=<#rrggbb>... [--steps 5] [--name ramps] [--out DIR]";

fn main() -> std::process::ExitCode {
    exit(run())
}

fn run() -> Result<bool, String> {
    let mut args = Args::new(USAGE);
    let steps = args.number("--steps", 5usize)?;
    let name = args.value("--name")?.unwrap_or_else(|| "ramps".to_string());
    let out = args.out()?;
    let bases = args
        .files()?
        .iter()
        .map(|w| {
            let w = w.to_string_lossy();
            let (material, base) = w
                .split_once('=')
                .ok_or(format!("{w}: expected <material>=<#rrggbb>\n{USAGE}"))?;
            Ok((
                material.to_string(),
                hex(base).map_err(|e| format!("{w}: {e}"))?,
            ))
        })
        .collect::<Result<Vec<_>, String>>()?;
    if bases.is_empty() {
        return Err(USAGE.to_string());
    }
    let palette = Palette::ramps(&bases, steps);
    for (c, n) in palette.colors.iter().zip(&palette.names) {
        println!("{n} {}", to_hex(*c));
    }
    write_text(&out.join(format!("{name}.gpl")), &palette.gpl(&name))?;
    Ok(true)
}
