//! A still of an effect beside the character it plays on, for checking that every effect keeps the game's ruler
//! (`pixfx_draw::RULER`): the busiest frame, composited over the backdrop and blown up by a whole number.

use pixfx_core::{Effect, Options, Rendered};
use pixfx_draw::{Frame, RgbaImage, composite};

use crate::gif::BACKDROP;

/// A shared canvas for default effect previews. Aligns their origins and keeps one scale across the gallery.
pub struct PreviewCanvas {
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
}

impl PreviewCanvas {
    pub fn for_effects(effects: &[&dyn Effect]) -> Self {
        let mut bounds = Self {
            left: 0,
            top: 0,
            right: 0,
            bottom: 0,
        };
        let opts = Options::default();
        for effect in effects.iter().filter(|effect| !effect.meta().is_screen()) {
            let canvas = effect.canvas(&opts);
            let (x, y) = canvas.origin;
            bounds.left = bounds.left.max(x);
            bounds.top = bounds.top.max(y);
            bounds.right = bounds.right.max(canvas.size.0 as i32 - x);
            bounds.bottom = bounds.bottom.max(canvas.size.1 as i32 - y);
        }
        bounds
    }

    /// Pad a render at its original pixel resolution. The bounds also include this render's canvas and margin.
    pub fn pad(&self, render: &mut Rendered) {
        let density = render
            .frames
            .first()
            .map_or(pixfx_draw::PixelDensity::One, Frame::density);
        let scale = density.factor() as i32;
        let x = render.origin.0 + render.margin as i32;
        let y = render.origin.1 + render.margin as i32;
        let left = (self.left * scale).max(x);
        let top = (self.top * scale).max(y);
        let w = (left + (self.right * scale).max(render.w as i32 - x)) as u32;
        let h = (top + (self.bottom * scale).max(render.h as i32 - y)) as u32;
        let offset = (top - y) as usize * w as usize + (left - x) as usize;
        let row_width = render.w as usize;
        for frame in &mut render.frames {
            let mut padded =
                Frame::with_density(w / density.factor(), h / density.factor(), 0, 0, density);
            for (row, (color, light)) in frame
                .color
                .chunks_exact(row_width)
                .zip(frame.light.chunks_exact(row_width))
                .enumerate()
            {
                let start = offset + row * w as usize;
                let end = start + row_width;
                padded.color[start..end].copy_from_slice(color);
                padded.light[start..end].copy_from_slice(light);
            }
            *frame = padded;
        }
        render.w = w;
        render.h = h;
        render.origin = (left, top);
        render.margin = 0;
    }
}

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
