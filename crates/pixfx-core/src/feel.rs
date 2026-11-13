//! The game's answer to an impact, for any game: a camera shake driven by trauma, a hit-stop that freezes play
//! for a few frames, slow motion, screen ripples and a flash. It takes the cues effects fire
//! ([`crate::cue::Cue`]) and the game's own impacts ([`Feel::add_trauma`], [`Feel::freeze`], [`Feel::slow`]),
//! and each frame tells the game how much play time passes ([`Feel::advance`]), where the screen sits
//! ([`Feel::shake`]) and how far it tilts ([`Feel::roll`]).
//!
//! The rules follow the usual game-feel practice: trauma 0 to 1 adds up per impact and drains over time, the
//! shake (x, y and roll) is a maximum times trauma squared (light hits stay subtle, heavy ones read as heavy) times
//! smooth Perlin noise (the screen sways rather than jumping to a new spot each frame); hit-stops do not add up, the longest one wins, under a cap; the
//! freeze runs on real time, and ripples and flash keep moving through it.

use crate::cue::{Cue, TimedCue};
use crate::math::{js_max, js_min, js_round, perlin, to_int32};
use crate::screen::Ripple;

/// What is left of a timer after `dt`, counted in whole frames: what is left after the last one is rounding, not
/// another frame.
fn run_down(left: f64, dt: f64) -> f64 {
    if left - dt > 1e-6 { left - dt } else { 0.0 }
}

/// How a [`Feel`] answers. `Default` is a pixel-art game's: a few game pixels of shake at most.
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize))]
#[cfg_attr(feature = "ts", derive(tsify::Tsify))]
#[cfg_attr(
    feature = "serde",
    serde(rename_all = "camelCase", deny_unknown_fields, default)
)]
pub struct FeelSpec {
    /// Game pixels the screen moves at full trauma.
    pub shake: f64,
    /// Radians the screen tilts at full trauma. A host that keeps a pixel grid (a canvas stage) leaves it out.
    pub roll: f64,
    /// How fast the shake wanders: smooth noise at this many cells a second.
    pub shake_rate: f64,
    /// Trauma drains this much a second.
    pub trauma_drain: f64,
    /// The longest hit-stop an effect's cue can ask for, in seconds.
    pub freeze_cap: f64,
    /// A flash fades this much a second.
    pub flash_fade: f64,
}

impl Default for FeelSpec {
    fn default() -> Self {
        Self {
            shake: 16.0,
            roll: 0.05,
            shake_rate: 12.0,
            trauma_drain: 1.0,
            freeze_cap: 13.0 / 60.0,
            flash_fade: 4.0,
        }
    }
}

/// The screen's state over a game's impacts and real time.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Feel {
    spec: FeelSpec,
    trauma: f64,
    stop: f64,
    slow: f64,
    slow_speed: f64,
    flash: f64,
    ripples: Vec<Ripple>,
    /// Real seconds gone by, for the shake's noise.
    clock: f64,
}

impl Feel {
    /// Answering as [`FeelSpec::default`] has it.
    pub fn new() -> Self {
        Self::default()
    }

    /// Answering as `spec` has it.
    pub fn with_spec(spec: FeelSpec) -> Self {
        Self {
            spec,
            ..Self::default()
        }
    }

    /// How it answers.
    pub fn spec(&self) -> &FeelSpec {
        &self.spec
    }

    /// Take in the cues effects fired: shake adds trauma, a hit-stop joins the one freeze (under the cap),
    /// ripples and flash go to the screen. Sounds are the game's own.
    pub fn take(&mut self, cues: &[TimedCue]) {
        for c in cues {
            match c.cue {
                Cue::Shake { trauma } => self.add_trauma(trauma),
                Cue::Hitstop { dur } => self.freeze(js_min(self.spec.freeze_cap, dur)),
                Cue::Ripple {
                    x,
                    y,
                    r0,
                    r1,
                    dur,
                    amp,
                } => self.ripples.push(Ripple {
                    x,
                    y,
                    r0,
                    r1,
                    dur,
                    amp,
                    age: 0.0,
                }),
                Cue::Flash { k, .. } => self.flash = js_max(self.flash, k),
                Cue::Sound { .. } => {}
            }
        }
    }

    /// Shake the screen harder: `k` (0 to 1) is added to the trauma, which stays at most 1.
    pub fn add_trauma(&mut self, k: f64) {
        if k.is_finite() {
            self.trauma = js_max(0.0, js_min(1.0, self.trauma + k));
        }
    }

    /// Freeze play for `secs` seconds: freezes do not add up, the longest one still to run wins.
    pub fn freeze(&mut self, secs: f64) {
        if secs.is_finite() {
            self.stop = js_max(self.stop, secs);
        }
    }

    /// Run play at `speed` (0 to 1) of real time for the next `secs` seconds, after any freeze.
    pub fn slow(&mut self, secs: f64, speed: f64) {
        if secs.is_finite() && speed.is_finite() {
            self.slow = js_max(0.0, secs);
            self.slow_speed = js_max(0.0, js_min(1.0, speed));
        }
    }

    /// Let `dt` real seconds pass. Returns how many seconds play should run for them: none during a freeze, a
    /// fraction in slow motion, all of them otherwise.
    pub fn advance(&mut self, dt: f64) -> f64 {
        if !dt.is_finite() || dt <= 0.0 {
            return 0.0;
        }
        self.clock += dt;
        self.trauma = js_max(0.0, self.trauma - self.spec.trauma_drain * dt);
        // the screen's own clock: ripples and flash run on through a freeze
        self.flash = js_max(0.0, self.flash - dt * self.spec.flash_fade);
        for r in &mut self.ripples {
            r.age += dt;
        }
        self.ripples.retain(|r| r.age <= r.dur);
        if self.stop > 0.0 {
            self.stop = run_down(self.stop, dt);
            return 0.0;
        }
        if self.slow > 0.0 {
            self.slow = run_down(self.slow, dt);
            return dt * self.slow_speed;
        }
        dt
    }

    /// Where the screen sits this frame, in whole game pixels: trauma squared times smooth noise over real time.
    /// `strength` scales it for the player's setting: 1 as designed, 0 (or anything not above 0) holds the
    /// screen still.
    pub fn shake(&self, strength: f64) -> (i32, i32) {
        let a = self.amount(strength) * self.spec.shake;
        let x = to_int32(js_round(self.wander(0.0) * a));
        let y = to_int32(js_round(self.wander(1.0) * a));
        (x, y)
    }

    /// How far the screen tilts this frame, in radians (positive is clockwise on a y-down screen): trauma
    /// squared times smooth noise, up to [`FeelSpec::roll`]. `strength` as in [`Feel::shake`].
    pub fn roll(&self, strength: f64) -> f64 {
        self.amount(strength) * self.spec.roll * self.wander(2.0)
    }

    /// Trauma squared at the player's `strength` (0 when it is not above 0).
    fn amount(&self, strength: f64) -> f64 {
        let strength = if strength > 0.0 { strength } else { 0.0 };
        self.trauma * self.trauma * strength
    }

    /// One axis of the shake (`axis` 0, 1 or 2): smooth noise over real time with its own seed. Each axis reads
    /// the noise a third of a cell apart, since Perlin noise is 0 at every whole cell: in step, every axis would
    /// pass the centre together and the screen would snap back to rest each cell.
    fn wander(&self, axis: f64) -> f64 {
        perlin(self.clock * self.spec.shake_rate + axis / 3.0, axis + 1.0)
    }

    /// How shaken the screen is, 0 to 1.
    pub fn trauma(&self) -> f64 {
        self.trauma
    }

    /// Play is frozen now.
    pub fn frozen(&self) -> bool {
        self.stop > 0.0
    }

    /// How white the screen is lifted, 0 to 1, for [`crate::screen::flash`].
    pub fn flash(&self) -> f64 {
        self.flash
    }

    /// The screen ripples alive now, for [`crate::screen::ripple`].
    pub fn ripples(&self) -> &[Ripple] {
        &self.ripples
    }
}
