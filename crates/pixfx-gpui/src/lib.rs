//! pixfx in gpui: live effect instances painted into a gpui window, colour with normal blending and light
//! with additive blending. Images are uploaded at the effect's own size and enlarged by the GPU with
//! nearest-neighbour sampling, so the pixel art stays crisp.
//!
//! Each paint of a changed effect uploads fresh images to gpui's sprite atlas (a [`RenderImage`] is
//! immutable and cached by id) and then drops the images it replaces, so the atlas holds one generation
//! per effect and layer.
#![forbid(unsafe_code)]

use std::sync::Arc;

use anyhow::Result;
use gpui::{
    Bounds, Corners, Pixels, Point, RenderImage, SpriteBlend, SpriteFilter, Window, point, px, size,
};
use image::{Frame as ImageFrame, RgbaImage};
use pixfx_core::{Effect, Frame, Instance, Layer, Options};

/// A rectangle of source pixels.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PixelRect {
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
}

/// The smallest rectangle of a `w`-wide frame's `words` holding a word with any bit of `mask` set.
pub fn occupied(words: &[u32], w: u32, mask: u32) -> Option<PixelRect> {
    let w = w as usize;
    if w == 0 {
        return None;
    }
    let (mut x0, mut y0, mut x1, mut y1) = (usize::MAX, usize::MAX, 0, 0);
    for (y, row) in words.chunks(w).enumerate() {
        let lit = |v: &u32| v & mask != 0;
        let Some(first) = row.iter().position(lit) else {
            continue;
        };
        let last = row.iter().rposition(lit).unwrap_or(first);
        x0 = x0.min(first);
        x1 = x1.max(last);
        y0 = y0.min(y);
        y1 = y;
    }
    (x0 != usize::MAX).then(|| PixelRect {
        x: x0 as u32,
        y: y0 as u32,
        w: (x1 - x0 + 1) as u32,
        h: (y1 - y0 + 1) as u32,
    })
}

/// BGRA bytes of `rect` in `words` (rows of `stride` words), each word turned into a pixel by `bgra`.
pub fn rect_bgra(
    words: &[u32],
    stride: u32,
    rect: PixelRect,
    bgra: impl Fn(u32) -> [u8; 4],
) -> Vec<u8> {
    let stride = stride as usize;
    let (x0, w) = (rect.x as usize, rect.w as usize);
    let row_bytes = w * 4;
    let mut out = Vec::with_capacity(row_bytes * rect.h as usize);
    for y in rect.y as usize..(rect.y + rect.h) as usize {
        let start = out.len();
        let src = words
            .get(y * stride + x0..y * stride + x0 + w)
            .unwrap_or(&[]);
        for &word in src {
            out.extend_from_slice(&bgra(word));
        }
        // short source rows pad with transparency so every row has the same length
        out.resize(start + row_bytes, 0);
    }
    out
}

/// A colour word (ABGR, see [`pixfx_core::Rgba::pack`]) as straight-alpha BGRA, as gpui's normal sprites
/// expect.
fn color_bgra(word: u32) -> [u8; 4] {
    let [r, g, b, a] = word.to_le_bytes();
    [b, g, r, a]
}

/// A light word (xBGR) as BGRA: additive sprites add rgb whatever the alpha.
fn light_bgra(word: u32) -> [u8; 4] {
    let [r, g, b, _] = word.to_le_bytes();
    [b, g, r, 255]
}

/// One uploaded layer: the image and where its top-left sits, in source pixels from the frame's top-left.
struct Sprite {
    image: Arc<RenderImage>,
    rect: PixelRect,
}

/// The images painted for one [`Layer`], and what they were rasterised from.
struct Painted {
    generation: u64,
    color: Option<Sprite>,
    light: Option<Sprite>,
}

/// A live effect: an [`Instance`] advanced in real time, rasterised into one reused frame when painted.
pub struct FxView {
    instance: Instance,
    frame: Frame,
    /// Source pixels from the frame's top-left to the effect's origin.
    pivot: (i32, i32),
    /// Bumped by every [`FxView::tick`]; a layer painted at an older generation is rasterised again.
    generation: u64,
    painted: [Option<Painted>; 3],
}

impl FxView {
    /// The effect at its origin, on the canvas `opts` give it ([`Effect::canvas`]), in a frame with room round
    /// that canvas for glow that spills past it ([`pixfx_core::Meta::glow_margin`] on every side).
    pub fn new(effect: &'static dyn Effect, opts: Options) -> Self {
        let margin = effect.meta().glow_margin();
        let canvas = effect.canvas(&opts);
        let (w, h) = (canvas.size.0 + 2 * margin, canvas.size.1 + 2 * margin);
        let pivot = (canvas.origin.0 + margin as i32, canvas.origin.1 + margin as i32);
        let at_pivot = Options {
            x: f64::from(pivot.0),
            y: f64::from(pivot.1),
            ..opts
        };
        Self {
            instance: Instance::new(effect, at_pivot),
            frame: Frame::new(w, h, 0, 0),
            pivot,
            generation: 0,
            painted: [None, None, None],
        }
    }

    /// Advance by `dt` seconds of real time ([`Instance::update`]: a NaN, infinite or negative `dt` is ignored).
    pub fn tick(&mut self, dt: f64) {
        self.instance.update(dt);
        self.generation += 1;
    }

    /// Rasterise `layer` into fresh images, one pixel per effect pixel.
    fn rasterise(&mut self, layer: Layer) -> Painted {
        self.frame.clear();
        self.instance.draw(&mut self.frame, layer);
        let (w, frame) = (self.frame.w, &self.frame);
        let sprite = |words: &[u32], mask: u32, bgra: fn(u32) -> [u8; 4]| {
            let rect = occupied(words, w, mask)?;
            let bytes = rect_bgra(words, w, rect, bgra);
            // `image::Frame` holds an `RgbaImage`; gpui reads its bytes as BGRA
            let buffer = RgbaImage::from_raw(rect.w, rect.h, bytes)?;
            Some(Sprite {
                image: Arc::new(RenderImage::new([ImageFrame::new(buffer)])),
                rect,
            })
        };
        Painted {
            generation: self.generation,
            color: sprite(&frame.color, 0xff00_0000, color_bgra),
            light: sprite(&frame.light, 0x00ff_ffff, light_bgra),
        }
    }
}

fn slot(layer: Layer) -> usize {
    match layer {
        Layer::Back => 0,
        Layer::Front => 1,
        Layer::Both => 2,
    }
}

/// Paint `layer` of `view` with the effect's origin at `origin`, each effect pixel `scale` x `scale` logical
/// pixels (a `scale` of 0 paints as 1): colour with [`SpriteBlend::Normal`], then light with
/// [`SpriteBlend::Additive`], both enlarged by the GPU with [`SpriteFilter::Nearest`]. Images are rebuilt
/// only when the view ticked; the images they replace are dropped from `window`'s atlas after the new ones
/// are painted, so paint a view into one window only. Call during gpui's paint phase.
pub fn paint_fx(
    view: &mut FxView,
    layer: Layer,
    origin: Point<Pixels>,
    scale: u32,
    window: &mut Window,
) -> Result<()> {
    let scale = scale.max(1);
    let fresh = view.painted[slot(layer)]
        .as_ref()
        .is_some_and(|p| p.generation == view.generation);
    let replaced = if fresh {
        None
    } else {
        let painted = view.rasterise(layer);
        view.painted[slot(layer)].replace(painted)
    };
    let mut painting = Ok(());
    if let Some(painted) = &view.painted[slot(layer)] {
        let corner = |rect: PixelRect| {
            let s = scale as f32;
            point(
                origin.x + px((rect.x as i32 - view.pivot.0) as f32 * s),
                origin.y + px((rect.y as i32 - view.pivot.1) as f32 * s),
            )
        };
        for (sprite, blend) in [
            (&painted.color, SpriteBlend::Normal),
            (&painted.light, SpriteBlend::Additive),
        ] {
            let (Some(sprite), Ok(())) = (sprite, &painting) else {
                continue;
            };
            let s = scale as f32;
            let bounds = Bounds::new(
                corner(sprite.rect),
                size(px(sprite.rect.w as f32 * s), px(sprite.rect.h as f32 * s)),
            );
            painting = window.paint_image_blended(
                bounds,
                bounds,
                Corners::default(),
                sprite.image.clone(),
                0,
                false,
                blend,
                SpriteFilter::Nearest,
            );
        }
    }
    // dropped whether or not the paint failed, so a failing frame cannot leak atlas space
    if let Some(old) = replaced {
        for sprite in [old.color, old.light].into_iter().flatten() {
            window.drop_image(sprite.image)?;
        }
    }
    painting
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rect_bgra_reads_only_the_rect() {
        // 3x2 frame, rect is the middle column
        let words = [1, 2, 3, 4, 5, 6];
        let rect = PixelRect {
            x: 1,
            y: 0,
            w: 1,
            h: 2,
        };
        let out = rect_bgra(&words, 3, rect, |w| [w as u8, 0, 0, 0]);
        let firsts: Vec<u8> = out.chunks(4).map(|p| p[0]).collect();
        assert_eq!(firsts, [2, 5]);
    }

    #[test]
    fn rect_bgra_pads_short_rows_with_transparency() {
        // the rect runs one row past the 2x1 frame
        let words = [0xff00_00ff_u32, 0x8000_ff00];
        let rect = PixelRect {
            x: 0,
            y: 0,
            w: 2,
            h: 2,
        };
        let out = rect_bgra(&words, 2, rect, |w| w.to_le_bytes());
        let mut want = Vec::new();
        want.extend_from_slice(&words[0].to_le_bytes());
        want.extend_from_slice(&words[1].to_le_bytes());
        want.extend_from_slice(&[0; 8]);
        assert_eq!(out, want);
    }

    #[test]
    fn occupied_bounds_the_set_words() {
        let mut words = vec![0_u32; 5 * 4];
        words[5 + 1] = 0xff00_0000;
        words[2 * 5 + 3] = 0x0100_0000;
        words[3 * 5 + 4] = 0x00ff_ffff; // light only: outside the colour mask
        let rect = occupied(&words, 5, 0xff00_0000);
        assert_eq!(
            rect,
            Some(PixelRect {
                x: 1,
                y: 1,
                w: 3,
                h: 2
            })
        );
        assert_eq!(occupied(&[0; 6], 3, 0xff00_0000), None);
    }

    #[test]
    fn colour_and_light_become_bgra() {
        // ABGR 0x80_30_20_10 is r 0x10, g 0x20, b 0x30, a 0x80
        assert_eq!(color_bgra(0x8030_2010), [0x30, 0x20, 0x10, 0x80]);
        assert_eq!(light_bgra(0x0030_2010), [0x30, 0x20, 0x10, 0xff]);
    }
}
