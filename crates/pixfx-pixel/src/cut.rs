//! Cutting a picture out of its background.

use crate::{OPAQUE, Rgba, RgbaImage, distance};

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
