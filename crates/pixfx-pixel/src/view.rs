//! A grid shown to the agent as a picture it can look at, sized to what it can afford to look at: [`view`] picks
//! the biggest whole scale that fits a side budget, and [`zoom`] draws a grid at a scale it is given.

use crate::{Map, Rgba, RgbaImage};

/// Cells between the brighter grid lines [`zoom`] draws.
pub const MAJOR: u32 = 8;
/// The smallest scale [`view`] draws grid lines at; below it, cells touch.
pub const GRID_FROM: u32 = 8;

/// Grid lines: between every cell, and brighter every few cells.
const MINOR: [u8; 4] = [96, 96, 112, 255];
const MAJOR_LINE: [u8; 4] = [200, 200, 220, 255];
/// An empty cell is a checker of these two, so it reads as empty rather than black.
const EMPTY: [[u8; 4]; 2] = [[40, 40, 48, 255], [52, 52, 62, 255]];

/// What [`view`] may spend.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ViewOptions {
    /// The longest side of the picture, in pixels.
    pub max_side: u32,
    /// The most screen pixels a cell gets across.
    pub max_scale: u32,
    /// Only these cells, `[x, y, w, h]`, clipped to the map: how an agent looks closer at a big map.
    pub region: Option<[u32; 4]>,
    /// Cells between the brighter grid lines, counted from the map's top left (not the region's); 0 for none.
    pub every: u32,
}

impl Default for ViewOptions {
    fn default() -> Self {
        ViewOptions {
            max_side: 512,
            max_scale: 16,
            region: None,
            every: MAJOR,
        }
    }
}

/// A picture of a map.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct View {
    pub image: RgbaImage,
    /// Screen pixels per cell across; when `overview`, cells per screen pixel across instead.
    pub scale: u32,
    /// The map did not fit even at one pixel a cell: each pixel is the average of `scale` x `scale` cells, with no
    /// grid lines. Look closer with a `region`.
    pub overview: bool,
}

/// `map` (or its `region`) as a picture within `opts`: the largest whole scale up to `max_scale` whose picture fits
/// `max_side`, each cell a square in its `palette` colour and an empty cell a dark checker. From [`GRID_FROM`] up
/// the cells sit inside one-pixel grid lines ([`zoom`]), brighter every `every` cells; below it they touch. A map
/// too big even at one pixel a cell comes back as an `overview`.
pub fn view(map: &Map, palette: &[Rgba], opts: &ViewOptions) -> View {
    let (map, origin) = match opts.region {
        Some(region) => crop(map, region),
        None => (map.clone(), (0, 0)),
    };
    let side = u64::from(map.w.max(map.h));
    let max_side = u64::from(opts.max_side.max(1));
    let fits = |s: u32| {
        let s = u64::from(s);
        if s >= u64::from(GRID_FROM) {
            side * (s + 1) < max_side
        } else {
            side * s <= max_side
        }
    };
    match (1..=opts.max_scale.max(1)).rev().find(|&s| fits(s)) {
        Some(scale) if scale >= GRID_FROM => View {
            image: grid(&map, palette, scale, opts.every, origin),
            scale,
            overview: false,
        },
        Some(scale) => View {
            image: blocks(&map, palette, scale),
            scale,
            overview: false,
        },
        None => {
            let per = side.div_ceil(max_side) as u32;
            View {
                image: overview(&map, palette, per),
                scale: per,
                overview: true,
            }
        }
    }
}

/// `map` blown up: each cell a `scale` x `scale` square in its `palette` colour (an empty cell a dark checker), a
/// one-pixel grid line round every cell and a brighter one every [`MAJOR`] cells, counted from the top left. An
/// empty map gives the bare grid: a sheet to hand an image model to draw into.
pub fn zoom(map: &Map, palette: &[Rgba], scale: u32) -> RgbaImage {
    grid(map, palette, scale, MAJOR, (0, 0))
}

/// The cells of `map` in `[x, y, w, h]`, clipped to it, and where they start.
fn crop(map: &Map, [x, y, w, h]: [u32; 4]) -> (Map, (u32, u32)) {
    let (x, y) = (x.min(map.w), y.min(map.h));
    let (w, h) = (w.min(map.w - x), h.min(map.h - y));
    let mut out = Map::empty(w, h);
    for cy in 0..h {
        for cx in 0..w {
            let cell = map.get(i64::from(x + cx), i64::from(y + cy));
            if let Some(c) = out.cells.get_mut(cy as usize * w as usize + cx as usize) {
                *c = cell;
            }
        }
    }
    (out, (x, y))
}

/// A cell's colour: its palette colour, opaque, or the checker when it is empty.
fn cell(map: &Map, palette: &[Rgba], cx: u32, cy: u32) -> [u8; 4] {
    match map
        .get(i64::from(cx), i64::from(cy))
        .and_then(|i| palette.get(i))
    {
        Some(&Rgba(r, g, b, _)) => [r, g, b, 255],
        None => EMPTY[((cx + cy) % 2) as usize],
    }
}

/// Cells as squares inside grid lines, brighter every `every` cells counted from `origin`.
fn grid(map: &Map, palette: &[Rgba], scale: u32, every: u32, (ox, oy): (u32, u32)) -> RgbaImage {
    let scale = scale.max(1);
    let step = scale + 1;
    let major = |at: u32| every > 0 && at.is_multiple_of(every);
    let mut img = RgbaImage::new(map.w * step + 1, map.h * step + 1);
    for y in 0..img.h {
        for x in 0..img.w {
            let (on_x, on_y) = (x % step == 0, y % step == 0);
            let p = if on_x || on_y {
                if (on_x && major(ox + x / step)) || (on_y && major(oy + y / step)) {
                    MAJOR_LINE
                } else {
                    MINOR
                }
            } else {
                cell(map, palette, x / step, y / step)
            };
            img.set(x, y, p);
        }
    }
    img
}

/// Cells as touching `scale` x `scale` squares.
fn blocks(map: &Map, palette: &[Rgba], scale: u32) -> RgbaImage {
    let mut img = RgbaImage::new(map.w * scale, map.h * scale);
    for y in 0..img.h {
        for x in 0..img.w {
            img.set(x, y, cell(map, palette, x / scale, y / scale));
        }
    }
    img
}

/// Each pixel the average colour of `per` x `per` cells, or the checker when fewer than half of them are painted.
fn overview(map: &Map, palette: &[Rgba], per: u32) -> RgbaImage {
    let per = per.max(1);
    let mut img = RgbaImage::new(map.w.div_ceil(per), map.h.div_ceil(per));
    for y in 0..img.h {
        for x in 0..img.w {
            let (mut sum, mut painted, mut total) = ([0u64; 3], 0u64, 0u64);
            for cy in y * per..((y + 1) * per).min(map.h) {
                for cx in x * per..((x + 1) * per).min(map.w) {
                    total += 1;
                    if let Some(&Rgba(r, g, b, _)) = map
                        .get(i64::from(cx), i64::from(cy))
                        .and_then(|i| palette.get(i))
                    {
                        painted += 1;
                        for (s, v) in sum.iter_mut().zip([r, g, b]) {
                            *s += u64::from(v);
                        }
                    }
                }
            }
            let p = if painted > 0 && painted * 2 >= total {
                let avg = |s: u64| ((s + painted / 2) / painted) as u8;
                [avg(sum[0]), avg(sum[1]), avg(sum[2]), 255]
            } else {
                EMPTY[((x + y) % 2) as usize]
            };
            img.set(x, y, p);
        }
    }
    img
}
