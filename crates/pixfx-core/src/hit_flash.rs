//! A hit flash for any sprite: struck, it is drawn as a solid silhouette (white, or the game's colour) for a few
//! frames. Pure numbers: the host fills its sprite while [`HitFlash::active`], or blends the fill in by
//! [`HitFlash::amount`] where it can draw at partial strength. It runs on play time, so a hit-stop holds the
//! struck sprite lit for the whole freeze, the way fighting games show who was hit.

use pixfx_draw::math::clamp;

/// How long a hit flashes when the game does not say: 5 frames at 60 a second.
pub const HIT_FLASH: f64 = 5.0 / 60.0;

/// One sprite's flash.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct HitFlash {
    left: f64,
    dur: f64,
}

impl HitFlash {
    /// Not flashing.
    pub fn new() -> Self {
        Self::default()
    }

    /// It was struck: flash for `secs` seconds. A hit during a flash restarts it, for the longer of the two.
    pub fn start(&mut self, secs: f64) {
        if secs.is_finite() && secs > 0.0 && secs >= self.left {
            self.left = secs;
            self.dur = secs;
        }
    }

    /// Let `dt` seconds of play pass (0 during a hit-stop holds the flash). It counts whole frames: what is left
    /// after the last one is rounding, not another frame.
    pub fn advance(&mut self, dt: f64) {
        if dt.is_finite() && dt > 0.0 {
            self.left = if self.left - dt > 1e-6 {
                self.left - dt
            } else {
                0.0
            };
        }
    }

    /// The sprite is drawn filled now.
    pub fn active(&self) -> bool {
        self.left > 0.0
    }

    /// How strongly it is filled, 1 as it is struck down to 0 as the flash ends, for a host that blends the fill.
    pub fn amount(&self) -> f64 {
        if self.dur > 0.0 {
            clamp(self.left / self.dur, 0.0, 1.0)
        } else {
            0.0
        }
    }
}
