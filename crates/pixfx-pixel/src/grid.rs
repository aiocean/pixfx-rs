//! A picture as a grid of palette indices: matched, shrunk to its pixel grid, merged into rects.

use crate::{OPAQUE, Rgba, RgbaImage, distance};

/// For each pixel of `img`, row by row: the index of the nearest `palette` colour, or `None` where it is
/// transparent.
pub fn nearest(img: &RgbaImage, palette: &[Rgba]) -> Vec<Option<usize>> {
    img.rgba
        .as_chunks::<4>()
        .0
        .iter()
        .map(|&[r, g, b, a]| {
            if a < OPAQUE {
                return None;
            }
            (0..palette.len())
                .min_by_key(|&i| palette.get(i).map_or(u32::MAX, |&c| distance([r, g, b], c)))
        })
        .collect()
}

/// The distinct colours of `img`'s painted pixels (alpha at least half), in the order they first appear row by
/// row: the palette of a picture that already is pixel art, such as a game's sprite.
pub fn colors(img: &RgbaImage) -> Vec<Rgba> {
    let mut out: Vec<Rgba> = Vec::new();
    for &[r, g, b, a] in img.rgba.as_chunks::<4>().0 {
        let c = Rgba(r, g, b, 255);
        if a >= OPAQUE && !out.contains(&c) {
            out.push(c);
        }
    }
    out
}

/// A value map: `w` x `h` cells, row by row, each a palette index or `None` (empty).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Map {
    pub w: u32,
    pub h: u32,
    pub cells: Vec<Option<usize>>,
}

impl Map {
    /// A `w` x `h` map with every cell empty.
    pub fn empty(w: u32, h: u32) -> Map {
        Map {
            w,
            h,
            cells: vec![None; w as usize * h as usize],
        }
    }

    /// The map at one image pixel per cell, each cell in its `palette` colour (opaque) or transparent.
    pub fn paint(&self, palette: &[Rgba]) -> RgbaImage {
        let mut img = RgbaImage::new(self.w, self.h);
        for y in 0..self.h {
            for x in 0..self.w {
                if let Some(Rgba(r, g, b, _)) = self
                    .get(i64::from(x), i64::from(y))
                    .and_then(|i| palette.get(i).copied())
                {
                    img.set(x, y, [r, g, b, 255]);
                }
            }
        }
        img
    }

    /// The cell at (x, y); `None` when it is empty or outside the map.
    pub fn get(&self, x: i64, y: i64) -> Option<usize> {
        if x < 0 || y < 0 || x >= i64::from(self.w) || y >= i64::from(self.h) {
            return None;
        }
        self.cells
            .get(y as usize * self.w as usize + x as usize)
            .copied()
            .flatten()
    }
}

/// Shrink a `w`-wide value map by `px` (source cells per cell of the result, 1 or more): each result cell takes
/// the value most of its square holds, empty included, so outlines and edges stay crisp instead of blurring.
pub fn shrink(cells: &[Option<usize>], w: u32, px: f64) -> Map {
    let px = px.max(1.0);
    let h = if w == 0 {
        0
    } else {
        (cells.len() / w as usize) as u32
    };
    let source = Map {
        w,
        h,
        cells: cells.to_vec(),
    };
    let (out_w, out_h) = (
        libm::ceil(f64::from(w) / px) as u32,
        libm::ceil(f64::from(h) / px) as u32,
    );
    let edge = |i: u32| libm::floor(f64::from(i) * px) as i64;
    let mut out = Vec::with_capacity(out_w as usize * out_h as usize);
    for oy in 0..out_h {
        for ox in 0..out_w {
            let mut votes: Vec<(Option<usize>, u32)> = Vec::new();
            for y in edge(oy)..edge(oy + 1).max(edge(oy) + 1) {
                for x in edge(ox)..edge(ox + 1).max(edge(ox) + 1) {
                    if x >= i64::from(w) || y >= i64::from(h) {
                        continue;
                    }
                    let v = source.get(x, y);
                    match votes.iter_mut().find(|(k, _)| *k == v) {
                        Some((_, n)) => *n += 1,
                        None => votes.push((v, 1)),
                    }
                }
            }
            // ties go to the first value met, so the result does not depend on hash order
            let best =
                votes.iter().fold(
                    None,
                    |best: Option<(Option<usize>, u32)>, &(v, n)| match best {
                        Some((_, m)) if m >= n => best,
                        _ => Some((v, n)),
                    },
                );
            out.push(best.and_then(|(v, _)| v));
        }
    }
    Map {
        w: out_w,
        h: out_h,
        cells: out,
    }
}

/// A rect of equal cells: top-left (x, y), `w` x `h`, and the value they hold.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Run {
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
    pub value: usize,
}

/// Merge a map's equal cells into rects: the longest run along a row, grown down while the rows below repeat it.
/// Every non-empty cell lands in exactly one rect.
pub fn rects(map: &Map) -> Vec<Run> {
    let (w, h) = (map.w, map.h);
    let mut taken = vec![false; w as usize * h as usize];
    let free = |taken: &[bool], x: u32, y: u32| {
        !taken
            .get(y as usize * w as usize + x as usize)
            .copied()
            .unwrap_or(true)
    };
    let mut out = Vec::new();
    for y in 0..h {
        let mut x = 0;
        while x < w {
            let Some(value) = map
                .get(i64::from(x), i64::from(y))
                .filter(|_| free(&taken, x, y))
            else {
                x += 1;
                continue;
            };
            let same = |taken: &[bool], cx: u32, cy: u32| {
                free(taken, cx, cy) && map.get(i64::from(cx), i64::from(cy)) == Some(value)
            };
            let mut run_w = 1;
            while x + run_w < w && same(&taken, x + run_w, y) {
                run_w += 1;
            }
            let mut run_h = 1;
            while y + run_h < h && (x..x + run_w).all(|cx| same(&taken, cx, y + run_h)) {
                run_h += 1;
            }
            for cy in y..y + run_h {
                for cx in x..x + run_w {
                    if let Some(t) = taken.get_mut(cy as usize * w as usize + cx as usize) {
                        *t = true;
                    }
                }
            }
            out.push(Run {
                x,
                y,
                w: run_w,
                h: run_h,
                value,
            });
            x += run_w;
        }
    }
    out
}
