//! Game-feel cues: the moments an effect wants the game to react to, timed to the frame, so a camera shake, a
//! hitstop, a screen ripple, a flash and a sound land exactly with the pixels that cause them. Port of
//! game-factory `src/core/cues.js` (`cuesOf`, `cuesBetween`).
//!
//! An effect lists its cues in its own space and time ([`Effect::cues`]); a loop's cues repeat every period.
//! An [`crate::effect::Instance`] reports them in world space as playback passes them
//! ([`crate::effect::Instance::take_cues`]).

use crate::effect::{Effect, Meta, Options};
use crate::math::clamp;

/// One thing an effect asks the game to do.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Cue {
    /// Camera shake, trauma 0..1 (the game squares it and decays it).
    Shake { trauma: f64 },
    /// Freeze the game for `dur` seconds (a few frames).
    Hitstop { dur: f64 },
    /// A ring of screen distortion growing from `r0` to `r1` over `dur`, centred on (`x`, `y`).
    Ripple {
        x: f64,
        y: f64,
        r0: f64,
        r1: f64,
        dur: f64,
        amp: f64,
    },
    /// A full-screen flash of strength `k` lasting `dur`.
    Flash { k: f64, dur: f64 },
    /// A sound by name (1.0 `pitch` = as designed); `len` caps its length in seconds; `x` is where it comes
    /// from in the effect's space, for pan (the effect's own position when `None`).
    Sound {
        name: &'static str,
        gain: f64,
        pitch: f64,
        len: Option<f64>,
        x: Option<f64>,
    },
}

/// A cue and when it happens, in seconds of the effect's playback.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TimedCue {
    pub t: f64,
    pub cue: Cue,
    /// Stereo position -1..1 across the screen width, set when the cue is fired with a screen width.
    pub pan: Option<f64>,
}

/// An effect's cues for `opts`, sorted by time (cues before time 0 are dropped).
pub fn cues_of(effect: &dyn Effect, opts: &Options) -> Vec<TimedCue> {
    let mut list = effect.cues(opts);
    list.retain(|c| c.t >= 0.0);
    list.sort_by(|a, b| a.t.total_cmp(&b.t));
    list
}

/// The cues of `list` that fall in `(t0, t1]` of an effect's playback, as world-space copies: a loop's repeat
/// every duration; x is mirrored by flip and offset by the effect's position, pan follows x across
/// `screen_w` (no pan when it is 0).
pub fn cues_between(
    list: &[TimedCue],
    t0: f64,
    t1: f64,
    meta: &Meta,
    opts: &Options,
    screen_w: f64,
) -> Vec<TimedCue> {
    let mut out = Vec::new();
    if list.is_empty() || t1 <= t0 {
        return out;
    }
    let d = meta.duration;
    let sx = if opts.flip { -1.0 } else { 1.0 };
    let (first, last) = if meta.looping {
        (libm::floor(t0 / d) as i64, libm::floor(t1 / d) as i64)
    } else {
        (0, 0)
    };
    for n in first..=last {
        for c in list {
            let at = c.t + n as f64 * d;
            if at <= t0 || at > t1 || (!meta.looping && c.t > d) {
                continue;
            }
            let mut w = *c;
            w.t = at;
            let mut world_x = None;
            match &mut w.cue {
                Cue::Ripple { x, y, .. } => {
                    *x = opts.x + sx * *x;
                    *y += opts.y;
                    world_x = Some(*x);
                }
                Cue::Sound { x: Some(x), .. } => {
                    *x = opts.x + sx * *x;
                    world_x = Some(*x);
                }
                _ => {}
            }
            if screen_w > 0.0 {
                let wx = world_x.unwrap_or(opts.x);
                w.pan = Some(clamp(wx / screen_w * 2.0 - 1.0, -1.0, 1.0));
            }
            out.push(w);
        }
    }
    out
}
