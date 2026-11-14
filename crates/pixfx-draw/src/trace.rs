//! Tracing a picture drawn elsewhere (by an artist, or an image model) into pixels the engine paints: cut it out of
//! its background, match every pixel to a palette, shrink it to the engine's pixel grid, and merge equal pixels into
//! rects a painter fills. Also [`diff`]: how far two images are apart, to check that what the engine paints is the
//! picture that was traced. Pure: images come in as values and leave as values.

use crate::color::Rgba;

/// An image, 8 bits a channel RGBA, straight alpha, rows top to bottom.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RgbaImage {
    pub w: u32,
    pub h: u32,
    pub rgba: Vec<u8>,
}

/// Alpha at and above which a pixel counts as painted.
const OPAQUE: u8 = 128;

impl RgbaImage {
    /// A `w` x `h` image, all transparent.
    pub fn new(w: u32, h: u32) -> Self {
        RgbaImage {
            w,
            h,
            rgba: vec![0; w as usize * h as usize * 4],
        }
    }

    fn at(&self, x: u32, y: u32) -> Option<usize> {
        (x < self.w && y < self.h).then(|| (y as usize * self.w as usize + x as usize) * 4)
    }

    /// The pixel at (x, y); `None` outside the image.
    pub fn get(&self, x: u32, y: u32) -> Option<[u8; 4]> {
        let i = self.at(x, y)?;
        let p = self.rgba.get(i..i + 4)?;
        Some([p[0], p[1], p[2], p[3]])
    }

    /// Set the pixel at (x, y); nothing outside the image.
    pub fn set(&mut self, x: u32, y: u32, p: [u8; 4]) {
        if let Some(i) = self.at(x, y)
            && let Some(dst) = self.rgba.get_mut(i..i + 4)
        {
            dst.copy_from_slice(&p);
        }
    }

    /// The image mirrored left to right.
    pub fn flipped(&self) -> RgbaImage {
        let mut out = RgbaImage::new(self.w, self.h);
        for y in 0..self.h {
            for x in 0..self.w {
                if let Some(p) = self.get(x, y) {
                    out.set(self.w - 1 - x, y, p);
                }
            }
        }
        out
    }

    /// The `w` x `h` part of the image whose top-left is (x, y), clipped to the image.
    pub fn crop(&self, x: u32, y: u32, w: u32, h: u32) -> RgbaImage {
        let (w, h) = (
            w.min(self.w.saturating_sub(x)),
            h.min(self.h.saturating_sub(y)),
        );
        let mut out = RgbaImage::new(w, h);
        for oy in 0..h {
            for ox in 0..w {
                if let Some(p) = self.get(x + ox, y + oy) {
                    out.set(ox, oy, p);
                }
            }
        }
        out
    }
}

fn distance([r, g, b]: [u8; 3], c: Rgba) -> u32 {
    let d = |a: u8, b: u8| u32::from(a.abs_diff(b)).pow(2);
    d(r, c.0) + d(g, c.1) + d(b, c.2)
}

/// Of all the blobs of painted pixels (touching sideways or corner to corner), only those at least `share` of
/// the largest one's size are kept: a drawing's stray specks, guide lines and the edges of its neighbours go.
fn keep_blobs(img: &mut RgbaImage, share: f64) {
    let (w, h) = (img.w as usize, img.h as usize);
    let painted = |img: &RgbaImage, x: usize, y: usize| {
        img.get(x as u32, y as u32).is_some_and(|p| p[3] >= OPAQUE)
    };
    let mut blob = vec![usize::MAX; w * h];
    let mut sizes = Vec::new();
    for start in 0..w * h {
        if blob.get(start) != Some(&usize::MAX) || !painted(img, start % w, start / w) {
            continue;
        }
        let id = sizes.len();
        let mut stack = vec![start];
        let mut size = 0;
        while let Some(i) = stack.pop() {
            if blob.get(i) != Some(&usize::MAX) || !painted(img, i % w, i / w) {
                continue;
            }
            if let Some(b) = blob.get_mut(i) {
                *b = id;
            }
            size += 1;
            let (x, y) = (i % w, i / w);
            for (dx, dy) in [
                (-1i64, -1i64),
                (0, -1),
                (1, -1),
                (-1, 0),
                (1, 0),
                (-1, 1),
                (0, 1),
                (1, 1),
            ] {
                let (nx, ny) = (x as i64 + dx, y as i64 + dy);
                if nx >= 0 && ny >= 0 && (nx as usize) < w && (ny as usize) < h {
                    stack.push(ny as usize * w + nx as usize);
                }
            }
        }
        sizes.push(size);
    }
    let largest = sizes.iter().copied().max().unwrap_or(0);
    for (i, &b) in blob.iter().enumerate() {
        let small = sizes
            .get(b)
            .is_some_and(|&n| (n as f64) < share * largest as f64);
        if small {
            img.set((i % w) as u32, (i / w) as u32, [0; 4]);
        }
    }
}

/// `img` cut out of its background: every pixel within `tolerance` (straight-line RGB distance) of its top-left
/// pixel becomes transparent, blobs under a tenth of the largest one's size go too (specks, guide lines, a
/// neighbour's edge), and the rest is cropped to what is left.
pub fn cut(img: &RgbaImage, tolerance: u32) -> RgbaImage {
    let Some([r, g, b, _]) = img.get(0, 0) else {
        return img.clone();
    };
    let back = Rgba(r, g, b, 255);
    let mut out = img.clone();
    for y in 0..img.h {
        for x in 0..img.w {
            let Some([r, g, b, a]) = img.get(x, y) else {
                continue;
            };
            if a < OPAQUE || distance([r, g, b], back) <= tolerance * tolerance {
                out.set(x, y, [0; 4]);
            }
        }
    }
    keep_blobs(&mut out, 0.1);
    let (mut x0, mut y0, mut x1, mut y1) = (u32::MAX, u32::MAX, 0, 0);
    for y in 0..img.h {
        for x in 0..img.w {
            if out.get(x, y).is_some_and(|p| p[3] >= OPAQUE) {
                (x0, y0, x1, y1) = (x0.min(x), y0.min(y), x1.max(x), y1.max(y));
            }
        }
    }
    if x0 > x1 {
        return RgbaImage::new(0, 0);
    }
    out.crop(x0, y0, x1 - x0 + 1, y1 - y0 + 1)
}

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

/// A value map: `w` x `h` cells, row by row, each a palette index or `None` (empty).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Map {
    pub w: u32,
    pub h: u32,
    pub cells: Vec<Option<usize>>,
}

impl Map {
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

/// How far two images are apart: pixels that differ, of all the pixels the larger covers. Two transparent pixels
/// are equal whatever colour they hold; past an image's edge counts as transparent.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Diff {
    pub differing: usize,
    pub total: usize,
}

fn same(a: Option<[u8; 4]>, b: Option<[u8; 4]>) -> bool {
    let a = a.filter(|p| p[3] > 0);
    let b = b.filter(|p| p[3] > 0);
    a == b
}

/// Compare `a` and `b` pixel by pixel; with the comparison, an image the larger one's size showing where they
/// differ (red) over a dimmed copy of `a`.
pub fn diff(a: &RgbaImage, b: &RgbaImage) -> (Diff, RgbaImage) {
    let (w, h) = (a.w.max(b.w), a.h.max(b.h));
    let mut mask = RgbaImage::new(w, h);
    let mut differing = 0;
    for y in 0..h {
        for x in 0..w {
            let (pa, pb) = (a.get(x, y), b.get(x, y));
            if same(pa, pb) {
                if let Some([r, g, b, al]) = pa {
                    mask.set(x, y, [r / 3, g / 3, b / 3, al]);
                }
            } else {
                differing += 1;
                mask.set(x, y, [255, 40, 40, 255]);
            }
        }
    }
    let total = w as usize * h as usize;
    (Diff { differing, total }, mask)
}
