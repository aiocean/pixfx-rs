//! `pixfx`: tools for pixfx effects.
//!
//!   pixfx parity <id> [--opt key=value]... [--pixelfx DIR] [--out DIR]
//!
//!   pixfx sheet <id> [--palette WORD] [--outline WORD] [--out DIR]
//!   pixfx render <id> --gif [--scale N] [--palette WORD] [--outline WORD] [--out DIR]
//!   pixfx bake <id> [--palette WORD] [--outline WORD] [--out DIR]
//!   pixfx bench [--ids ID,ID...] [--n N] [--secs S] [--hz H]
//!   pixfx poses <thunder|frost> [--blank POSE,...] [--out DIR]
//!   pixfx trace <art.png> --look <thunder|frost> --pose POSE --px N [--crop X,Y,W,H] [--flip] [--lift N]
//!               [--tolerance N] [--out DIR] [--target PNG]
//!   pixfx diff <a.png> <b.png> [--a-crop X,Y,W,H] [--b-crop X,Y,W,H] [--out PNG]
//!
//! `sheet` writes `<name>.png`, `<name>_color.png`, `<name>_light.png` and `<name>.json` (beanfx layout) for
//! `<id>`; `render --gif` writes `<name>.gif`, each pixel blown up `N` times (default 4); `bake` writes
//! `<name>.bake.json` and its pages `<name>.bake.<n>.png`, every frame as live hosts draw it, which hosts load to
//! play the effect without drawing it. `<name>` is `<id>`
//! with `/` as `_`; `--out` defaults to `target/out`. `--outline` outlines the effect: `off`, a preset (`dark`,
//! `black`, `white`, `thick`) or a hex colour (`#ffcc00`); left out, it looks as designed. `--palette` recolours
//! it with a preset (`fire`, `ice`, `poison`, `holy`, `shadow`, `arcane`, `blood`, `electric`, `nature`, `gold`).
//!
//! `parity` renders `<id>` with pixelfx (through `tools/pixelfx-ref.mjs`, under node) and with its pixfx port, prints
//! how many pixels differ and whether that passes, and writes `<out>/<id>.side.gif` (pixelfx left, pixfx
//! right). Each `--opt key=value` sets an option on both sides: `seed` (a whole number), `flip` (0 or 1), or a
//! numeric option of the effect (`hits=2`). The pixelfx dir is `--pixelfx`, else `$PIXELFX`, else `./pixelfx`; `--out` defaults to
//! `target/parity`. Exits 0 on PASS, 1 on FAIL, 2 when the check could not run.
//!
//! `bench` plays `N` instances of each id (default the three fires, 50 each) for `S` seconds of an `H` Hz display
//! (default 10 s at 60 Hz) and prints what a display frame costs natively: updating, then per id drawing and RGBA.
//!
//! `poses` writes `<look>-poses.png`, every pose of a fighter look painted as the duel paints it, 8 to a row in
//! the order of `pixfx_cli::poses::POSES`; `--blank` leaves the named poses' cells empty (a reference sheet for an
//! artist or an image model to fill).
//!
//! `trace` turns drawn art of a fighter pose (facing right) into a table the fighter painter paints from: the art
//! (`--crop`ped to the pose, mirrored with `--flip` when it faces left) is cut from its background (the top-left colour, give or take `--tolerance`,
//! default 24), matched to the look's colours, shrunk by `--px` (its pixels per sprite pixel) and merged into
//! rects, standing `--lift` pixels above the feet. It writes `<pose>_<hairdo>.rs` into `--out` (default
//! `crates/pixfx-fighter/src/traced`) and, with `--target`, the pose as the engine paints that table. List the
//! table in `TABLES` in `crates/pixfx-fighter/src/traced.rs` to use it.
//!
//! `diff` compares two images (each `--a-crop` / `--b-crop`ped first), prints how many pixels differ, and with
//! `--out` writes a, b and the difference side by side. Exits 0 when they match, 1 when they differ.
#![forbid(unsafe_code)]

use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

use pixfx_cli::bench;
use pixfx_cli::opts::CliOpts;
use pixfx_cli::parity::{compare, read_reference, write_side_gif};
use pixfx_cli::poses::{POSES, pose_cells, pose_named};
use pixfx_cli::trace::table_source;
use pixfx_core::{Options, render_frames};
use pixfx_draw::Outline;
use pixfx_draw::trace::{RgbaImage, cut, diff};
use pixfx_export::{
    Grid, bake, decode_png_rgba, pack_cells, write_bake, write_gif, write_png_rgba, write_sheet,
};
use pixfx_fighter::look::Look;
use pixfx_fighter::paint::{SPRITE, paint_traced};
use pixfx_fighter::traced;

const USAGE: &str = "usage: pixfx parity <id> [--opt key=value]... [--pixelfx DIR] [--out DIR]
       pixfx sheet <id> [--palette WORD] [--outline WORD] [--out DIR]
       pixfx render <id> --gif [--scale N] [--palette WORD] [--outline WORD] [--out DIR]
       pixfx bake <id> [--palette WORD] [--outline WORD] [--out DIR]
       pixfx poses <thunder|frost> [--blank POSE,...] [--out DIR]
       pixfx trace <art.png> --look <thunder|frost> --pose POSE --px N [--crop X,Y,W,H] [--flip] [--lift N] [--tolerance N] [--out DIR] [--target PNG]
       pixfx diff <a.png> <b.png> [--a-crop X,Y,W,H] [--b-crop X,Y,W,H] [--out PNG]
       pixfx bench [--ids ID,ID...] [--n N] [--secs S] [--hz H]";

/// What `pixfx sheet`, `pixfx render` and `pixfx bake` produce.
enum Export {
    Sheet,
    Bake,
    Gif { scale: u32 },
}

struct ExportArgs {
    id: String,
    out: PathBuf,
    export: Export,
    outline: Option<Outline>,
    palette: Option<Vec<pixfx_draw::Rgba>>,
}

/// The reference script, found from this crate's source so `pixfx` runs from any directory of the repo.
const REF_SCRIPT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../tools/pixelfx-ref.mjs");

struct ParityArgs {
    id: String,
    pixelfx: PathBuf,
    out: PathBuf,
    opts: CliOpts,
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let result = match args.split_first() {
        Some((cmd, rest)) if cmd == "parity" => parse_parity(rest).and_then(|a| parity(&a)),
        Some((cmd, rest)) if cmd == "poses" => poses(rest).map(|()| true),
        Some((cmd, rest)) if cmd == "trace" => trace(rest).map(|()| true),
        Some((cmd, rest)) if cmd == "diff" => diff_images(rest),
        Some((cmd, rest)) if cmd == "sheet" || cmd == "render" || cmd == "bake" => {
            parse_export(cmd, rest).and_then(|a| export(&a).map(|()| true))
        }
        Some((cmd, rest)) if cmd == "bench" => bench::parse_args(rest)
            .and_then(|cfg| bench::run(&cfg))
            .map(|report| {
                println!("{}", bench::table(&report));
                true
            }),
        _ => Err(USAGE.to_string()),
    };
    match result {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::from(1),
        Err(e) => {
            eprintln!("pixfx: {e}");
            ExitCode::from(2)
        }
    }
}

fn parse_parity(args: &[String]) -> Result<ParityArgs, String> {
    let mut id = None;
    let mut pixelfx =
        std::env::var_os("PIXELFX").map_or_else(|| PathBuf::from("pixelfx"), PathBuf::from);
    let mut out = PathBuf::from("target/parity");
    let mut opt_words = Vec::new();
    let mut it = args.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--pixelfx" => pixelfx = it.next().ok_or(USAGE)?.into(),
            "--out" => out = it.next().ok_or(USAGE)?.into(),
            "--opt" => opt_words.push(it.next().ok_or(USAGE)?.clone()),
            flag if flag.starts_with("--") => return Err(format!("unknown flag {flag}\n{USAGE}")),
            _ if id.is_none() => id = Some(a.clone()),
            _ => return Err(USAGE.to_string()),
        }
    }
    Ok(ParityArgs {
        id: id.ok_or(USAGE)?,
        pixelfx,
        out,
        opts: CliOpts::parse(&opt_words)?,
    })
}

fn parse_export(cmd: &str, args: &[String]) -> Result<ExportArgs, String> {
    let (mut id, mut out) = (None, PathBuf::from("target/out"));
    let (mut gif, mut scale) = (false, 4);
    let mut outline = None;
    let mut palette = None;
    let mut it = args.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--out" => out = it.next().ok_or(USAGE)?.into(),
            "--outline" => {
                let word = it.next().ok_or(USAGE)?;
                outline = Some(Outline::parse(word).map_err(|e| e.to_string())?);
            }
            "--palette" => {
                let word = it.next().ok_or(USAGE)?;
                palette = Some(pixfx_draw::palette::parse(word).map_err(|e| e.to_string())?);
            }
            "--gif" if cmd == "render" => gif = true,
            "--scale" if cmd == "render" => {
                scale = it
                    .next()
                    .and_then(|n| n.parse().ok())
                    .filter(|&n| n >= 1)
                    .ok_or(USAGE)?;
            }
            flag if flag.starts_with("--") => return Err(format!("unknown flag {flag}\n{USAGE}")),
            _ if id.is_none() => id = Some(a.clone()),
            _ => return Err(USAGE.to_string()),
        }
    }
    let export = match (cmd, gif) {
        ("sheet", _) => Export::Sheet,
        ("bake", _) => Export::Bake,
        (_, true) => Export::Gif { scale },
        _ => return Err(format!("render needs --gif\n{USAGE}")),
    };
    Ok(ExportArgs {
        id: id.ok_or(USAGE)?,
        out,
        export,
        outline,
        palette,
    })
}

/// Render `a.id` and write the sheet, gif or baked clip into `a.out`.
fn export(a: &ExportArgs) -> Result<(), String> {
    let id = a.id.as_str();
    let effect = pixfx_effects::effect(id)
        .ok_or_else(|| format!("no pixfx effect {id} (not ported yet?)"))?;
    let opts = Options {
        outline: a.outline,
        palette: a.palette.clone(),
        ..Options::default()
    };
    let name = id.replace('/', "_");
    std::fs::create_dir_all(&a.out).map_err(|e| format!("creating {}: {e}", a.out.display()))?;
    let rendered = || render_frames(effect, &opts, 0);
    match a.export {
        Export::Bake => {
            let clip = bake(effect, opts.clone())?;
            let files = write_bake(&clip, &a.out, &name)
                .map_err(|e| format!("writing the {id} bake: {e}"))?;
            println!(
                "{id}: {} ({} pages)",
                files.json.display(),
                files.pages.len()
            );
        }
        Export::Sheet => {
            let files = write_sheet(&rendered(), effect.meta(), &a.out, &name)
                .map_err(|e| format!("writing the {id} sheet: {e}"))?;
            println!("{id}: {}", files.json.display());
        }
        Export::Gif { scale } => {
            let path = a.out.join(format!("{name}.gif"));
            write_gif(&path, &rendered(), scale)
                .map_err(|e| format!("writing {}: {e}", path.display()))?;
            println!("{id}: {}", path.display());
        }
    }
    Ok(())
}

/// `pixfx poses`: write `<look>-poses.png`, every pose of a fighter look on one sheet, into the out dir.
fn poses(args: &[String]) -> Result<(), String> {
    let mut look = None;
    let mut blank = Vec::new();
    let mut out = PathBuf::from("out");
    let mut it = args.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--blank" => {
                for name in it.next().ok_or(USAGE)?.split(',') {
                    let names = POSES.map(|(n, _)| n).join(", ");
                    blank.push(pose_named(name).ok_or(format!("no pose {name}: one of {names}"))?);
                }
            }
            "--out" => out = it.next().ok_or(USAGE)?.into(),
            flag if flag.starts_with("--") => return Err(format!("unknown flag {flag}\n{USAGE}")),
            _ if look.is_none() => look = Some(a.clone()),
            _ => return Err(USAGE.to_string()),
        }
    }
    let name = look.ok_or(USAGE)?;
    let look = pixfx_fighter::look::Look::preset(&name)
        .ok_or(format!("no fighter look {name}: thunder or frost"))?;
    let cells = pose_cells(&look, &blank);
    let grid = Grid::new(cells.len() as u32, SPRITE as u32, SPRITE as u32);
    std::fs::create_dir_all(&out).map_err(|e| format!("creating {}: {e}", out.display()))?;
    let path = out.join(format!("{name}-poses.png"));
    write_png_rgba(
        &path,
        grid.width(),
        grid.height(),
        &pack_cells(&grid, &cells, 4),
    )
    .map_err(|e| format!("writing {}: {e}", path.display()))?;
    println!("{name}: {}", path.display());
    Ok(())
}

/// Read a PNG as RGBA.
fn read_image(path: &Path) -> Result<RgbaImage, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("reading {}: {e}", path.display()))?;
    decode_png_rgba(&bytes).map_err(|e| format!("{}: {e}", path.display()))
}

/// `X,Y,W,H`, four whole numbers.
fn parse_rect(v: Option<&String>) -> Result<[u32; 4], String> {
    let v = v.ok_or(USAGE)?;
    let n: Vec<u32> = v
        .split(',')
        .map(|p| p.trim().parse())
        .collect::<Result<_, _>>()
        .map_err(|_| format!("expected X,Y,W,H: {v}"))?;
    n.try_into().map_err(|_| format!("expected X,Y,W,H: {v}"))
}

fn cropped(img: RgbaImage, rect: Option<[u32; 4]>) -> RgbaImage {
    match rect {
        Some([x, y, w, h]) => img.crop(x, y, w, h),
        None => img,
    }
}

/// `pixfx trace`: drawn art of a pose into a table the fighter painter paints from.
fn trace(args: &[String]) -> Result<(), String> {
    let (mut art, mut look, mut pose, mut px, mut crop) = (None, None, None, None, None);
    let (mut lift, mut tolerance) = (0, 24);
    let mut out = PathBuf::from(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../pixfx-fighter/src/traced"
    ));
    let mut target = None;
    let mut flip = false;
    let mut it = args.iter();
    let number = |v: Option<&String>| {
        v.and_then(|v| v.parse::<f64>().ok())
            .ok_or(format!("expected a number\n{USAGE}"))
    };
    while let Some(a) = it.next() {
        match a.as_str() {
            "--look" => {
                let name = it.next().ok_or(USAGE)?;
                look = Some(
                    Look::preset(name)
                        .ok_or(format!("no fighter look {name}: thunder or frost"))?,
                );
            }
            "--pose" => {
                let name = it.next().ok_or(USAGE)?;
                pose = Some((
                    name.clone(),
                    pose_named(name).ok_or(format!("no pose {name}"))?,
                ));
            }
            "--px" => px = Some(number(it.next())?),
            "--lift" => lift = number(it.next())? as i32,
            "--tolerance" => tolerance = number(it.next())? as u32,
            "--crop" => crop = Some(parse_rect(it.next())?),
            "--flip" => flip = true,
            "--out" => out = it.next().ok_or(USAGE)?.into(),
            "--target" => target = Some(PathBuf::from(it.next().ok_or(USAGE)?)),
            flag if flag.starts_with("--") => return Err(format!("unknown flag {flag}\n{USAGE}")),
            _ if art.is_none() => art = Some(PathBuf::from(a)),
            _ => return Err(USAGE.to_string()),
        }
    }
    let (art_path, look) = (
        art.ok_or(USAGE)?,
        look.ok_or(format!("--look is required\n{USAGE}"))?,
    );
    let ((pose_name, _), px) = (
        pose.ok_or(format!("--pose is required\n{USAGE}"))?,
        px.ok_or(format!("--px is required\n{USAGE}"))?,
    );
    let art = cropped(read_image(&art_path)?, crop);
    let art = cut(&if flip { art.flipped() } else { art }, tolerance);
    if art.w == 0 {
        return Err(format!(
            "{}: nothing left once the background is cut",
            art_path.display()
        ));
    }
    let rects = traced::trace(&art, &look, px, lift);
    let hairdo = format!("{:?}", look.hairdo).to_lowercase();
    std::fs::create_dir_all(&out).map_err(|e| format!("creating {}: {e}", out.display()))?;
    let table = out.join(format!("{pose_name}_{hairdo}.rs"));
    let made_by = format!("pixfx trace {}", args.join(" "));
    std::fs::write(&table, table_source(&rects, &made_by))
        .map_err(|e| format!("writing {}: {e}", table.display()))?;
    println!(
        "{pose_name}: {} rects from {}x{} art -> {}",
        rects.len(),
        art.w,
        art.h,
        table.display()
    );
    if let Some(path) = target {
        let sprite = paint_traced(&look, &rects).to_image();
        write_png_rgba(&path, sprite.w, sprite.h, &sprite.rgba)
            .map_err(|e| format!("writing {}: {e}", path.display()))?;
        println!("{pose_name}: target {}", path.display());
    }
    Ok(())
}

/// `pixfx diff`: `Ok(true)` when the two images match.
fn diff_images(args: &[String]) -> Result<bool, String> {
    let (mut paths, mut a_crop, mut b_crop, mut out) = (Vec::new(), None, None, None);
    let mut it = args.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--a-crop" => a_crop = Some(parse_rect(it.next())?),
            "--b-crop" => b_crop = Some(parse_rect(it.next())?),
            "--out" => out = Some(PathBuf::from(it.next().ok_or(USAGE)?)),
            flag if flag.starts_with("--") => return Err(format!("unknown flag {flag}\n{USAGE}")),
            _ => paths.push(PathBuf::from(a)),
        }
    }
    let [a_path, b_path] = <[PathBuf; 2]>::try_from(paths).map_err(|_| USAGE.to_string())?;
    let a = cropped(read_image(&a_path)?, a_crop);
    let b = cropped(read_image(&b_path)?, b_crop);
    let (d, mask) = diff(&a, &b);
    println!("{} of {} pixels differ", d.differing, d.total);
    if let Some(path) = out {
        if (a.w, a.h) != (b.w, b.h) {
            return Err(format!(
                "side by side needs one size: {}x{} and {}x{}",
                a.w, a.h, b.w, b.h
            ));
        }
        let grid = Grid::new(3, a.w, a.h);
        let sheet = pack_cells(&grid, &[a.rgba, b.rgba, mask.rgba], 4);
        write_png_rgba(&path, grid.width(), grid.height(), &sheet)
            .map_err(|e| format!("writing {}: {e}", path.display()))?;
        println!("side by side: {}", path.display());
    }
    Ok(d.differing == 0)
}

/// Run the parity check; `Ok(pass)` once it ran.
fn parity(a: &ParityArgs) -> Result<bool, String> {
    let id = a.id.as_str();
    let effect = pixfx_effects::effect(id)
        .ok_or_else(|| format!("no pixfx effect {id} (not ported yet?)"))?;
    reference_render(a)?;
    let port = render_frames(effect, &a.opts.options(), 0);
    let reference = read_reference(&a.out, id, port.origin)
        .map_err(|e| format!("reading the {id} reference: {e}"))?;
    let report = compare(&reference, &port);
    if (reference.w, reference.h, reference.frames.len()) != (port.w, port.h, port.frames.len()) {
        eprintln!(
            "{id}: pixelfx renders {} frames of {}x{}, pixfx {} of {}x{}",
            reference.frames.len(),
            reference.w,
            reference.h,
            port.frames.len(),
            port.w,
            port.h
        );
    }
    let gif = a.out.join(format!("{id}.side.gif"));
    write_side_gif(&gif, &reference, &port)
        .map_err(|e| format!("writing {}: {e}", gif.display()))?;
    println!(
        "{id}: colour {:.2}% differ, light {:.2}% differ → {}",
        report.colour_diff_frac * 100.0,
        report.light_diff_frac * 100.0,
        if report.pass() { "PASS" } else { "FAIL" }
    );
    Ok(report.pass())
}

/// Write pixelfx's render of `a.id` into `a.out` with the reference script (its progress line is dropped).
fn reference_render(a: &ParityArgs) -> Result<(), String> {
    let run = Command::new("node")
        .arg(Path::new(REF_SCRIPT))
        .arg(&a.id)
        .arg("--pixelfx")
        .arg(&a.pixelfx)
        .arg("--out")
        .arg(&a.out)
        .arg("--opts")
        .arg(a.opts.to_json())
        .output()
        .map_err(|e| format!("running node: {e}"))?;
    if run.status.success() {
        Ok(())
    } else {
        Err(format!(
            "pixelfx reference render of {} failed ({}):\n{}",
            a.id,
            run.status,
            String::from_utf8_lossy(&run.stderr).trim_end()
        ))
    }
}
