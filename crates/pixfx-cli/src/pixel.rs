//! `pixfx pixel <verb>`: the pixfx-pixel library for an agent at a shell. Each verb reads its pictures, runs the
//! library, writes what it made into `--out` (default `target/pixel`) and prints what it wrote.
//!
//! The verbs' work is in pure functions here ([`clean`], [`read`], [`draw`], [`view`], [`sheet`], [`cut`],
//! [`parts`]), so tests drive them without files; [`run`] is the shell that reads and writes.

use std::path::{Path, PathBuf};

use pixfx_draw::{Rgba, RgbaImage, hex};
use pixfx_export::{decode_png_rgba, write_png_rgba};
use pixfx_fighter::look::Look;
use pixfx_pixel::critique::{Report, Severity, critique, critique_frames};
use pixfx_pixel::parts::{Part, parse_spec, split};
use pixfx_pixel::recover::{fit_height, recover};
use pixfx_pixel::reduce::{SnapMode, reduce, snap};
use pixfx_pixel::sheet::{
    Guide, Layout, blank, directions, guide, layout_json, parse_bones, parse_layout,
};
use pixfx_pixel::view::{View, ViewOptions};
use pixfx_pixel::{LEGEND, Map, colors, nearest, rects, shrink, text_named};

pub const USAGE: &str = "pixfx pixel clean <picture.png> [--crop X,Y,W,H] [--flip] [--cut TOLERANCE] [--height N] [--palette P] [--max-colors N]
       pixfx pixel read <sprite.png> [--palette P] [--region X,Y,W,H] [--rects]
       pixfx pixel draw <sprite.txt>
       pixfx pixel view <sprite.png> [--region X,Y,W,H] [--max-side N]
       pixfx pixel critique <sprite.png>... [--frames] [--intended X,Y;X,Y...]
       pixfx pixel sheet (--cell WxH --grid CxR | --directions <sprite.png>) [--gap N] [--scale N] [--pixel-lines] [--bones FILE]
       pixfx pixel cut <drawn.png> <sheet.json>
       pixfx pixel parts <sprite.png> <spec.txt> [--palette P]
every verb takes [--out DIR] (default target/pixel); P is hex colours split by commas, or a fighter look (thunder, frost)";

/// Colours a grid is matched to, with a name for each the agent reads in the legend (empty when it has none).
#[derive(Clone, Debug, PartialEq)]
pub struct Palette {
    pub colors: Vec<Rgba>,
    pub names: Vec<String>,
}

/// `pixfx pixel clean`: an image model's picture down to one pixel a cell.
#[derive(Clone, Debug, PartialEq)]
pub struct CleanArgs {
    pub picture: PathBuf,
    pub crop: Option<[u32; 4]>,
    pub flip: bool,
    /// Cut the picture from its background (the top-left colour, give or take this much) before the grid is found.
    pub cut: Option<u32>,
    /// Resample the clean sprite to this many rows, so a drawing fits the game's sprites.
    pub height: Option<u32>,
    /// Snap every colour to this palette; without one, merge colours down to `max_colors`.
    pub palette: Option<Palette>,
    pub max_colors: usize,
}

/// `pixfx pixel sheet`: a blank sheet for an image model to draw into.
#[derive(Clone, Debug, PartialEq)]
pub struct SheetArgs {
    pub cell: (u32, u32),
    pub grid: (u32, u32),
    pub gap: u32,
    pub scale: u32,
    pub pixel_lines: bool,
    /// A sprite to draw its eight directions around (a 3x3 sheet); `cell` and `grid` are then the sprite's.
    pub directions: Option<PathBuf>,
    /// A skeleton drawn into every cell: a file of bones, one `X,Y X,Y` per line, in sprite pixels.
    pub bones: Option<PathBuf>,
}

/// One `pixfx pixel` verb and what it was given.
#[derive(Clone, Debug, PartialEq)]
pub enum Verb {
    Clean(CleanArgs),
    Read {
        sprite: PathBuf,
        palette: Option<Palette>,
        region: Option<[u32; 4]>,
        rects: bool,
    },
    Draw {
        text: PathBuf,
    },
    View {
        sprite: PathBuf,
        region: Option<[u32; 4]>,
        max_side: u32,
    },
    Critique {
        sprites: Vec<PathBuf>,
        frames: bool,
        intended: Vec<(u32, u32)>,
    },
    Sheet(SheetArgs),
    Cut {
        drawn: PathBuf,
        layout: PathBuf,
    },
    Parts {
        sprite: PathBuf,
        spec: PathBuf,
        palette: Option<Palette>,
    },
}

/// A parsed `pixfx pixel` command line.
#[derive(Clone, Debug, PartialEq)]
pub struct Command {
    pub verb: Verb,
    pub out: PathBuf,
}

// --- Palettes ---------------------------------------------------------------------------------------------------

/// A fighter look's colours, each once, in the order it lists them, with the name of the part it paints
/// (`robe0`, `boot`); the palette its poses are drawn in.
pub fn look_colors(look: &Look) -> Vec<(String, Rgba)> {
    let mut all: Vec<(String, Rgba)> = Vec::new();
    let mut add = |name: &str, colors: &[Rgba]| {
        for (i, &c) in colors.iter().enumerate() {
            let named = if colors.len() > 1 {
                format!("{name}{i}")
            } else {
                name.to_string()
            };
            all.push((named, c));
        }
    };
    add("robe", &look.robe);
    add("inner", &look.inner);
    add("trim", &look.trim);
    add("hair", &look.hair);
    add("skin", &look.skin);
    add("boot", &[look.boot]);
    add("blade", &look.blade);
    add("jade", &[look.jade]);
    add("ribbon", &look.ribbon);
    add("glow", &[look.glow]);
    add("outline", &[look.outline]);
    let mut distinct: Vec<(String, Rgba)> = Vec::new();
    for (name, c) in all {
        if !distinct.iter().any(|(_, d)| *d == c) {
            distinct.push((name, c));
        }
    }
    distinct
}

/// A fighter look's colours, as [`look_colors`] orders them.
pub fn look_palette(look: &Look) -> Vec<Rgba> {
    look_colors(look).into_iter().map(|(_, c)| c).collect()
}

/// The names of a fighter look's colours, in [`look_palette`] order.
pub fn look_names(look: &Look) -> Vec<String> {
    look_colors(look).into_iter().map(|(n, _)| n).collect()
}

/// `--palette`: a fighter look's name (its colours come named), or colours as `#rrggbb` (the `#` may be left out)
/// split by commas.
pub fn parse_palette(v: &str) -> Result<Palette, String> {
    if let Some(look) = Look::preset(v) {
        let (names, colors) = look_colors(&look).into_iter().unzip();
        return Ok(Palette { colors, names });
    }
    let colors = v
        .split(',')
        .map(|c| hex(c.trim()).map_err(|e| format!("--palette: {e}")))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(Palette {
        names: vec![String::new(); colors.len()],
        colors,
    })
}

// --- Arguments --------------------------------------------------------------------------------------------------

fn value<'a>(flag: &str, v: Option<&'a String>) -> Result<&'a String, String> {
    v.ok_or(format!("{flag} needs a value\n{USAGE}"))
}

fn number<T: std::str::FromStr>(flag: &str, v: Option<&String>) -> Result<T, String> {
    value(flag, v)?
        .parse()
        .map_err(|_| format!("{flag} expects a whole number\n{USAGE}"))
}

fn rect(flag: &str, v: Option<&String>) -> Result<[u32; 4], String> {
    let v = value(flag, v)?;
    let n: Vec<u32> = v
        .split(',')
        .map(|p| p.trim().parse())
        .collect::<Result<_, _>>()
        .map_err(|_| format!("{flag} expects X,Y,W,H: {v}"))?;
    n.try_into()
        .map_err(|_| format!("{flag} expects X,Y,W,H: {v}"))
}

fn pair(flag: &str, v: Option<&String>) -> Result<(u32, u32), String> {
    let v = value(flag, v)?;
    v.split_once('x')
        .and_then(|(w, h)| Some((w.parse().ok()?, h.parse().ok()?)))
        .filter(|&(w, h)| w > 0 && h > 0)
        .ok_or(format!("{flag} expects WxH: {v}"))
}

/// `X,Y;X,Y;...`: cells by their column and row.
fn cells(flag: &str, v: Option<&String>) -> Result<Vec<(u32, u32)>, String> {
    let v = value(flag, v)?;
    v.split(';')
        .filter(|c| !c.trim().is_empty())
        .map(|c| {
            c.split_once(',')
                .and_then(|(x, y)| Some((x.trim().parse().ok()?, y.trim().parse().ok()?)))
                .ok_or(format!("{flag} expects X,Y;X,Y: {c}"))
        })
        .collect()
}

/// Parse the arguments after `pixfx pixel`.
pub fn parse_args(args: &[String]) -> Result<Command, String> {
    let (verb, rest) = args.split_first().ok_or(USAGE)?;
    let mut out = PathBuf::from("target/pixel");
    let mut files: Vec<PathBuf> = Vec::new();
    let (mut crop, mut region, mut cut, mut height, mut palette) = (None, None, None, None, None);
    let (mut flip, mut rects, mut frames, mut pixel_lines) = (false, false, false, false);
    let (mut max_colors, mut max_side, mut gap, mut scale) =
        (32, ViewOptions::default().max_side, 2, 8);
    let (mut cell, mut grid, mut directions, mut bones) = (None, None, None, None);
    let mut intended = Vec::new();
    let mut it = rest.iter();
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "--out" => out = value(arg, it.next())?.into(),
            "--crop" => crop = Some(rect(arg, it.next())?),
            "--region" => region = Some(rect(arg, it.next())?),
            "--flip" => flip = true,
            "--cut" => cut = Some(number(arg, it.next())?),
            "--height" => height = Some(number(arg, it.next())?),
            "--palette" => palette = Some(parse_palette(value(arg, it.next())?)?),
            "--max-colors" => max_colors = number(arg, it.next())?,
            "--rects" => rects = true,
            "--max-side" => max_side = number(arg, it.next())?,
            "--frames" => frames = true,
            "--intended" => intended = cells(arg, it.next())?,
            "--cell" => cell = Some(pair(arg, it.next())?),
            "--grid" => grid = Some(pair(arg, it.next())?),
            "--gap" => gap = number(arg, it.next())?,
            "--scale" => scale = number(arg, it.next())?,
            "--pixel-lines" => pixel_lines = true,
            "--directions" => directions = Some(PathBuf::from(value(arg, it.next())?)),
            "--bones" => bones = Some(PathBuf::from(value(arg, it.next())?)),
            flag if flag.starts_with("--") => return Err(format!("unknown flag {flag}\n{USAGE}")),
            file => files.push(PathBuf::from(file)),
        }
    }
    let one = |files: &[PathBuf], what: &str| -> Result<PathBuf, String> {
        match files {
            [f] => Ok(f.clone()),
            _ => Err(format!("pixfx pixel {verb} takes one {what}\n{USAGE}")),
        }
    };
    let verb = match verb.as_str() {
        "clean" => Verb::Clean(CleanArgs {
            picture: one(&files, "picture")?,
            crop,
            flip,
            cut,
            height,
            palette,
            max_colors,
        }),
        "read" => Verb::Read {
            sprite: one(&files, "sprite")?,
            palette,
            region,
            rects,
        },
        "draw" => Verb::Draw {
            text: one(&files, "text file")?,
        },
        "view" => Verb::View {
            sprite: one(&files, "sprite")?,
            region,
            max_side,
        },
        "critique" if !files.is_empty() => Verb::Critique {
            sprites: files,
            frames,
            intended,
        },
        "sheet" => {
            if !files.is_empty() {
                return Err(format!(
                    "pixfx pixel sheet takes no file but its flags\n{USAGE}"
                ));
            }
            let (cell, grid) = match (cell, grid, &directions) {
                (Some(c), Some(g), None) => (c, g),
                (None, None, Some(_)) => ((0, 0), (3, 3)),
                _ => {
                    return Err(format!(
                        "sheet needs --cell WxH --grid CxR, or --directions\n{USAGE}"
                    ));
                }
            };
            Verb::Sheet(SheetArgs {
                cell,
                grid,
                gap,
                scale,
                pixel_lines,
                directions,
                bones,
            })
        }
        "cut" => match files.as_slice() {
            [drawn, layout] => Verb::Cut {
                drawn: drawn.clone(),
                layout: layout.clone(),
            },
            _ => {
                return Err(format!(
                    "pixfx pixel cut takes <drawn.png> <sheet.json>\n{USAGE}"
                ));
            }
        },
        "parts" => match files.as_slice() {
            [sprite, spec] => Verb::Parts {
                sprite: sprite.clone(),
                spec: spec.clone(),
                palette,
            },
            _ => {
                return Err(format!(
                    "pixfx pixel parts takes <sprite.png> <spec.txt>\n{USAGE}"
                ));
            }
        },
        _ => return Err(USAGE.to_string()),
    };
    Ok(Command { verb, out })
}

// --- The verbs' work --------------------------------------------------------------------------------------------

/// What `clean` made and how sure it is of the grid it found.
#[derive(Clone, Debug, PartialEq)]
pub struct Cleaned {
    /// One pixel a cell, on the palette or merged down to `max_colors`.
    pub sprite: RgbaImage,
    /// The picture's pixels per cell.
    pub pitch: f64,
    /// The mean confidence of the cells, 0..1.
    pub confidence: f64,
}

/// An image model's picture as a clean sprite: cropped, mirrored, cut from its background, down to its pixel grid
/// (`recover`), resampled to `height` rows, then snapped to the palette or merged down to `max_colors` colours.
pub fn clean(img: &RgbaImage, a: &CleanArgs) -> Result<Cleaned, String> {
    let img = match a.crop {
        Some([x, y, w, h]) => img.crop(x, y, w, h),
        None => img.clone(),
    };
    let img = if a.flip { img.flipped() } else { img };
    let img = match a.cut {
        Some(tolerance) => pixfx_pixel::cut(&img, tolerance),
        None => img,
    };
    if img.w == 0 || img.h == 0 {
        return Err("nothing left of the picture once cut".to_string());
    }
    let found = recover(&img);
    let sprite = match a.height {
        Some(h) => fit_height(&found.picture, h),
        None => found.picture,
    };
    let sprite = match &a.palette {
        Some(p) => snap(&sprite, &p.colors, SnapMode::Nearest),
        None => reduce(&sprite, a.max_colors, 0.03),
    };
    Ok(Cleaned {
        sprite,
        pitch: found.pitch,
        confidence: found.total,
    })
}

/// A sprite (one pixel a cell) as a map on `palette`, or on its own colours when there is none.
pub fn map_of(sprite: &RgbaImage, palette: Option<&Palette>) -> (Map, Palette) {
    let palette = palette.cloned().unwrap_or_else(|| {
        let colors = colors(sprite);
        Palette {
            names: vec![String::new(); colors.len()],
            colors,
        }
    });
    (
        shrink(&nearest(sprite, &palette.colors), sprite.w, 1.0),
        palette,
    )
}

/// A sprite as text (`draw` takes it back): a letter per cell under a legend, named when the palette is a look's,
/// and with `rects`, the cells merged into rects. Only `region` when one is given; its cells count from 0.
pub fn read(
    sprite: &RgbaImage,
    palette: Option<&Palette>,
    region: Option<[u32; 4]>,
    rects_too: bool,
) -> Result<String, String> {
    let sprite = match region {
        Some([x, y, w, h]) => sprite.crop(x, y, w, h),
        None => sprite.clone(),
    };
    let (map, palette) = map_of(&sprite, palette);
    let names: Vec<&str> = palette.names.iter().map(String::as_str).collect();
    let mut out = text_named(&map, &palette.colors, &names).map_err(|e| {
        format!("{e}: give --palette (hex colours or a look), or `clean` the picture first")
    })?;
    if rects_too {
        let letters: Vec<char> = LEGEND.chars().collect();
        out.push_str("rects: x y w h letter\n");
        for r in rects(&map) {
            let letter = letters.get(r.value).copied().unwrap_or('?');
            out.push_str(&format!("{} {} {} {} {letter}\n", r.x, r.y, r.w, r.h));
        }
    }
    Ok(out)
}

/// The sprite a text grid (as `read` writes it, edited or not) draws.
pub fn draw(text: &str) -> Result<RgbaImage, String> {
    let (map, palette) = pixfx_pixel::parse(text).map_err(|e| e.to_string())?;
    Ok(map.paint(&palette))
}

/// A sprite as a picture sized for an agent to look at.
pub fn view(sprite: &RgbaImage, region: Option<[u32; 4]>, max_side: u32) -> View {
    let (map, palette) = map_of(sprite, None);
    let opts = ViewOptions {
        max_side,
        region,
        ..ViewOptions::default()
    };
    pixfx_pixel::view::view(&map, &palette.colors, &opts)
}

/// A blank sheet: `directions` round a sprite when given, else `cell` x `grid` cells; `bones` drawn into each.
pub fn sheet(
    a: &SheetArgs,
    sprite: Option<&RgbaImage>,
    bones: &[[(u32, u32); 2]],
) -> (Layout, RgbaImage) {
    let (layout, mut img) = match sprite {
        Some(s) => directions(s, a.scale),
        None => {
            let layout = Layout {
                cell_w: a.cell.0,
                cell_h: a.cell.1,
                cols: a.grid.0,
                rows: a.grid.1,
                gap: a.gap,
                scale: a.scale,
            };
            (layout, blank(&layout, a.pixel_lines))
        }
    };
    if !bones.is_empty() {
        for row in 0..layout.rows {
            for col in 0..layout.cols {
                guide(&mut img, &layout, col, row, &Guide::Bones(bones.to_vec()));
            }
        }
    }
    (layout, img)
}

/// Each cell an image model drew into a sheet, row by row, down to one pixel a cell.
pub fn cut(drawn: &RgbaImage, layout: &Layout) -> Vec<RgbaImage> {
    pixfx_pixel::sheet::cut(drawn, layout)
        .iter()
        .map(|cell| recover(cell).picture)
        .collect()
}

/// A sprite split by a spec (its colour letters are those `read` shows with the same palette).
pub fn parts(
    sprite: &RgbaImage,
    spec: &str,
    palette: Option<&Palette>,
) -> Result<(Vec<Part>, Palette), String> {
    let spec = parse_spec(spec).map_err(|e| e.to_string())?;
    let (map, palette) = map_of(sprite, palette);
    let parts = split(&map, &spec).map_err(|e| e.to_string())?;
    Ok((parts, palette))
}

/// A part as one line: `name parent x,y,w,h pivot`.
pub fn part_line(p: &Part) -> String {
    let [x, y, w, h] = p.bbox;
    let pivot = p
        .pivot
        .map_or("-".to_string(), |(px, py)| format!("{px},{py}"));
    format!(
        "{} {} {x},{y},{w},{h} {pivot}",
        p.name,
        p.parent.as_deref().unwrap_or("-")
    )
}

/// A critique report as lines an agent reads: one per issue, then the score.
pub fn report_text(r: &Report) -> String {
    let mut out = String::new();
    for i in &r.issues {
        let cells: Vec<String> = i.cells.iter().map(|(x, y)| format!("{x},{y}")).collect();
        out.push_str(&format!(
            "{:?} {}: {}\n  cells {}\n  fix: {}\n",
            i.severity,
            i.check,
            i.message,
            if cells.is_empty() {
                "-".to_string()
            } else {
                cells.join(" ")
            },
            i.fix
        ));
    }
    out.push_str(&format!("score {}\n", r.score));
    out
}

// --- The shell --------------------------------------------------------------------------------------------------

fn read_image(path: &Path) -> Result<RgbaImage, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("reading {}: {e}", path.display()))?;
    decode_png_rgba(&bytes).map_err(|e| format!("{}: {e}", path.display()))
}

fn read_text(path: &Path) -> Result<String, String> {
    std::fs::read_to_string(path).map_err(|e| format!("reading {}: {e}", path.display()))
}

fn write_image(path: &Path, img: &RgbaImage) -> Result<(), String> {
    write_png_rgba(path, img.w, img.h, &img.rgba)
        .map_err(|e| format!("writing {}: {e}", path.display()))?;
    println!("{}", path.display());
    Ok(())
}

fn write_text(path: &Path, text: &str) -> Result<(), String> {
    std::fs::write(path, text).map_err(|e| format!("writing {}: {e}", path.display()))?;
    println!("{}", path.display());
    Ok(())
}

fn stem(path: &Path) -> String {
    path.file_stem()
        .map_or("picture".into(), |s| s.to_string_lossy().into_owned())
}

/// Run a parsed command: read its files, do its work, write into `out`. `Ok(false)` when a critique found an
/// error (the process exits 1).
pub fn run(cmd: &Command) -> Result<bool, String> {
    let out = &cmd.out;
    std::fs::create_dir_all(out).map_err(|e| format!("creating {}: {e}", out.display()))?;
    match &cmd.verb {
        Verb::Clean(a) => {
            let c = clean(&read_image(&a.picture)?, a)?;
            println!(
                "pitch {:.2}, cells {}x{}, confidence {:.2}",
                c.pitch, c.sprite.w, c.sprite.h, c.confidence
            );
            write_image(
                &out.join(format!("{}.clean.png", stem(&a.picture))),
                &c.sprite,
            )?;
        }
        Verb::Read {
            sprite,
            palette,
            region,
            rects,
        } => {
            let listing = read(&read_image(sprite)?, palette.as_ref(), *region, *rects)?;
            print!("{listing}");
            write_text(&out.join(format!("{}.txt", stem(sprite))), &listing)?;
        }
        Verb::Draw { text } => {
            write_image(
                &out.join(format!("{}.png", stem(text))),
                &draw(&read_text(text)?)?,
            )?;
        }
        Verb::View {
            sprite,
            region,
            max_side,
        } => {
            let v = view(&read_image(sprite)?, *region, *max_side);
            if v.overview {
                println!(
                    "overview: a pixel is {0}x{0} cells; look closer with --region",
                    v.scale
                );
            } else {
                println!("{} screen pixels a cell", v.scale);
            }
            write_image(&out.join(format!("{}.view.png", stem(sprite))), &v.image)?;
        }
        Verb::Critique {
            sprites,
            frames,
            intended,
        } => {
            let imgs = sprites
                .iter()
                .map(|p| read_image(p))
                .collect::<Result<Vec<_>, _>>()?;
            let mut reports = Vec::new();
            for (p, img) in sprites.iter().zip(&imgs) {
                reports.push((p.display().to_string(), critique(img, intended)));
            }
            if *frames {
                reports.push(("frames".to_string(), critique_frames(&imgs)));
            }
            let mut ok = true;
            for (name, r) in &reports {
                println!("{name}");
                print!("{}", report_text(r));
                ok &= !r.issues.iter().any(|i| i.severity == Severity::Error);
            }
            return Ok(ok);
        }
        Verb::Sheet(a) => {
            let sprite = a.directions.as_deref().map(read_image).transpose()?;
            let bones = match &a.bones {
                Some(p) => parse_bones(&read_text(p)?)?,
                None => Vec::new(),
            };
            let (layout, img) = sheet(a, sprite.as_ref(), &bones);
            write_image(&out.join("sheet.png"), &img)?;
            write_text(&out.join("sheet.json"), &layout_json(&layout))?;
        }
        Verb::Cut { drawn, layout } => {
            let layout = parse_layout(&read_text(layout)?)?;
            let cells = cut(&read_image(drawn)?, &layout);
            for (i, cell) in cells.iter().enumerate() {
                let (c, r) = (i as u32 % layout.cols.max(1), i as u32 / layout.cols.max(1));
                write_image(&out.join(format!("cell-{c}-{r}.png")), cell)?;
            }
        }
        Verb::Parts {
            sprite,
            spec,
            palette,
        } => {
            let (parts, palette) =
                parts(&read_image(sprite)?, &read_text(spec)?, palette.as_ref())?;
            let names: Vec<&str> = palette.names.iter().map(String::as_str).collect();
            for p in &parts {
                println!("{}", part_line(p));
                write_image(
                    &out.join(format!("{}.png", p.name)),
                    &p.map.paint(&palette.colors),
                )?;
                let listing =
                    text_named(&p.map, &palette.colors, &names).map_err(|e| e.to_string())?;
                write_text(&out.join(format!("{}.txt", p.name)), &listing)?;
            }
        }
    }
    Ok(true)
}
