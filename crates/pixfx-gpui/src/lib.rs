//! pixfx in gpui: live effect instances painted into a gpui window, colour with normal blending and light
//! with additive blending. Images are uploaded at the effect's own size and enlarged by the GPU with
//! nearest-neighbour sampling, so the pixel art stays crisp.
//!
//! An effect's frame is rasterised and uploaded to gpui's sprite atlas (a [`RenderImage`] is immutable and
//! cached by id) only when the effect reaches a frame not uploaded yet: views showing the same frame
//! ([`pixfx_core::Player::shown`]) paint the same images from one [`FxFrames`] per window, which drops the
//! least recently painted frames from the atlas past its byte budget. An effect fading out after
//! [`FxView::stop`] paints its own images, redrawn once per frame of its clock.
#![forbid(unsafe_code)]

use std::sync::Arc;

use anyhow::Result;
use gpui::{
    Bounds, Corners, Pixels, Point, RenderImage, SpriteBlend, SpriteFilter, Window, point, px, size,
};
use image::{Frame as ImageFrame, RgbaImage};
use pixfx_core::{Effect, Frame, FrameStore, Layer, Options, Player};

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

/// The images painted for one [`Layer`] of one frame.
struct Painted {
    color: Option<Sprite>,
    light: Option<Sprite>,
}

impl Painted {
    /// Upload `frame`'s layers as images cropped to their drawn pixels; also returns the bytes they hold.
    fn of(frame: &Frame) -> (Self, u64) {
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
        let painted = Self {
            color: sprite(&frame.color, 0xff00_0000, color_bgra),
            light: sprite(&frame.light, 0x00ff_ffff, light_bgra),
        };
        let bytes = [&painted.color, &painted.light]
            .into_iter()
            .flatten()
            .map(|s| u64::from(s.rect.w) * u64::from(s.rect.h) * 4)
            .sum();
        (painted, bytes)
    }

    fn images(self) -> impl Iterator<Item = Arc<RenderImage>> {
        [self.color, self.light]
            .into_iter()
            .flatten()
            .map(|s| s.image)
    }
}

/// The frames uploaded to one window's atlas, shared by every [`FxView`] painted into it. Paint the views,
/// then call [`FxFrames::end_frame`] once per window frame.
#[derive(Default)]
pub struct FxFrames {
    store: FrameStore<Painted>,
    /// Window frames ended so far: when each held frame was last painted.
    now: u64,
    /// Images no view paints any more, dropped from the atlas at the end of the frame.
    retired: Vec<Arc<RenderImage>>,
}

impl FxFrames {
    /// Bytes the uploaded frames may hold ([`pixfx_core::FRAME_BUDGET`] unless set).
    pub fn set_budget(&mut self, bytes: u64) {
        self.store.set_budget(bytes);
    }

    /// Bytes the uploaded frames hold.
    pub fn bytes(&self) -> u64 {
        self.store.bytes()
    }

    /// Stop holding `view`'s own images (an effect removed while fading out); they are dropped from the atlas
    /// at the end of the frame.
    pub fn release(&mut self, view: FxView) {
        self.retired.extend(
            view.own
                .into_iter()
                .flatten()
                .flat_map(|o| o.painted.images()),
        );
    }

    /// Call once after every view is painted: drops from `window`'s atlas the images replaced this frame and,
    /// past the budget, the least recently painted frames (never one painted this frame).
    pub fn end_frame(&mut self, window: &mut Window) -> Result<()> {
        let trimmed = self.store.trim(self.now);
        self.now += 1;
        let mut dropping = Ok(());
        for image in self
            .retired
            .drain(..)
            .chain(trimmed.into_iter().flat_map(Painted::images))
        {
            // every image goes even if one fails, so a failing drop cannot leak the rest
            let dropped = window.drop_image(image);
            if dropping.is_ok() {
                dropping = dropped;
            }
        }
        dropping
    }
}

/// A fading effect's own images and the clock frame they were drawn on.
struct Own {
    painted: Painted,
    drawn: u64,
}

/// A live effect: a [`Player`] advanced in real time, its frames painted from an [`FxFrames`].
pub struct FxView {
    player: Player,
    /// Source pixels from the frame's top-left to the effect's origin.
    pivot: (i32, i32),
    /// Per layer, the images of the fade frame last painted.
    own: [Option<Own>; 3],
}

impl FxView {
    /// The effect at its origin, on the canvas `opts` give it ([`Effect::canvas`]), in a frame with room round
    /// that canvas for glow that spills past it ([`pixfx_core::Meta::glow_margin`] on every side).
    pub fn new(effect: &'static dyn Effect, opts: Options) -> Self {
        let player = Player::live(effect, opts);
        let (origin, margin) = (player.canvas().origin, player.margin() as i32);
        Self {
            pivot: (origin.0 + margin, origin.1 + margin),
            player,
            own: [None, None, None],
        }
    }

    /// Advance by `dt` seconds of real time ([`Player::update`]: a NaN, infinite or negative `dt` is ignored).
    pub fn tick(&mut self, dt: f64) {
        self.player.update(dt);
    }

    /// Let it end: loops fade out, one-shots are cut short.
    pub fn stop(&mut self) {
        self.player.stop();
    }

    pub fn done(&self) -> bool {
        self.player.done()
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
/// [`SpriteBlend::Additive`], both enlarged by the GPU with [`SpriteFilter::Nearest`]. The frame is taken from
/// `frames` and rasterised only when no view painted it yet. Call during gpui's paint phase, with the
/// window's own `frames`.
pub fn paint_fx(
    view: &mut FxView,
    frames: &mut FxFrames,
    layer: Layer,
    origin: Point<Pixels>,
    scale: u32,
    window: &mut Window,
) -> Result<()> {
    let place = view.player.placement();
    let player = &mut view.player;
    let painted = match player.shown(place) {
        // a running effect never has fade frames of its own: a stopped one stays stopped
        Some(id) => frames.store.show(&id, layer, frames.now, || {
            player.draw(layer, place);
            Ok::<_, anyhow::Error>(Painted::of(player.frame()))
        })?,
        None => {
            let clock = player.clock_frame();
            let own = &mut view.own[slot(layer)];
            if own.as_ref().is_none_or(|o| o.drawn != clock) {
                player.draw(layer, place);
                let (painted, _) = Painted::of(player.frame());
                let replaced = own.replace(Own {
                    painted,
                    drawn: clock,
                });
                frames
                    .retired
                    .extend(replaced.into_iter().flat_map(|o| o.painted.images()));
            }
            match own {
                Some(o) => &o.painted,
                None => return Ok(()),
            }
        }
    };
    let s = scale.max(1) as f32;
    for (sprite, blend) in [
        (&painted.color, SpriteBlend::Normal),
        (&painted.light, SpriteBlend::Additive),
    ] {
        let Some(sprite) = sprite else {
            continue;
        };
        let corner = point(
            origin.x + px((sprite.rect.x as i32 - view.pivot.0) as f32 * s),
            origin.y + px((sprite.rect.y as i32 - view.pivot.1) as f32 * s),
        );
        let bounds = Bounds::new(
            corner,
            size(px(sprite.rect.w as f32 * s), px(sprite.rect.h as f32 * s)),
        );
        window.paint_image_blended(
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
