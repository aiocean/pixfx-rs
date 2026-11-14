//! A body's squash, stretch and blink for JS ([`Squash`], [`Blink`] over [`pixfx_core::squash`]): the host draws
//! its own sprite through `shape(vx, vy)` about the feet (or scales it by `scale(vx, vy)` when it cannot skew) and
//! at `alpha()`.

use pixfx_core::squash::{self, SquashSpec};
use wasm_bindgen::prelude::*;

use crate::js::{pair, parse};
use js_sys::Array;

fn spec_of(v: &JsValue) -> Result<SquashSpec, JsError> {
    if v.is_undefined() || v.is_null() {
        Ok(SquashSpec::default())
    } else {
        parse("squash spec", v)
    }
}

/// One body's squash and stretch: `land(strength)` when it lands, `advance(dt)` each frame, then draw the sprite
/// with its origin at its feet through `shape(vx, vy)` (stretched along its velocity), or scaled by
/// `scale(vx, vy)` where it cannot skew. Its area is kept.
#[wasm_bindgen]
pub struct Squash {
    inner: squash::Moving,
}

#[wasm_bindgen]
impl Squash {
    /// `spec`: `undefined`, or `{fullSpeed?, mostStretch?, mostSquash?, squashTime?, blinkRate?, blinkAlpha?}`
    /// over the defaults (stretched all the way at 220 px/s, 1.3 long at most along its velocity, 0.7 tall as it
    /// lands hard, easing back over 0.15 s).
    #[wasm_bindgen(constructor)]
    pub fn new(
        #[wasm_bindgen(unchecked_optional_param_type = "SquashSpec")] spec: JsValue,
    ) -> Result<Squash, JsError> {
        Ok(Squash {
            inner: squash::Moving::new(spec_of(&spec)?),
        })
    }

    /// It lands with `strength` (0 to 1: how hard, a fall's speed over the fastest you expect).
    pub fn land(&mut self, strength: f64) {
        self.inner.squash.land(strength);
    }

    pub fn advance(&mut self, dt: f64) {
        self.inner.squash.advance(dt);
    }

    /// For a host that cannot skew and may not know the body's velocity: let `dt` seconds pass for a body now at
    /// `(x, y)`, moving at `(vx, vy)` px/s when given (both finite), else as fast as it moved since the last step;
    /// returns its upright scale `[sx, sy]`. A step with no time (a hit-stop) holds the last scale. Use it instead
    /// of `advance` and `scale`.
    #[wasm_bindgen(unchecked_return_type = "[number, number]")]
    pub fn step(&mut self, dt: f64, x: f64, y: f64, vx: Option<f64>, vy: Option<f64>) -> Array {
        let velocity = match (vx, vy) {
            (Some(vx), Some(vy)) if vx.is_finite() && vy.is_finite() => Some((vx, vy)),
            _ => None,
        };
        let (sx, sy) = self.inner.step(dt, x, y, velocity);
        pair(sx, sy)
    }

    /// The sprite's shape this frame, moving at `(vx, vy)` pixels a second (y down): `[a, b, c, d]` for a
    /// canvas `transform(a, b, c, d, feetX, feetY)` drawn with the feet at the origin.
    #[wasm_bindgen(unchecked_return_type = "[number, number, number, number]")]
    pub fn shape(&self, vx: f64, vy: f64) -> Array {
        let s = self.inner.squash.shape(vx, vy);
        [s.a, s.b, s.c, s.d]
            .into_iter()
            .map(JsValue::from_f64)
            .collect()
    }

    /// The sprite's upright scale this frame, `[sx, sy]`, moving at `(vx, vy)` pixels a second: the shape
    /// without its lean, for a host that cannot skew a sprite.
    #[wasm_bindgen(unchecked_return_type = "[number, number]")]
    pub fn scale(&self, vx: f64, vy: f64) -> Array {
        let (sx, sy) = self.inner.squash.shape(vx, vy).axis_scale();
        pair(sx, sy)
    }
}

/// One body's blink while it cannot be hurt: `start(secs)`, `advance(dt)` each frame, draw it at `alpha()`. The
/// protection is the game's rule; this only shows it.
#[wasm_bindgen]
pub struct Blink {
    inner: squash::Blink,
}

#[wasm_bindgen]
impl Blink {
    /// `spec` as [`Squash::new`] reads it (`blinkRate`, `blinkAlpha` are the ones a blink uses).
    #[wasm_bindgen(constructor)]
    pub fn new(
        #[wasm_bindgen(unchecked_optional_param_type = "SquashSpec")] spec: JsValue,
    ) -> Result<Blink, JsError> {
        Ok(Blink {
            inner: squash::Blink::new(spec_of(&spec)?),
        })
    }

    /// Blink for `secs` seconds from now.
    pub fn start(&mut self, secs: f64) {
        self.inner.start(secs);
    }

    pub fn advance(&mut self, dt: f64) {
        self.inner.advance(dt);
    }

    /// Still blinking.
    pub fn active(&self) -> bool {
        self.inner.active()
    }

    /// How strongly to draw it this frame: 1, or faint on every other beat while it blinks.
    pub fn alpha(&self) -> f64 {
        self.inner.alpha()
    }
}
