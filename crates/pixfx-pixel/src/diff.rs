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
