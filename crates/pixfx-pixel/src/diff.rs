//! How far two pictures are apart.

use crate::RgbaImage;

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

/// Where a figure stands: the middle of its painted columns (halves down) and the row just under its lowest
/// painted pixel; `None` when nothing is painted.
fn feet_of(img: &RgbaImage) -> Option<(u32, u32)> {
    let mut span: Option<(u32, u32, u32)> = None;
    for y in 0..img.h {
        for x in 0..img.w {
            if img.get(x, y).is_some_and(|p| p[3] > 0) {
                let (x0, x1, _) = span.unwrap_or((x, x, y));
                span = Some((x0.min(x), x1.max(x), y + 1));
            }
        }
    }
    span.map(|(x0, x1, bottom)| ((x0 + x1) / 2, bottom))
}

/// `a` and `b` placed on one canvas so they stand on the same spot: their bottom painted rows and the middles of
/// their painted columns line up. A picture with nothing painted stands at its bottom centre. Compare the two with
/// [`diff`] to see a drawing against a sprite whatever margin each was cut with.
pub fn feet(a: &RgbaImage, b: &RgbaImage) -> (RgbaImage, RgbaImage) {
    let (ax, ay) = feet_of(a).unwrap_or((a.w / 2, a.h));
    let (bx, by) = feet_of(b).unwrap_or((b.w / 2, b.h));
    let (left, top) = (ax.max(bx), ay.max(by));
    let w = left + (a.w - ax).max(b.w - bx);
    let h = top + (a.h - ay).max(b.h - by);
    let place = |img: &RgbaImage, fx: u32, fy: u32| {
        let mut out = RgbaImage::new(w, h);
        let (ox, oy) = (left - fx, top - fy);
        for y in 0..img.h {
            for x in 0..img.w {
                if let Some(p) = img.get(x, y) {
                    out.set(ox + x, oy + y, p);
                }
            }
        }
        out
    };
    (place(a, ax, ay), place(b, bx, by))
}
