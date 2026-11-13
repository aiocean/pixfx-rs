//! What the canvas renderer decides, in plain Rust: which effects play, where, how much room round them, at
//! what scale, in which order their layers are drawn, when finished ones go, and the pixels each layer
//! becomes. No browser types, so it is tested natively; [`crate::Stage`] blits what this produces.

use std::fmt;
use std::num::NonZeroU32;

use js_sys::Reflect;
use pixfx_core::math::js_round;
use pixfx_core::{Effect, Frame, Layer, Options, composite_px, valid_dt};
use pixfx_wasm::{Player, parse_options};
use wasm_bindgen::{JsError, JsValue};

/// [`parse_options`] for [`crate::Stage::play`]: the stage sizes each effect's margin itself, so a `margin`
/// option is an error.
pub fn stage_options(v: &JsValue) -> Result<Options, JsError> {
    let has_margin = v.is_object()
        && Reflect::get(v, &JsValue::from_str("margin"))
            .map_err(|_| JsError::new("could not read the options"))?
            != JsValue::UNDEFINED;
    if has_margin {
        return Err(JsError::new(
            "option margin is not supported: Stage sizes the margin itself",
        ));
    }
    Ok(parse_options(v)?.0)
}

/// `effect` with `opts` as the stage plays it: on its canvas plus [`Meta::glow_margin`], so nothing at the
/// canvas edge is faded or cut.
pub fn live_player(effect: &'static dyn Effect, opts: Options) -> Player {
    Player::new(effect, opts, effect.meta().glow_margin())
}

/// A target rectangle in canvas pixels (game pixels times the scale).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rect {
    pub x: i64,
    pub y: i64,
    pub w: u64,
    pub h: u64,
}

/// One blit: `layer` of the instance `handle`, drawn into `rect`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Pass {
    pub handle: u32,
    pub layer: Layer,
    pub rect: Rect,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StageError {
    /// A position was NaN or infinite.
    NotFinite,
    /// Every `u32` handle has been given out once.
    OutOfHandles,
    /// A time step was NaN, infinite or negative.
    BadStep,
}

impl fmt::Display for StageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::NotFinite => "x and y must be finite numbers",
            Self::OutOfHandles => "every handle has been used",
            Self::BadStep => "dt must be a finite number of seconds, 0 or more",
        })
    }
}

impl std::error::Error for StageError {}

struct Entry<T> {
    handle: u32,
    /// Where the effect's origin sits, in whole game pixels.
    at: (i64, i64),
    player: Player,
    /// What the shell keeps per instance (its scratch canvas); `()` in tests.
    surface: T,
}

/// The playing instances, in the order they were played, and the game-to-canvas scale.
pub struct Scene<T> {
    scale: NonZeroU32,
    /// The next handle to give out; handles start at 1 and are never reused.
    next: Option<u32>,
    entries: Vec<Entry<T>>,
}

impl<T> Scene<T> {
    pub fn new(scale: NonZeroU32) -> Self {
        Self {
            scale,
            next: Some(1),
            entries: Vec::new(),
        }
    }

    pub fn scale(&self) -> NonZeroU32 {
        self.scale
    }

    pub fn set_scale(&mut self, scale: NonZeroU32) {
        self.scale = scale;
    }

    /// Add `player` with its origin at game pixel (`x`, `y`), rounded as JS `Math.round` does; returns its
    /// handle.
    pub fn play(&mut self, player: Player, x: f64, y: f64, surface: T) -> Result<u32, StageError> {
        if !x.is_finite() || !y.is_finite() {
            return Err(StageError::NotFinite);
        }
        let handle = self.next.ok_or(StageError::OutOfHandles)?;
        self.next = handle.checked_add(1);
        // `as` saturates; a position past i64 is off any canvas anyway
        let at = (js_round(x) as i64, js_round(y) as i64);
        self.entries.push(Entry {
            handle,
            at,
            player,
            surface,
        });
        Ok(handle)
    }

    /// Put the instance's origin at game pixel (`x`, `y`), rounded as [`Scene::play`] does, keeping its time:
    /// an effect that follows something moves without restarting. `Ok(false)` if no instance has this handle.
    pub fn move_to(&mut self, handle: u32, x: f64, y: f64) -> Result<bool, StageError> {
        if !x.is_finite() || !y.is_finite() {
            return Err(StageError::NotFinite);
        }
        let at = (js_round(x) as i64, js_round(y) as i64);
        Ok(self
            .entries
            .iter_mut()
            .find(|e| e.handle == handle)
            .map(|e| e.at = at)
            .is_some())
    }

    /// Let the instance end (loops fade out); false if no instance has this handle.
    pub fn stop(&mut self, handle: u32) -> bool {
        self.entries
            .iter_mut()
            .find(|e| e.handle == handle)
            .map(|e| e.player.stop())
            .is_some()
    }

    /// Drop the instance now; false if no instance has this handle.
    pub fn remove(&mut self, handle: u32) -> bool {
        let before = self.entries.len();
        self.entries.retain(|e| e.handle != handle);
        self.entries.len() != before
    }

    /// The playing handles, oldest first.
    pub fn handles(&self) -> impl Iterator<Item = u32> + '_ {
        self.entries.iter().map(|e| e.handle)
    }

    /// Where the instance's canvas lands on the target canvas.
    pub fn rect(&self, handle: u32) -> Option<Rect> {
        self.entries
            .iter()
            .find(|e| e.handle == handle)
            .map(|e| self.rect_of(e))
    }

    /// The instance's frame (its canvas plus margin) placed so the effect's origin lands on `e.at`.
    fn rect_of(&self, e: &Entry<T>) -> Rect {
        let s = u64::from(self.scale.get());
        let origin = e.player.canvas().origin;
        let pad = i64::from(e.player.margin());
        let frame = e.player.frame();
        let corner = |at: i64, origin: i32| {
            at.saturating_sub(i64::from(origin))
                .saturating_sub(pad)
                .saturating_mul(s as i64)
        };
        Rect {
            x: corner(e.at.0, origin.0),
            y: corner(e.at.1, origin.1),
            w: u64::from(frame.w) * s,
            h: u64::from(frame.h) * s,
        }
    }

    /// Seconds since the instance started.
    pub fn t(&self, handle: u32) -> Option<f64> {
        self.entries
            .iter()
            .find(|e| e.handle == handle)
            .map(|e| e.player.t())
    }

    /// Advance every instance by `dt` seconds, then drop the ones that are done. A `dt` that fails
    /// [`valid_dt`] (NaN, infinite or negative) is an error and changes nothing.
    pub fn update(&mut self, dt: f64) -> Result<(), StageError> {
        if !valid_dt(dt) {
            return Err(StageError::BadStep);
        }
        for e in &mut self.entries {
            e.player.update(dt);
        }
        self.entries.retain(|e| !e.player.done());
        Ok(())
    }

    /// Draw the back layer of every instance, then the front layer of every instance, oldest first, handing
    /// each drawn frame to `blit`. Stops at the first error `blit` returns.
    pub fn draw<E>(
        &mut self,
        mut blit: impl FnMut(Pass, &Frame, &mut T) -> Result<(), E>,
    ) -> Result<(), E> {
        self.draw_layer(Layer::Back, &mut blit)?;
        self.draw_layer(Layer::Front, &mut blit)
    }

    /// Draw only `layer` of every instance, oldest first, handing each drawn frame to `blit`. Stops at the
    /// first error `blit` returns.
    pub fn draw_layer<E>(
        &mut self,
        layer: Layer,
        mut blit: impl FnMut(Pass, &Frame, &mut T) -> Result<(), E>,
    ) -> Result<(), E> {
        for i in 0..self.entries.len() {
            let Some(e) = self.entries.get(i) else { break };
            let pass = Pass {
                handle: e.handle,
                layer,
                rect: self.rect_of(e),
            };
            let Some(e) = self.entries.get_mut(i) else {
                break;
            };
            e.player.draw(layer);
            blit(pass, e.player.frame(), &mut e.surface)?;
        }
        Ok(())
    }
}

/// Colour words (ABGR) as straight-alpha RGBA bytes into `out` (`w * h * 4`); true if any pixel is visible.
pub fn color_rgba(frame: &Frame, out: &mut [u8]) -> bool {
    let mut any = false;
    for (&c, o) in frame.color.iter().zip(out.as_chunks_mut::<4>().0) {
        *o = c.to_le_bytes();
        any |= c >> 24 != 0;
    }
    any
}

/// Light words (xBGR) as RGBA bytes into `out` (`w * h * 4`) for drawing with `"lighter"` (dst += src): each
/// lit pixel is light over transparency as [`composite_px`] makes it (alpha its brightest channel, rgb
/// `light * 255 / alpha`), so it adds its light to an opaque canvas and shows as that light's colour on a
/// transparent one; unlit pixels are fully transparent. True if any pixel is lit.
pub fn light_rgba(frame: &Frame, out: &mut [u8]) -> bool {
    let mut any = false;
    for (&l, o) in frame.light.iter().zip(out.as_chunks_mut::<4>().0) {
        // most of a frame is unlit, and unlit is transparent: skip the blend for it
        if l & 0x00ff_ffff == 0 {
            *o = [0; 4];
            continue;
        }
        *o = composite_px(0, l, None);
        any = true;
    }
    any
}
