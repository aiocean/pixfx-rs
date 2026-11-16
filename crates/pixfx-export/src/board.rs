//! A still of an effect beside the character it plays on, for checking that every effect keeps the game's ruler
//! (`pixfx_draw::RULER`): the busiest frame, composited over the backdrop and blown up by a whole number.

use pixfx_core::Rendered;
use pixfx_draw::{RgbaImage, composite};

use crate::gif::BACKDROP;

/// The frame of `r` with the most pixels painted (colour or light); `None` for a render with no frames.
pub fn busiest(r: &Rendered) -> Option<usize> {
    r.frames
        .iter()
        .enumerate()
        .map(|(k, f)| {
            let n = f
                .color
                .iter()
                .zip(&f.light)
                .filter(|&(&c, &l)| c >> 24 != 0 || l & 0x00ff_ffff != 0)
                .count();
            (k, n)
        })
        .fold(None, |best: Option<(usize, usize)>, (k, n)| match best {
            Some((_, m)) if m >= n => best,
            _ => Some((k, n)),
        })
        .map(|(k, _)| k)
}

/// Frame `k` of `r` over the backdrop, each pixel `scale` x `scale` (at least 1); `None` past the last frame.
pub fn still(r: &Rendered, k: usize, scale: u32) -> Option<RgbaImage> {
    let f = r.frames.get(k)?;
    let s = scale.max(1);
    let n = r.w as usize * r.h as usize;
    let ground = BACKDROP.repeat(n);
    let mut rgba = vec![0; n * 4];
    composite(f, &mut rgba, Some(&ground));
    let mut out = RgbaImage::new(r.w * s, r.h * s);
    for y in 0..out.h {
        for x in 0..out.w {
            let i = ((y / s) * r.w + x / s) as usize * 4;
            let p = rgba.get(i..i + 4).and_then(|p| <[u8; 4]>::try_from(p).ok());
            out.set(x, y, p.unwrap_or(BACKDROP));
        }
    }
    Some(out)
}
