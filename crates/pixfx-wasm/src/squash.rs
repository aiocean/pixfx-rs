//! A body's squash, stretch and blink for JS ([`Squash`], [`Blink`] over [`pixfx_core::squash`]): the host scales
//! its own sprite by `scale(vy)` about the feet and draws it at `alpha()`.

use pixfx_core::squash::{self, SquashSpec};
use wasm_bindgen::prelude::*;

use crate::js::parse;

fn spec_of(v: &JsValue) -> Result<SquashSpec, JsError> {
    if v.is_undefined() || v.is_null() {
        Ok(SquashSpec::default())
    } else {
        parse("squash spec", v)
    }
}

/// One body's squash and stretch: `land(strength)` when it lands, `advance(dt)` each frame, then scale the sprite
/// by `scale(vy)` (`[sx, sy]`, its area kept) with its origin at its feet.
#[wasm_bindgen]
pub struct Squash {
    inner: squash::Squash,
}

#[wasm_bindgen]
impl Squash {
    /// `spec`: `undefined`, or `{fullSpeed?, mostStretch?, mostSquash?, squashTime?, blinkRate?, blinkAlpha?}`
    /// over the defaults (stretched all the way at 220 px/s, 1.15 tall at most, 0.8 as it lands hard, easing
    /// back over 0.12 s).
    #[wasm_bindgen(constructor)]
    pub fn new(
        #[wasm_bindgen(unchecked_optional_param_type = "SquashSpec")] spec: JsValue,
    ) -> Result<Squash, JsError> {
        Ok(Squash {
            inner: squash::Squash::new(spec_of(&spec)?),
        })
    }

    /// It lands with `strength` (0 to 1: how hard, a fall's speed over the fastest you expect).
    pub fn land(&mut self, strength: f64) {
        self.inner.land(strength);
    }

    pub fn advance(&mut self, dt: f64) {
        self.inner.advance(dt);
    }

    /// The sprite's scale this frame, `[sx, sy]`, moving up or down at `vy` pixels a second.
    #[wasm_bindgen(unchecked_return_type = "[number, number]")]
    pub fn scale(&self, vy: f64) -> Vec<f64> {
        let (sx, sy) = squash::scale(self.inner.tall(vy));
        vec![sx, sy]
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
