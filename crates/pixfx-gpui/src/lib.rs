//! pixfx in gpui: live effect instances painted into a gpui window, colour with normal blending and light
//! with additive blending. Images are uploaded at the effect's own size and enlarged by the GPU with
//! nearest-neighbour sampling, so the pixel art stays crisp.
//!
//! A [`Scene`] decides everything: an effect's frame is rasterised and uploaded to gpui's sprite atlas (a
//! [`RenderImage`] is immutable and cached by id) only when the effect reaches a frame not uploaded yet, effects
//! showing the same frame paint the same images, the least recently painted frames leave the atlas past its byte
//! budget, and an effect fading out after [`FxScene::stop`] paints its own images, redrawn once per frame of its
//! clock. This crate is the gpui shell: it uploads, paints and drops images ([`FxScene`]).
#![forbid(unsafe_code)]

use std::num::NonZeroU32;
use std::sync::Arc;

use anyhow::{Result, anyhow};
use gpui::{
    Bounds, Corners, Pixels, Point, RenderImage, SpriteBlend, SpriteFilter, Window, point, px, size,
};
use image::{Frame as ImageFrame, RgbaImage};
use pixfx_core::{Effect, Layer, Options, Player};
use pixfx_draw::Frame;
use pixfx_scene::{Pass, Scene, Shell};

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

/// A colour word (ABGR, see [`pixfx_draw::Rgba::pack`]) as straight-alpha BGRA, as gpui's normal sprites
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

/// The images painted for one [`Layer`] of one frame.
struct Painted {
    color: Option<Sprite>,
    light: Option<Sprite>,
}

impl Painted {
    /// Upload `frame`'s layers as images cropped to their drawn pixels.
    fn of(frame: &Frame) -> Self {
        let w = frame.w;
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
        Self {
            color: sprite(&frame.color, 0xff00_0000, color_bgra),
            light: sprite(&frame.light, 0x00ff_ffff, light_bgra),
        }
    }

    fn images(self) -> impl Iterator<Item = Arc<RenderImage>> {
        [self.color, self.light]
            .into_iter()
            .flatten()
            .map(|s| s.image)
    }
}

/// The effects painted into one window: a [`Scene`] decides what each effect shows, which frames are shared and
/// when one leaves the atlas; this crate only uploads, paints and drops images. Paint each effect with
/// [`FxScene::paint`], then call [`FxScene::end_frame`] once per window frame.
pub struct FxScene {
    scene: Scene<Painted>,
    /// Images no effect paints any more, dropped from the atlas at the end of the frame.
    retired: Vec<Arc<RenderImage>>,
}

impl Default for FxScene {
    fn default() -> Self {
        // effects sit at their view's origin in effect pixels; the view scales them as it paints
        let mut scene = Scene::new(NonZeroU32::MIN);
        // a gpui app answers cues with its own feel, if any
        scene.set_feel(None);
        Self {
            scene,
            retired: Vec::new(),
        }
    }
}

/// One effect playing in an [`FxScene`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct FxHandle(u32);

impl FxScene {
    /// Play `effect` on the canvas `opts` give it ([`Effect::canvas`]), with room round that canvas for glow that
    /// spills past it ([`pixfx_core::Meta::glow_margin`] on every side).
    pub fn play(&mut self, effect: &'static dyn Effect, opts: Options) -> Result<FxHandle> {
        let handle = self
            .scene
            .play(Player::live(effect, opts), 0.0, 0.0)
            .map_err(|e| anyhow!(e))?;
        Ok(FxHandle(handle))
    }

    /// Advance every effect by `dt` seconds of real time; finished effects leave, their images dropped at the end
    /// of the next frame. A NaN, infinite or negative `dt` is an error and changes nothing.
    pub fn tick(&mut self, dt: f64) -> Result<()> {
        self.scene.update(dt).map_err(|e| anyhow!(e))?;
        Ok(())
    }

    /// Let it end: loops fade out, one-shots are cut short. False if it is gone.
    pub fn stop(&mut self, fx: FxHandle) -> bool {
        self.scene.stop(fx.0)
    }

    /// Drop it now; its images leave the atlas at the end of the next frame. False if it is gone.
    pub fn remove(&mut self, fx: FxHandle) -> bool {
        self.scene.remove(fx.0)
    }

    /// Whether it finished (or was removed).
    pub fn done(&self, fx: FxHandle) -> bool {
        !self.scene.handles().any(|h| h == fx.0)
    }

    /// Bytes the shared frames may hold ([`pixfx_core::FRAME_BUDGET`] unless set).
    pub fn set_budget(&mut self, bytes: u64) {
        self.scene.set_frame_budget(bytes);
    }

    /// Draw at most `layers` layers per window frame, or every changed one with `None` (the default); effects
    /// left over show their last frame and wait ([`Scene::set_draw_budget`]). Views take turns in the order they
    /// paint.
    pub fn set_draw_budget(&mut self, layers: Option<NonZeroU32>) {
        self.scene.set_draw_budget(layers);
    }

    /// Bytes the shared frames hold.
    pub fn bytes(&self) -> u64 {
        self.scene.frame_bytes()
    }

    /// Paint `layer` of `fx` with the effect's origin at `origin`, each effect pixel `scale` x `scale` logical
    /// pixels (a `scale` of 0 paints as 1): colour with [`SpriteBlend::Normal`], then light with
    /// [`SpriteBlend::Additive`], both enlarged by the GPU with [`SpriteFilter::Nearest`]. Call during gpui's paint
    /// phase. False if `fx` is gone.
    pub fn paint(
        &mut self,
        fx: FxHandle,
        layer: Layer,
        origin: Point<Pixels>,
        scale: u32,
        window: &mut Window,
    ) -> Result<bool> {
        let mut shell = GpuiShell {
            window,
            origin,
            scale: scale.max(1) as f32,
            retired: &mut self.retired,
        };
        self.scene.draw_handle(fx.0, layer, &mut shell)
    }

    /// Call once after every effect is painted: drops from `window`'s atlas the images replaced or released this
    /// frame and, past the budget, the least recently painted frames (never one painted this frame).
    pub fn end_frame(&mut self, window: &mut Window) -> Result<()> {
        let mut shell = GpuiShell {
            window,
            origin: point(px(0.), px(0.)),
            scale: 1.0,
            retired: &mut self.retired,
        };
        self.scene.end_frame(&mut shell)?;
        let mut dropping = Ok(());
        for image in self.retired.drain(..) {
            // every image goes even if one fails, so a failing drop cannot leak the rest
            let dropped = window.drop_image(image);
            if dropping.is_ok() {
                dropping = dropped;
            }
        }
        dropping
    }
}

/// The window's side of [`Shell`]: frames uploaded as images, images painted where a [`Pass`] says (in effect
/// pixels from the view's origin), images let go of queued until the frame ends, never dropped mid-frame.
struct GpuiShell<'a> {
    window: &'a mut Window,
    origin: Point<Pixels>,
    scale: f32,
    retired: &'a mut Vec<Arc<RenderImage>>,
}

impl Shell for GpuiShell<'_> {
    type Surface = Painted;
    type Error = anyhow::Error;

    fn make(&mut self, frame: &Frame) -> Result<Painted> {
        Ok(Painted::of(frame))
    }

    fn refill(&mut self, surface: &mut Painted, frame: &Frame) -> Result<()> {
        let old = std::mem::replace(surface, Painted::of(frame));
        self.retired.extend(old.images());
        Ok(())
    }

    fn show(&mut self, pass: Pass, surface: &Painted) -> Result<()> {
        let s = self.scale;
        for (sprite, blend) in [
            (&surface.color, SpriteBlend::Normal),
            (&surface.light, SpriteBlend::Additive),
        ] {
            let Some(sprite) = sprite else {
                continue;
            };
            // `pass.rect` is the frame's corner in effect pixels from the origin (the scene's scale is 1)
            let corner = point(
                self.origin.x + px((pass.rect.x as f32 + sprite.rect.x as f32) * s),
                self.origin.y + px((pass.rect.y as f32 + sprite.rect.y as f32) * s),
            );
            let bounds = Bounds::new(
                corner,
                size(px(sprite.rect.w as f32 * s), px(sprite.rect.h as f32 * s)),
            );
            self.window.paint_image_blended(
                bounds,
                bounds,
                Corners::default(),
                sprite.image.clone(),
                0,
                false,
                blend,
                SpriteFilter::Nearest,
            )?;
        }
        Ok(())
    }

    fn free(&mut self, surface: Painted) -> Result<()> {
        self.retired.extend(surface.images());
        Ok(())
    }
}
