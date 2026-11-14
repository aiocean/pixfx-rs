//! `pixfx`: tools for pixfx effects.
//!
//!   pixfx parity <id> [--opt key=value]... [--pixelfx DIR] [--out DIR]
//!
//!   pixfx sheet <id> [--palette WORD] [--outline WORD] [--out DIR]
//!   pixfx render <id> --gif [--scale N] [--palette WORD] [--outline WORD] [--out DIR]
//!   pixfx bake <id> [--palette WORD] [--outline WORD] [--out DIR]
//!   pixfx bench [--ids ID,ID...] [--n N] [--secs S] [--hz H]
//!   pixfx poses <thunder|frost> [--blank POSE,...] [--out DIR]
//!   pixfx pixel <picture.png> [--crop X,Y,W,H] [--flip] [--cut TOLERANCE] [--px N]
//!               [--palette HEX,HEX,...|thunder|frost] [--rects] [--scale N] [--out DIR]
//!   pixfx pixel --blank WxH [--scale N] [--out DIR]
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
//! `pixel` shows a picture to an agent as a pixel grid: an image model's drawing, an artist's, or a game's own
//! sprite. The picture is `--crop`ped, mirrored with `--flip`, cut from its background with `--cut` (the top-left
//! colour, give or take the tolerance; specks and guide lines go too), matched to `--palette` (hex colours, or a
//! fighter look's colours by name; left out, the picture's own colours) and shrunk by `--px` (the picture's pixels
//! per cell, default 1). It prints the grid as text, a letter per cell with a colour legend and the column and row
//! numbers, and `--rects` adds the cells merged into rects (`x y w h letter`, y down from the top). It writes
//! `<name>.txt`, `<name>.pixels.png` (one pixel a cell) and `<name>.zoom.png` (`--scale` pixels a cell, default 8,
//! inside grid lines, brighter every 8 cells) into `--out` (default `target/pixel`). `--blank WxH` writes only
//! `blank-<W>x<H>.zoom.png`, the empty grid, for an image model to draw into. The output is for the agent to read
//! and write painter code from; nothing in the engine paints from it.
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
use pixfx_core::{Options, render_frames};
use pixfx_draw::Outline;
use pixfx_draw::RgbaImage;
use pixfx_export::{
    Grid, bake, decode_png_rgba, pack_cells, write_bake, write_gif, write_png_rgba, write_sheet,
};
use pixfx_fighter::paint::SPRITE;
use pixfx_pixel::{Map, diff, rects, text, zoom};

const USAGE: &str = "usage: pixfx parity <id> [--opt key=value]... [--pixelfx DIR] [--out DIR]
       pixfx sheet <id> [--palette WORD] [--outline WORD] [--out DIR]
       pixfx render <id> --gif [--scale N] [--palette WORD] [--outline WORD] [--out DIR]
       pixfx bake <id> [--palette WORD] [--outline WORD] [--out DIR]
       pixfx poses <thunder|frost> [--blank POSE,...] [--out DIR]
       pixfx pixel <picture.png> [--crop X,Y,W,H] [--flip] [--cut TOLERANCE] [--px N] [--palette HEX,HEX,...|thunder|frost] [--rects] [--scale N] [--out DIR]
       pixfx pixel --blank WxH [--scale N] [--out DIR]
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
        Some((cmd, rest)) if cmd == "pixel" => pixel(rest).map(|()| true),
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

/// Write `img` as a PNG.
fn write_image(path: &Path, img: &RgbaImage) -> Result<(), String> {
    write_png_rgba(path, img.w, img.h, &img.rgba)
        .map_err(|e| format!("writing {}: {e}", path.display()))
}

/// `pixfx pixel`: a picture as a pixel grid an agent reads, or a blank grid for an image model.
fn pixel(args: &[String]) -> Result<(), String> {
    let a = pixfx_cli::pixel::parse_args(args)?;
    std::fs::create_dir_all(&a.out).map_err(|e| format!("creating {}: {e}", a.out.display()))?;
    let Some(picture) = &a.picture else {
        let (w, h) = a.blank.ok_or(pixfx_cli::pixel::USAGE)?;
        let path = a.out.join(format!("blank-{w}x{h}.zoom.png"));
        write_image(&path, &zoom(&Map::empty(w, h), &[], a.scale))?;
        println!("{}", path.display());
        return Ok(());
    };
    let (map, palette) = pixfx_cli::pixel::grid(&read_image(picture)?, &a)?;
    let name = picture
        .file_stem()
        .map_or("picture".into(), |s| s.to_string_lossy());
    let mut listing = text(&map, &palette).map_err(|e| e.to_string())?;
    if a.rects {
        let letters: Vec<char> = pixfx_pixel::LEGEND.chars().collect();
        listing.push_str("rects: x y w h letter\n");
        for r in rects(&map) {
            let letter = letters.get(r.value).copied().unwrap_or('?');
            listing.push_str(&format!("{} {} {} {} {letter}\n", r.x, r.y, r.w, r.h));
        }
    }
    let txt = a.out.join(format!("{name}.txt"));
    std::fs::write(&txt, &listing).map_err(|e| format!("writing {}: {e}", txt.display()))?;
    let pixels = a.out.join(format!("{name}.pixels.png"));
    write_image(&pixels, &map.paint(&palette))?;
    let zoomed = a.out.join(format!("{name}.zoom.png"));
    write_image(&zoomed, &zoom(&map, &palette, a.scale))?;
    print!("{listing}");
    println!(
        "{}\n{}\n{}",
        txt.display(),
        pixels.display(),
        zoomed.display()
    );
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
