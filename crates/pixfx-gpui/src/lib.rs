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
use pixfx_core::volley::{Aim, Order};
use pixfx_core::{Actor, Clip, Effect, Layer, Options};
use pixfx_draw::Frame;
use pixfx_scene::{
    Body, Frames, InlinePainter, Job, Painter, Pass, Receive, Scene, Shell, StageEvent, Target,
};

#[cfg(not(target_arch = "wasm32"))]
mod threads;
#[cfg(not(target_arch = "wasm32"))]
pub use threads::ThreadPainter;

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
#[derive(Clone)]
struct Sprite {
    image: Arc<RenderImage>,
    rect: PixelRect,
}

/// The images painted for one [`Layer`] of one frame.
#[derive(Clone)]
pub(crate) struct Painted {
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

    /// Bytes the images take, counted as a frame's lit box is ([`pixfx_core::PixelBox::lit`]: the box round its
    /// colour and light, 4 bytes a pixel each).
    fn bytes(&self) -> u64 {
        let rects = [&self.color, &self.light]
            .into_iter()
            .flatten()
            .map(|s| s.rect);
        let lit = rects.reduce(|a, b| {
            let (x, y) = (a.x.min(b.x), a.y.min(b.y));
            let (r, d) = ((a.x + a.w).max(b.x + b.w), (a.y + a.h).max(b.y + b.h));
            PixelRect {
                x,
                y,
                w: r - x,
                h: d - y,
            }
        });
        lit.map_or(0, |r| u64::from(r.w) * u64::from(r.h) * 8)
    }

    fn images(self) -> impl Iterator<Item = Arc<RenderImage>> {
        [self.color, self.light]
            .into_iter()
            .flatten()
            .map(|s| s.image)
    }
}

/// Who paints a window's frames: on the spot, or on native threads.
enum Paint {
    Inline(InlinePainter),
    #[cfg(not(target_arch = "wasm32"))]
    Threads(ThreadPainter),
}

impl Painter for Paint {
    fn paint(&mut self, job: &Job<'_>) -> Option<&Frame> {
        match self {
            Self::Inline(p) => p.paint(job),
            #[cfg(not(target_arch = "wasm32"))]
            Self::Threads(p) => p.paint(job),
        }
    }

    fn end(&mut self, effect: pixfx_scene::EffectKey) {
        match self {
            Self::Inline(p) => p.end(effect),
            #[cfg(not(target_arch = "wasm32"))]
            Self::Threads(p) => p.end(effect),
        }
    }
}

/// The effects painted into one window: a [`Scene`] decides what each effect shows, which frames are shared and
/// when one leaves the atlas; this crate only uploads, paints and drops images. Paint each effect with
/// [`FxScene::paint`], then call [`FxScene::end_frame`] once per window frame.
pub struct FxScene {
    /// The frames the scene shares, and who paints them.
    frames: Frames<Painted, Paint>,
    scene: Scene<Painted>,
    /// Images no effect paints any more, dropped from the atlas at the end of the frame.
    retired: Vec<Arc<RenderImage>>,
    /// Whether this window frame took back what the threads painted.
    received: bool,
}

impl Default for FxScene {
    /// Frames painted on the spot, on the window's thread.
    fn default() -> Self {
        Self::with(Paint::Inline(InlinePainter::default()))
    }
}

/// One effect playing in an [`FxScene`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct FxHandle(u32);

impl FxScene {
    fn with(paint: Paint) -> Self {
        // effects sit at their view's origin in effect pixels; the view scales them as it paints
        let mut frames = Frames::new(paint);
        let mut scene = frames.scene(NonZeroU32::MIN);
        // a gpui app answers cues with its own feel, if any
        scene.set_feel(None);
        Self {
            frames,
            scene,
            retired: Vec::new(),
            received: false,
        }
    }

    /// Frames painted on `n` native threads (at least one): each effect is stepped, drawn and turned into images
    /// on one of them, so the window's thread only paints images. A frame asked for shows from the window frame
    /// after it comes back; until then the effect shows the one it had. For windows with many effects at once.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn with_threads(n: usize) -> Result<Self> {
        Ok(Self::with(Paint::Threads(ThreadPainter::new(n)?)))
    }

    /// Take back what the threads painted into `frames` and `scene`, before the first effect of the window frame
    /// paints.
    fn receive(
        frames: &mut Frames<Painted, Paint>,
        scene: &mut Scene<Painted>,
        shell: &mut GpuiShell<'_>,
    ) -> Result<()> {
        #[cfg(not(target_arch = "wasm32"))]
        if let Paint::Threads(p) = frames.painter_mut() {
            for done in p.take() {
                match done.target {
                    Target::Shared(id) => {
                        frames.deliver(&id, done.layer, &done.painted, shell)?;
                    }
                    Target::Own(at) => {
                        scene.deliver_own(
                            done.effect.handle,
                            done.layer,
                            at,
                            &done.painted,
                            shell,
                        )?;
                    }
                }
            }
        }
        #[cfg(target_arch = "wasm32")]
        let _ = (frames, scene, shell);
        Ok(())
    }

    /// Play `effect` on the canvas `opts` give it ([`Effect::canvas`]), with room round that canvas for glow that
    /// spills past it ([`pixfx_core::Meta::glow_margin`] on every side).
    pub fn play(&mut self, effect: &'static dyn Effect, opts: Options) -> Result<FxHandle> {
        let handle = self
            .scene
            .play(Clip::live(effect, opts), 0.0, 0.0)
            .map_err(|e| anyhow!(e))?;
        Ok(FxHandle(handle))
    }

    /// Advance every effect by `dt` seconds of real time; finished effects leave, their images dropped at the end
    /// of the next frame. A NaN, infinite or negative `dt` is an error and changes nothing.
    pub fn tick(&mut self, dt: f64) -> Result<()> {
        self.scene.update(dt).map_err(|e| anyhow!(e))?;
        Ok(())
    }

    /// Play it on a character: `actor` is its sprite's frame as the view shows it now, placed in effect pixels with
    /// the effect's origin at (0, 0), or `None` for none. An effect that wraps its actor (as `feedback/power-up`
    /// does) grips that outline from its next step; others ignore it. Hand it every frame: the same sprite at the
    /// same place changes nothing. False if it is gone.
    pub fn set_actor(&mut self, fx: FxHandle, actor: Option<Actor>) -> Result<bool> {
        self.scene.set_actor(fx.0, actor).map_err(|e| anyhow!(e))
    }

    /// Let it end: loops fade out, one-shots are cut short. False if it is gone.
    pub fn stop(&mut self, fx: FxHandle) -> bool {
        self.scene.stop(fx.0)
    }

    /// Order `fx` to attack toward the point (`x`, `y`) in effect pixels from its own origin, at any distance: an
    /// effect that takes orders ([`pixfx_core::Effect::order`]) launches its volley ([`FxScene::launched`]), whose
    /// pieces strike the bodies they meet ([`FxScene::set_body`]) but `caster`, and ends once the last piece is
    /// done. False if it takes no orders, was ordered already, or is gone.
    pub fn order(
        &mut self,
        fx: FxHandle,
        x: f64,
        y: f64,
        aim: Aim,
        caster: Option<u32>,
    ) -> Result<bool> {
        self.scene
            .order(fx.0, Order { x, y, aim, caster })
            .map_err(|e| anyhow!(e))
    }

    /// Body `id` (a fighter, a wall) is `body`, in the same pixels as [`FxScene::order`]'s point, or none to drop
    /// it: volley pieces that meet it strike it. Set it as it moves.
    pub fn set_body(&mut self, id: u32, body: Option<Body>) {
        self.scene.set_body(id, body);
    }

    /// The game's own collisions say volley piece `piece` (one of [`FxScene::launched`]) hit something: it strikes
    /// where its tip is and goes. False if it is not a piece in flight.
    pub fn burst(&mut self, piece: FxHandle) -> Result<bool> {
        self.scene.burst(piece.0).map_err(|e| anyhow!(e))
    }

    /// Lay `effect` (`projectiles/trail`) under `mover` as its trail: it draws the way `mover` came, curving with
    /// it, as long as it is fast, and ends soon after it. Paint it where `mover` is painted. None if `mover` is
    /// gone or `effect` draws no trail.
    pub fn trail(
        &mut self,
        mover: FxHandle,
        effect: &'static dyn Effect,
        opts: Options,
    ) -> Result<Option<FxHandle>> {
        Ok(self
            .scene
            .trail(mover.0, Clip::live(effect, opts))
            .map_err(|e| anyhow!(e))?
            .map(FxHandle))
    }

    /// What the volley `fx` was ordered to launch still plays: its shots in flight and what they played where
    /// they struck, placed from `fx`'s origin. Paint each where `fx` is painted; they outlive `fx` itself.
    pub fn launched(&self, fx: FxHandle) -> Vec<FxHandle> {
        self.scene.launched(fx.0).map(FxHandle).collect()
    }

    /// What happened since the last call, oldest first: effects that ended, cues they fired, and each shot of an
    /// ordered volley striking (`StageEvent::Strike`, in effect pixels from the ordered effect's origin).
    pub fn events(&mut self) -> Vec<StageEvent> {
        self.scene.drain_events()
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
        self.frames.set_budget(bytes);
    }

    /// Draw at most `layers` layers per window frame, or every changed one with `None` (the default); effects
    /// left over show their last frame and wait ([`Scene::set_draw_budget`]). Views take turns in the order they
    /// paint.
    pub fn set_draw_budget(&mut self, layers: Option<NonZeroU32>) {
        self.scene.set_draw_budget(layers);
    }

    /// Bytes the shared frames hold.
    pub fn bytes(&self) -> u64 {
        self.frames.bytes()
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
        if !std::mem::replace(&mut self.received, true) {
            Self::receive(&mut self.frames, &mut self.scene, &mut shell)?;
        }
        self.scene
            .draw_handle(fx.0, layer, &mut self.frames, &mut shell)
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
        self.received = false;
        self.scene.end_frame(&mut self.frames, &mut shell)?;
        self.frames.end_frame(&mut shell)?;
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

/// Images a thread made: held as they are, the images they replace dropped at the end of the frame.
impl Receive<Painted> for GpuiShell<'_> {
    fn make_from(&mut self, painted: &Painted) -> Result<Painted> {
        Ok(painted.clone())
    }

    fn refill_from(&mut self, surface: &mut Painted, painted: &Painted) -> Result<()> {
        let old = std::mem::replace(surface, painted.clone());
        self.retired.extend(old.images());
        Ok(())
    }

    fn bytes_of(&self, painted: &Painted) -> u64 {
        painted.bytes()
    }
}
