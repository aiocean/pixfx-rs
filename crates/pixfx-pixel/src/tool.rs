//! The steps an agent takes with a picture, each a pure function its example script runs on files: [`clean`] an
//! image model's picture down to one pixel a cell, [`read`] a sprite as text and [`draw`] text back, [`view`] it at a
//! size worth looking at, make a blank [`sheet`] for a model to draw into and [`cut`] the drawn cells back, split it
//! into [`parts`]. Scripts: `crates/pixfx-pixel/examples/<step>.rs`.

use crate::critique::Report;
use crate::parts::{Part, parse_spec, split};
use crate::recover::{fit_height, recover};
use crate::reduce::{SnapMode, reduce, snap};
use crate::sheet::{CellBone, Guide, Layout, blank, bones_of, check, directions, guide};
use crate::view::{View, ViewOptions};
use crate::{LEGEND, Map, colors, nearest, rects, shrink, text_named};
use pixfx_draw::{Rgba, RgbaImage, hex};

/// Colours a grid is matched to, with a name for each the agent reads in the legend (empty when it has none).
#[derive(Clone, Debug, PartialEq)]
pub struct Palette {
    pub colors: Vec<Rgba>,
    pub names: Vec<String>,
}

impl Palette {
    /// Colours with a name each (a fighter look's, `Look::colors`).
    pub fn named(colors: Vec<(String, Rgba)>) -> Self {
        let (names, colors) = colors.into_iter().unzip();
        Self { colors, names }
    }

    /// Colours as `#rrggbb` (the `#` may be left out) split by commas, unnamed.
    pub fn parse_hex(v: &str) -> Result<Self, String> {
        let colors = v
            .split(',')
            .map(|c| hex(c.trim()).map_err(|e| format!("palette: {e}")))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self {
            names: vec![String::new(); colors.len()],
            colors,
        })
    }

    /// A palette file: a GIMP palette (`.gpl`: a `GIMP Palette` head, then `R G B name` lines, `#` comments) or a
    /// hex list (`.hex`: one `#rrggbb` a line, `#` optional, `;` comments).
    pub fn parse_file(text: &str) -> Result<Self, String> {
        let mut lines = text.lines().map(str::trim).filter(|l| !l.is_empty());
        let gimp = lines.clone().next() == Some("GIMP Palette");
        let mut out = Self {
            colors: Vec::new(),
            names: Vec::new(),
        };
        if gimp {
            lines.next();
        }
        for line in lines {
            if gimp {
                if line.starts_with('#') || line.contains(':') {
                    continue;
                }
                let mut words = line.split_whitespace();
                let mut channel = || -> Result<u8, String> {
                    words.next().and_then(|w| w.parse().ok()).ok_or(format!(
                        "palette: \"{line}\" is not R G B (0 to 255) and a name"
                    ))
                };
                let c = Rgba(channel()?, channel()?, channel()?, 255);
                out.colors.push(c);
                out.names.push(words.collect::<Vec<_>>().join(" "));
            } else {
                if line.starts_with(';') {
                    continue;
                }
                out.colors
                    .push(hex(line).map_err(|e| format!("palette: one #rrggbb a line: {e}"))?);
                out.names.push(String::new());
            }
        }
        if out.colors.is_empty() {
            return Err("palette: the file holds no colour".to_string());
        }
        Ok(out)
    }

    /// Ramps from base colours, each `steps` long: `name-1` (dark, cool) to `name-N` (light, warm).
    pub fn ramps(bases: &[(String, Rgba)], steps: usize) -> Self {
        let mut out = Self {
            colors: Vec::new(),
            names: Vec::new(),
        };
        for (name, base) in bases {
            for (i, c) in crate::ramp::ramp(*base, steps).into_iter().enumerate() {
                out.colors.push(c);
                out.names.push(format!("{name}-{}", i + 1));
            }
        }
        out
    }

    /// As a GIMP palette (`.gpl`) titled `title`, which [`Palette::parse_file`] reads back with its names.
    pub fn gpl(&self, title: &str) -> String {
        let mut out = format!("GIMP Palette\nName: {title}\n#\n");
        for (c, name) in self.colors.iter().zip(&self.names) {
            out.push_str(&format!("{:3} {:3} {:3}\t{name}\n", c.0, c.1, c.2));
        }
        out
    }
}

/// How [`clean`] treats a picture.
#[derive(Clone, Debug, PartialEq)]
pub struct CleanOptions {
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

impl Default for CleanOptions {
    fn default() -> Self {
        Self {
            crop: None,
            flip: false,
            cut: None,
            height: None,
            palette: None,
            max_colors: 32,
        }
    }
}

/// The blank [`sheet`] a model draws into: `cell` x `grid` cells, or round a sprite's eight directions.
#[derive(Clone, Debug, PartialEq)]
pub struct SheetOptions {
    pub cell: (u32, u32),
    pub grid: (u32, u32),
    pub gap: u32,
    pub scale: u32,
    pub pixel_lines: bool,
}

/// What `clean` made and how sure it is of the grid it found.
#[derive(Clone, Debug, PartialEq)]
pub struct Cleaned {
    /// One pixel a cell, on the palette or merged down to `max_colors`.
    pub sprite: RgbaImage,
    /// The picture's pixels per cell.
    pub pitch: f64,
    /// The mean confidence of the painted cells, 0..1.
    pub confidence: f64,
}

/// An image model's picture as a clean sprite: cropped, mirrored, cut from its background, down to its pixel grid
/// (`recover`), resampled to `height` rows, then snapped to the palette or merged down to `max_colors` colours.
pub fn clean(img: &RgbaImage, a: &CleanOptions) -> Result<Cleaned, String> {
    let img = match a.crop {
        Some([x, y, w, h]) => img.crop(x, y, w, h),
        None => img.clone(),
    };
    let img = if a.flip { img.flipped() } else { img };
    let img = match a.cut {
        Some(tolerance) => crate::cut(&img, tolerance),
        None => img,
    };
    if img.w == 0 || img.h == 0 {
        return Err("nothing left of the picture once cut".to_string());
    }
    let found = recover(&img);
    let confidence = painted_confidence(&found.picture, &found.confidence);
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
        confidence,
    })
}

/// The mean confidence of the painted cells only: background cells are always sure, and counting them would hide a
/// badly recovered figure on a big empty grid. 1 when nothing is painted.
fn painted_confidence(picture: &RgbaImage, confidence: &[f64]) -> f64 {
    let painted: Vec<f64> = picture
        .rgba
        .chunks(4)
        .zip(confidence)
        .filter(|(p, _)| p.get(3).is_some_and(|&a| a >= 128))
        .map(|(_, &c)| c)
        .collect();
    if painted.is_empty() {
        1.0
    } else {
        painted.iter().sum::<f64>() / painted.len() as f64
    }
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
        format!("{e}: give a palette (hex colours or a look), or `clean` the picture first")
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
    let (map, palette) = crate::parse(text).map_err(|e| e.to_string())?;
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
    crate::view::view(&map, &palette.colors, &opts)
}

/// A blank sheet: `directions` round a sprite when given (a 3x3 with a gap of 2: `gap` and `pixel_lines` do
/// not apply), else `cell` x `grid` cells; `bones` drawn into each. Refuses a sheet too big to draw.
pub fn sheet(
    a: &SheetOptions,
    sprite: Option<&RgbaImage>,
    bones: &[CellBone],
) -> Result<(Layout, RgbaImage), String> {
    let (layout, mut img) = match sprite {
        Some(s) => {
            let layout = Layout {
                cell_w: s.w,
                cell_h: s.h,
                cols: 3,
                rows: 3,
                gap: 2,
                scale: a.scale,
            };
            check(&layout)?;
            directions(s, a.scale)
        }
        None => {
            let layout = Layout {
                cell_w: a.cell.0,
                cell_h: a.cell.1,
                cols: a.grid.0,
                rows: a.grid.1,
                gap: a.gap,
                scale: a.scale,
            };
            check(&layout)?;
            (layout, blank(&layout, a.pixel_lines))
        }
    };
    if !bones.is_empty() {
        for row in 0..layout.rows {
            for col in 0..layout.cols {
                let own = bones_of(bones, col, row);
                if !own.is_empty() {
                    guide(&mut img, &layout, col, row, &Guide::Bones(own));
                }
            }
        }
    }
    Ok((layout, img))
}

/// Each cell an image model drew into a sheet, row by row, down to one pixel a cell.
pub fn cut(drawn: &RgbaImage, layout: &Layout) -> Vec<RgbaImage> {
    crate::sheet::cut(drawn, layout)
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
        .map_or("-".to_string(), |(px, py)| format!("{px:.2},{py:.2}"));
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
