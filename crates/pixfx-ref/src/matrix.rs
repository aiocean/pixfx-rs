//! The frames as pixel matrices: every model pixel of every frame is empty, colour or light, with the exact value
//! the model drew and an index into a palette the whole sheet shares.
//!
//! The model's colours are nearly continuous (tens of thousands of distinct values on one sheet), so the exact value
//! stays the truth and nothing on screen changes. The palette is the handle on top of it: change one entry and every
//! frame's pixels that point at it change, which is how an effect is recoloured and what painter code is written
//! with. Colour and light have their own palettes, each built by [`pixfx_pixel::reduce::reduce`] over every frame's
//! pixels of that kind at once, so every entry is a colour the model drew. How far the palette misses the exact
//! values is measured, as a mean per channel (0..255).
//!
//! As text, a frame is two maps, colour and light, a letter per pixel from [`pixfx_pixel::LEGEND`] naming its palette
//! entry and `.` where the pixel is not of that kind.

use pixfx_pixel::reduce::reduce;
use pixfx_pixel::{LEGEND, Rgba};
use serde_json::{Value, json};

use crate::RgbaImage;
use crate::extract::Extracted;

/// Palette entries for colour pixels.
pub const COLOURS: usize = 32;
/// Palette entries for light pixels.
pub const LIGHTS: usize = 16;
/// Colours within this OKLab distance join one cluster before clusters are merged (as `pixfx-pixel`'s `clean`).
const JOIN: f64 = 0.03;

/// One model pixel.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Px {
    Empty,
    /// Drawn opaque: its colour as drawn, and its colour palette entry.
    Colour { rgb: [u8; 3], ink: u8 },
    /// See-through glow: what it adds over black, and its light palette entry.
    Light { rgb: [u8; 3], ink: u8 },
}

/// One frame's pixels, row by row.
#[derive(Clone, Debug, PartialEq)]
pub struct Frame {
    pub name: String,
    pub px: Vec<Px>,
}

/// Every frame of a sheet as a pixel matrix, `w` by `h`, with the shared palettes and how far they miss the exact
/// values (mean per channel, 0..255, over the pixels of each kind).
#[derive(Clone, Debug, PartialEq)]
pub struct Matrix {
    pub w: u32,
    pub h: u32,
    pub colours: Vec<[u8; 3]>,
    pub lights: Vec<[u8; 3]>,
    pub frames: Vec<Frame>,
    pub colour_err: f64,
    pub light_err: f64,
}

/// The pixels of one kind from every frame, stacked top to bottom into one picture: `pick` gives a pixel's value, or
/// `None` where it is not of that kind.
fn stack(ex: &Extracted, pick: impl Fn(&crate::extract::Cell, u32, u32) -> Option<[u8; 3]>) -> RgbaImage {
    let (w, h) = ex.size;
    let mut out = RgbaImage::new(w, h * ex.cells.len() as u32);
    for (i, c) in ex.cells.iter().enumerate() {
        for y in 0..h {
            for x in 0..w {
                if let Some([r, g, b]) = pick(c, x, y) {
                    out.set(x, i as u32 * h + y, [r, g, b, 255]);
                }
            }
        }
    }
    out
}

/// The palette of a stacked picture reduced to `n` colours, and each pixel's entry (row by row, `None` where empty).
fn palette(stacked: &RgbaImage, n: usize) -> (Vec<[u8; 3]>, Vec<Option<u8>>) {
    let reduced = reduce(stacked, n, JOIN);
    let mut entries: Vec<[u8; 3]> = Vec::new();
    let index = reduced
        .rgba
        .as_chunks::<4>()
        .0
        .iter()
        .map(|&[r, g, b, a]| {
            if a == 0 {
                return None;
            }
            let at = entries.iter().position(|e| *e == [r, g, b]).unwrap_or_else(|| {
                entries.push([r, g, b]);
                entries.len() - 1
            });
            Some(at as u8)
        })
        .collect();
    (entries, index)
}

/// Mean difference per channel between exact values and their palette entries.
fn miss(values: &RgbaImage, index: &[Option<u8>], entries: &[[u8; 3]]) -> f64 {
    let (mut sum, mut n) = (0u64, 0u64);
    for (p, i) in values.rgba.as_chunks::<4>().0.iter().zip(index) {
        if let Some(e) = i.and_then(|i| entries.get(usize::from(i))) {
            sum += (0..3).map(|c| u64::from(p[c].abs_diff(e[c]))).sum::<u64>();
            n += 3;
        }
    }
    if n == 0 { 0.0 } else { sum as f64 / n as f64 }
}

/// The frames of `ex` as pixel matrices with shared palettes of at most [`COLOURS`] and [`LIGHTS`] entries.
pub fn matrix(ex: &Extracted) -> Matrix {
    let (w, h) = ex.size;
    let colour = stack(ex, |c, x, y| c.colour.get(x, y).filter(|p| p[3] > 0).map(|p| [p[0], p[1], p[2]]));
    let light = stack(ex, |c, x, y| {
        c.light.get(x, y).filter(|p| p[0] > 0 || p[1] > 0 || p[2] > 0).map(|p| [p[0], p[1], p[2]])
    });
    let (colours, ci) = palette(&colour, COLOURS);
    let (lights, li) = palette(&light, LIGHTS);
    let per = (w * h) as usize;
    let frames = ex
        .cells
        .iter()
        .enumerate()
        .map(|(f, c)| {
            let px = (0..per)
                .map(|i| {
                    let at = f * per + i;
                    let value = |img: &RgbaImage| {
                        let [r, g, b, _] = img.get((i % w as usize) as u32, (i / w as usize) as u32).unwrap_or([0; 4]);
                        [r, g, b]
                    };
                    match (ci[at], li[at]) {
                        (Some(ink), _) => Px::Colour { rgb: value(&c.colour), ink },
                        (None, Some(ink)) => Px::Light { rgb: value(&c.light), ink },
                        (None, None) => Px::Empty,
                    }
                })
                .collect();
            Frame { name: c.name.clone(), px }
        })
        .collect();
    Matrix {
        w,
        h,
        colour_err: miss(&colour, &ci, &colours),
        light_err: miss(&light, &li, &lights),
        colours,
        lights,
        frames,
    }
}

/// A frame painted from the palettes instead of the exact values, over black: what the effect looks like through its
/// palette.
pub fn paletted(m: &Matrix, frame: &Frame) -> RgbaImage {
    let mut out = RgbaImage::new(m.w, m.h);
    for (i, p) in frame.px.iter().enumerate() {
        let rgb = match *p {
            Px::Empty => [0; 3],
            Px::Colour { ink, .. } => m.colours.get(usize::from(ink)).copied().unwrap_or([0; 3]),
            Px::Light { ink, .. } => m.lights.get(usize::from(ink)).copied().unwrap_or([0; 3]),
        };
        out.set(i as u32 % m.w, i as u32 / m.w, [rgb[0], rgb[1], rgb[2], 255]);
    }
    out
}

fn hex([r, g, b]: [u8; 3]) -> String {
    pixfx_draw::color::to_hex(Rgba(r, g, b, 255))
}

/// A frame's map of one kind as text rows: the palette entry's letter where the pixel is of that kind, `.` elsewhere.
fn rows(m: &Matrix, frame: &Frame, colour: bool) -> Vec<String> {
    let letters: Vec<char> = LEGEND.chars().collect();
    frame
        .px
        .chunks(m.w as usize)
        .map(|row| {
            row.iter()
                .map(|p| match (*p, colour) {
                    (Px::Colour { ink, .. }, true) | (Px::Light { ink, .. }, false) => {
                        letters.get(usize::from(ink)).copied().unwrap_or('?')
                    }
                    _ => '.',
                })
                .collect()
        })
        .collect()
}

/// The matrix as JSON (`matrix.json`): size; the palettes as `#rrggbb` with the letter each is written as and how
/// far they miss; per frame its colour and light maps as text rows and the exact values of its colour and light
/// pixels in reading order.
pub fn to_json(m: &Matrix) -> Value {
    let letters: Vec<char> = LEGEND.chars().collect();
    let entries = |p: &[[u8; 3]]| -> Vec<Value> {
        p.iter()
            .enumerate()
            .map(|(i, c)| json!({"letter": letters.get(i).map(char::to_string), "rgb": hex(*c)}))
            .collect()
    };
    json!({
        "w": m.w,
        "h": m.h,
        "palette": {
            "colour": entries(&m.colours),
            "light": entries(&m.lights),
            "colour_err": m.colour_err,
            "light_err": m.light_err,
        },
        "frames": m.frames.iter().map(|f| {
            let exact = |colour: bool| -> Vec<String> {
                f.px.iter().filter_map(|p| match (*p, colour) {
                    (Px::Colour { rgb, .. }, true) | (Px::Light { rgb, .. }, false) => Some(hex(rgb)),
                    _ => None,
                }).collect()
            };
            json!({
                "name": f.name,
                "colour": rows(m, f, true),
                "light": rows(m, f, false),
                "exact": {"colour": exact(true), "light": exact(false)},
            })
        }).collect::<Vec<_>>(),
    })
}
