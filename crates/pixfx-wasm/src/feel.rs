//! The game's answer to impacts for JS ([`Feel`] over [`pixfx_core::Feel`]): a host feeds it the cues its effects
//! fire ([`crate::Fx::take_cues`]) and its own impacts, and each frame asks how much play time passes and where the
//! view sits.

use pixfx_core::cue::{Cue, TimedCue};
use pixfx_core::feel::{self, FeelSpec};
use serde::Deserialize;
use wasm_bindgen::prelude::*;

use crate::js::{pair, parse, to_js};
use js_sys::Array;

/// A cue as JS hands it back: what [`crate::Fx::take_cues`] gives (a sound's name is the host's, so it is read
/// and let be).
#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "lowercase", deny_unknown_fields)]
enum CueIn {
    Shake {
        trauma: f64,
    },
    Hitstop {
        dur: f64,
    },
    Ripple {
        x: f64,
        y: f64,
        r0: f64,
        r1: f64,
        dur: f64,
        amp: f64,
    },
    Flash {
        k: f64,
        dur: f64,
    },
    Sound {
        #[serde(rename = "name")]
        _name: String,
        #[serde(rename = "gain")]
        _gain: f64,
        #[serde(rename = "pitch")]
        _pitch: f64,
        #[serde(rename = "len")]
        _len: Option<f64>,
        #[serde(rename = "x")]
        _x: Option<f64>,
    },
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct TimedCueIn {
    t: f64,
    pan: Option<f64>,
    cue: CueIn,
}

impl TimedCueIn {
    /// The cue the screen answers; a sound asks nothing of it.
    fn into_cue(self) -> Option<TimedCue> {
        let cue = match self.cue {
            CueIn::Shake { trauma } => Cue::Shake { trauma },
            CueIn::Hitstop { dur } => Cue::Hitstop { dur },
            CueIn::Ripple {
                x,
                y,
                r0,
                r1,
                dur,
                amp,
            } => Cue::Ripple {
                x,
                y,
                r0,
                r1,
                dur,
                amp,
            },
            CueIn::Flash { k, dur } => Cue::Flash { k, dur },
            CueIn::Sound { .. } => return None,
        };
        Some(TimedCue {
            t: self.t,
            cue,
            pan: self.pan,
        })
    }
}

/// The game's answer to impacts: trauma and a smooth shake, a hit-stop that freezes play, slow motion, screen
/// ripples and a flash.
#[wasm_bindgen]
pub struct Feel {
    inner: feel::Feel,
}

impl Default for Feel {
    fn default() -> Self {
        Self {
            inner: feel::Feel::new(),
        }
    }
}

#[wasm_bindgen]
impl Feel {
    /// `spec`: `undefined`, or `{shake?, roll?, shakeRate?, traumaDrain?, freezeCap?, flashFade?}` over the
    /// defaults (16 game pixels of shake and 0.05 radians of roll at full trauma, 12 noise cells a second, trauma
    /// drains 1 a second, a cue's hit-stop at most 13/60 s, a flash fades 4 a second).
    #[wasm_bindgen(constructor)]
    pub fn new(
        #[wasm_bindgen(unchecked_optional_param_type = "FeelSpec")] spec: JsValue,
    ) -> Result<Feel, JsError> {
        let spec: FeelSpec = if spec.is_undefined() || spec.is_null() {
            FeelSpec::default()
        } else {
            parse("feel spec", &spec)?
        };
        Ok(Feel {
            inner: feel::Feel::with_spec(spec),
        })
    }

    /// Take in the cues effects fired, as [`crate::Fx::take_cues`] gives them (`{t, pan, cue}`); sounds are the
    /// host's own and are skipped here.
    pub fn take(
        &mut self,
        #[wasm_bindgen(unchecked_param_type = "TimedCue[]")] cues: JsValue,
    ) -> Result<(), JsError> {
        let list: Vec<TimedCueIn> = parse("cues", &cues)?;
        let cues: Vec<TimedCue> = list.into_iter().filter_map(TimedCueIn::into_cue).collect();
        self.inner.take(&cues);
        Ok(())
    }

    /// The game's own impact shakes the view: `k` (0 to 1) is added to the trauma.
    #[wasm_bindgen(js_name = addTrauma)]
    pub fn add_trauma(&mut self, k: f64) {
        self.inner.add_trauma(k);
    }

    /// The game's own hit-stop: freeze play for `secs` seconds. Freezes do not add up; the longest wins.
    pub fn freeze(&mut self, secs: f64) {
        self.inner.freeze(secs);
    }

    /// Slow motion: play runs at `speed` (0 to 1) of real time for the next `secs` seconds, after any freeze.
    pub fn slow(&mut self, secs: f64, speed: f64) {
        self.inner.slow(secs, speed);
    }

    /// Let `dt` real seconds pass; returns the seconds play runs for them (none in a freeze, a fraction in slow
    /// motion).
    pub fn advance(&mut self, dt: f64) -> f64 {
        self.inner.advance(dt)
    }

    /// Where the view sits this frame, `[dx, dy]` in game pixels. `strength` is the player's setting: 1 as
    /// designed, 0 holds the view still.
    #[wasm_bindgen(unchecked_return_type = "[number, number]")]
    pub fn shake(&self, strength: f64) -> Array {
        let (x, y) = self.inner.shake(strength);
        pair(x.into(), y.into())
    }

    /// How far the view tilts this frame, in radians (positive is clockwise). `strength` as in `shake`.
    pub fn roll(&self, strength: f64) -> f64 {
        self.inner.roll(strength)
    }

    /// How shaken the view is, 0 to 1.
    pub fn trauma(&self) -> f64 {
        self.inner.trauma()
    }

    /// Play is frozen by a hit-stop now.
    pub fn frozen(&self) -> bool {
        self.inner.frozen()
    }

    /// How white the view is lifted, 0 to 1.
    pub fn flash(&self) -> f64 {
        self.inner.flash()
    }

    /// The screen ripples alive now: `{x, y, r0, r1, dur, amp, age}`.
    #[wasm_bindgen(unchecked_return_type = "Ripple[]")]
    pub fn ripples(&self) -> Result<JsValue, JsError> {
        to_js(self.inner.ripples())
    }
}
