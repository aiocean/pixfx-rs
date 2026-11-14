//! A grid shown back to the agent: as letters it can read cell by cell, and as a big picture it can look at.

use core::fmt;

use pixfx_draw::color::to_hex;

use crate::{Map, Rgba, RgbaImage};

/// The letters [`text`] gives palette colours, in order; an empty cell is `.`.
pub const LEGEND: &str = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";

/// A palette with more colours than [`LEGEND`] has letters.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TooManyColors(pub usize);

impl fmt::Display for TooManyColors {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} colours, more than the {} letters a text grid has",
            self.0,
            LEGEND.len()
        )
    }
}

/// `map` as text: a head line (size, colours), one legend line per `palette` colour (`a #rrggbb`), then the grid
/// with its column numbers above (tens, then units) and its row number before each row, so every cell can be named
/// by (x, y). `.` is an empty cell.
pub fn text(map: &Map, palette: &[Rgba]) -> Result<String, TooManyColors> {
    if palette.len() > LEGEND.len() {
        return Err(TooManyColors(palette.len()));
    }
    let mut out = format!("{}x{}, {} colours, . empty\n", map.w, map.h, palette.len());
    for (letter, &c) in LEGEND.chars().zip(palette) {
        out.push_str(&format!("{letter} {}\n", to_hex(c)));
    }
    let pad = map.h.saturating_sub(1).to_string().len();
    let digits = |f: fn(u32) -> u32| -> String {
        (0..map.w).map(|x| char::from(b'0' + f(x) as u8)).collect()
    };
    if map.w > 10 {
        out.push_str(&format!("{:pad$} {}\n", "", digits(|x| x / 10 % 10)));
    }
    out.push_str(&format!("{:pad$} {}\n", "", digits(|x| x % 10)));
    let letters: Vec<char> = LEGEND.chars().collect();
    for y in 0..map.h {
        let row: String = (0..map.w)
            .map(|x| {
                map.get(i64::from(x), i64::from(y))
                    .and_then(|i| letters.get(i).copied())
                    .unwrap_or('.')
            })
            .collect();
        out.push_str(&format!("{y:>pad$} {row}\n"));
    }
    Ok(out)
}

/// Grid lines: between every cell, and brighter every [`MAJOR`] cells.
const MINOR: [u8; 4] = [96, 96, 112, 255];
const MAJOR_LINE: [u8; 4] = [200, 200, 220, 255];
/// Cells between the brighter grid lines.
pub const MAJOR: u32 = 8;
/// An empty cell is a checker of these two, so it reads as empty rather than black.
const EMPTY: [[u8; 4]; 2] = [[40, 40, 48, 255], [52, 52, 62, 255]];

/// `map` blown up: each cell a `scale` x `scale` square in its `palette` colour (an empty cell a dark checker), a
/// one-pixel grid line round every cell and a brighter one every [`MAJOR`] cells, counted from the top left. An
/// empty map gives the bare grid: a sheet to hand an image model to draw into.
pub fn zoom(map: &Map, palette: &[Rgba], scale: u32) -> RgbaImage {
    let scale = scale.max(1);
    let step = scale + 1;
    let mut img = RgbaImage::new(map.w * step + 1, map.h * step + 1);
    for y in 0..img.h {
        for x in 0..img.w {
            let (on_x, on_y) = (x % step == 0, y % step == 0);
            let p = if on_x || on_y {
                let major = (on_x && (x / step).is_multiple_of(MAJOR))
                    || (on_y && (y / step).is_multiple_of(MAJOR));
                if major { MAJOR_LINE } else { MINOR }
            } else {
                let (cx, cy) = (x / step, y / step);
                match map
                    .get(i64::from(cx), i64::from(cy))
                    .and_then(|i| palette.get(i))
                {
                    Some(&Rgba(r, g, b, _)) => [r, g, b, 255],
                    None => EMPTY[((cx + cy) % 2) as usize],
                }
            };
            img.set(x, y, p);
        }
    }
    img
}
