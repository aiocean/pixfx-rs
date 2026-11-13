//! A sprite's hit flash for JS ([`HitFlash`] over [`pixfx_core::hit_flash`]): the host fills its sprite with one
//! colour while `active()`, or blends the fill in by `amount()`.

use pixfx_core::hit_flash::{self, HIT_FLASH};
use wasm_bindgen::prelude::*;

/// One sprite's hit flash: `start()` (or `start(secs)`) when it is struck, `advance(dt)` each frame with the play
/// time (0 in a hit-stop holds it lit), then draw the sprite filled while `active()`.
#[wasm_bindgen]
pub struct HitFlash {
    inner: hit_flash::HitFlash,
}

#[wasm_bindgen]
impl HitFlash {
    #[wasm_bindgen(constructor)]
    pub fn new() -> HitFlash {
        HitFlash {
            inner: hit_flash::HitFlash::new(),
        }
    }

    /// It was struck: flash for `secs` seconds (5 frames at 60 a second when left out). A shorter hit leaves a
    /// longer flash running.
    pub fn start(&mut self, secs: Option<f64>) {
        self.inner.start(secs.unwrap_or(HIT_FLASH));
    }

    pub fn advance(&mut self, dt: f64) {
        self.inner.advance(dt);
    }

    /// Draw the sprite filled now.
    pub fn active(&self) -> bool {
        self.inner.active()
    }

    /// How strongly it is filled, 1 as struck down to 0, for a host that blends the fill.
    pub fn amount(&self) -> f64 {
        self.inner.amount()
    }
}

impl Default for HitFlash {
    fn default() -> Self {
        Self::new()
    }
}
