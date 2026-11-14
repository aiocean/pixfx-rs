//! `pixfx pixel`: a picture as a pixel grid an agent reads (text with a colour legend, a zoomed PNG with grid
//! lines, the true-size pixels), or a blank grid to hand an image model to draw into.

use std::path::PathBuf;

use pixfx_draw::{Rgba, RgbaImage, hex};
use pixfx_fighter::look::Look;
use pixfx_pixel::{LEGEND, Map, colors, cut, nearest, shrink};

pub const USAGE: &str = "pixfx pixel <picture.png> [--crop X,Y,W,H] [--flip] [--cut TOLERANCE] [--px N] [--palette HEX,HEX,...|thunder|frost] [--rects] [--scale N] [--out DIR]
       pixfx pixel --blank WxH [--scale N] [--out DIR]";

/// What `pixfx pixel` was asked for.
#[derive(Clone, Debug, PartialEq)]
pub struct PixelArgs {
    /// The picture, or `None` for a blank grid.
    pub picture: Option<PathBuf>,
    /// The size of a blank grid.
    pub blank: Option<(u32, u32)>,
    pub crop: Option<[u32; 4]>,
    pub flip: bool,
    /// Cut the picture from its background (the top-left colour, give or take this much).
    pub cut: Option<u32>,
    /// The picture's pixels per grid cell.
    pub px: f64,
    /// The colours cells are matched to; `None` takes the picture's own.
    pub palette: Option<Vec<Rgba>>,
    /// Also print the grid merged into rects.
    pub rects: bool,
    /// Image pixels per cell in the zoomed PNG.
    pub scale: u32,
    pub out: PathBuf,
}

/// The colours of a fighter look, in the order it lists them: the palette its poses are drawn in.
pub fn look_palette(look: &Look) -> Vec<Rgba> {
    let mut out = Vec::new();
    out.extend(look.robe);
    out.extend(look.inner);
    out.extend(look.trim);
    out.extend(look.hair);
    out.extend(look.skin);
    out.push(look.boot);
    out.extend(look.blade);
    out.push(look.jade);
    out.extend(look.ribbon);
    out.push(look.glow);
    out.push(look.outline);
    let mut distinct: Vec<Rgba> = Vec::new();
    for c in out {
        if !distinct.contains(&c) {
            distinct.push(c);
        }
    }
    distinct
}

/// `--palette`: a fighter look's name, or colours as `#rrggbb` (the `#` may be left out) split by commas.
pub fn parse_palette(v: &str) -> Result<Vec<Rgba>, String> {
    if let Some(look) = Look::preset(v) {
        return Ok(look_palette(&look));
    }
    v.split(',')
        .map(|c| hex(c.trim()).map_err(|e| format!("--palette: {e}")))
        .collect()
}

fn rect(v: Option<&String>) -> Result<[u32; 4], String> {
    let v = v.ok_or(USAGE)?;
    let n: Vec<u32> = v
        .split(',')
        .map(|p| p.trim().parse())
        .collect::<Result<_, _>>()
        .map_err(|_| format!("expected X,Y,W,H: {v}"))?;
    n.try_into().map_err(|_| format!("expected X,Y,W,H: {v}"))
}

fn size(v: Option<&String>) -> Result<(u32, u32), String> {
    let v = v.ok_or(USAGE)?;
    v.split_once('x')
        .and_then(|(w, h)| Some((w.parse().ok()?, h.parse().ok()?)))
        .filter(|&(w, h)| w > 0 && h > 0)
        .ok_or(format!("expected WxH: {v}"))
}

fn number<T: std::str::FromStr>(flag: &str, v: Option<&String>) -> Result<T, String> {
    v.and_then(|v| v.parse().ok())
        .ok_or(format!("{flag} expects a number\n{USAGE}"))
}

/// Parse the arguments after `pixfx pixel`.
pub fn parse_args(args: &[String]) -> Result<PixelArgs, String> {
    let mut a = PixelArgs {
        picture: None,
        blank: None,
        crop: None,
        flip: false,
        cut: None,
        px: 1.0,
        palette: None,
        rects: false,
        scale: 8,
        out: PathBuf::from("target/pixel"),
    };
    let mut it = args.iter();
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "--blank" => a.blank = Some(size(it.next())?),
            "--crop" => a.crop = Some(rect(it.next())?),
            "--flip" => a.flip = true,
            "--cut" => a.cut = Some(number("--cut", it.next())?),
            "--px" => a.px = number("--px", it.next())?,
            "--palette" => a.palette = Some(parse_palette(it.next().ok_or(USAGE)?)?),
            "--rects" => a.rects = true,
            "--scale" => a.scale = number("--scale", it.next())?,
            "--out" => a.out = it.next().ok_or(USAGE)?.into(),
            flag if flag.starts_with("--") => return Err(format!("unknown flag {flag}\n{USAGE}")),
            _ if a.picture.is_none() => a.picture = Some(PathBuf::from(arg)),
            _ => return Err(USAGE.to_string()),
        }
    }
    if a.picture.is_some() == a.blank.is_some() {
        return Err(format!("give a picture or --blank WxH\n{USAGE}"));
    }
    if a.px < 1.0 {
        return Err("--px is the picture's pixels per cell: 1 or more".to_string());
    }
    Ok(a)
}

/// `img` as a grid on a palette, as `args` asks: cropped, mirrored, cut from its background, matched to the
/// palette (or its own colours) and shrunk by `px`. With its own colours a picture must have no more of them than
/// a text grid has letters: a picture with more (an image model's) needs a palette to be matched to.
pub fn grid(img: &RgbaImage, args: &PixelArgs) -> Result<(Map, Vec<Rgba>), String> {
    let img = match args.crop {
        Some([x, y, w, h]) => img.crop(x, y, w, h),
        None => img.clone(),
    };
    let img = if args.flip { img.flipped() } else { img };
    let img = match args.cut {
        Some(tolerance) => cut(&img, tolerance),
        None => img,
    };
    if img.w == 0 || img.h == 0 {
        return Err("nothing left of the picture once cut".to_string());
    }
    let palette = match &args.palette {
        Some(p) => p.clone(),
        None => {
            let own = colors(&img);
            if own.len() > LEGEND.len() {
                return Err(format!(
                    "the picture has {} colours: give --palette (hex colours or a look) to match it to",
                    own.len()
                ));
            }
            own
        }
    };
    let map = shrink(&nearest(&img, &palette), img.w, args.px);
    Ok((map, palette))
}
