//! A body's shape and visibility answering how it moves, for any sprite: squash and stretch (it stretches along a
//! fast rise or fall, flattens as it lands and eases back, its width the inverse so it keeps its size) and a blink
//! while it cannot be hurt. Pure numbers: the host scales its sprite by [`scale`] about the feet, and draws it at
//! [`Blink::alpha`]. The drawing stays the host's; collision shapes keep their size.

use crate::math::{clamp, ease, js_min};

/// How a body answers. `Default` suits a pixel-art body a few dozen pixels tall jumping at about 220 px/s.
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize))]
#[cfg_attr(feature = "ts", derive(tsify::Tsify))]
#[cfg_attr(
    feature = "serde",
    serde(rename_all = "camelCase", deny_unknown_fields, default)
)]
pub struct SquashSpec {
    /// Moving this fast up or down (pixels a second), a body is stretched all the way.
    pub full_speed: f64,
    /// How tall a body is drawn at most while it flies fast, and at least as it lands at full strength.
    pub most_stretch: f64,
    pub most_squash: f64,
    /// A landing's squash eases back to the body's own shape over this long, in seconds.
    pub squash_time: f64,
    /// A blink turns faint this many times a second, drawn at `blink_alpha` while faint.
    pub blink_rate: f64,
    pub blink_alpha: f64,
}

impl Default for SquashSpec {
    fn default() -> Self {
        Self {
            full_speed: 220.0,
            most_stretch: 1.15,
            most_squash: 0.8,
            squash_time: 0.12,
            blink_rate: 7.5,
            blink_alpha: 0.35,
        }
    }
}

/// How tall a body moving up or down at `vy` (pixels a second) is drawn against its own height: 1 at rest, up to
/// `most_stretch` at `full_speed` or faster either way.
pub fn stretch(spec: &SquashSpec, vy: f64) -> f64 {
    let k = if vy.is_finite() && spec.full_speed > 0.0 {
        js_min(1.0, vy.abs() / spec.full_speed)
    } else {
        0.0
    };
    1.0 + (spec.most_stretch - 1.0) * k
}

/// How tall a body is drawn `since` seconds after it landed with `strength` (0 to 1): flattened toward
/// `most_squash`, easing back to 1 over `squash_time`.
pub fn squash(spec: &SquashSpec, since: f64, strength: f64) -> f64 {
    let k = if spec.squash_time > 0.0 {
        clamp(since / spec.squash_time, 0.0, 1.0)
    } else {
        1.0
    };
    let deep = (1.0 - spec.most_squash) * clamp(strength, 0.0, 1.0);
    1.0 - deep * (1.0 - ease::out_quad(k))
}

/// The scale to draw a body `tall` times its height at: `(1 / tall, tall)`, so its area stays the same.
pub fn scale(tall: f64) -> (f64, f64) {
    if tall.is_finite() && tall > 0.0 {
        (1.0 / tall, tall)
    } else {
        (1.0, 1.0)
    }
}

/// How strongly a blinking body is drawn `t` seconds into its blink: whole, then faint, in turn.
pub fn blink(spec: &SquashSpec, t: f64) -> f64 {
    let beat = libm::floor(t * spec.blink_rate * 2.0);
    if beat.is_finite() && beat % 2.0 != 0.0 {
        spec.blink_alpha
    } else {
        1.0
    }
}

/// One body's squash over time: it lands ([`Squash::land`]), time passes ([`Squash::advance`]), and each frame
/// [`Squash::tall`] says how tall to draw it for how fast it moves.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Squash {
    spec: SquashSpec,
    /// Seconds since the last landing and how hard it was; none before the first.
    landed: Option<(f64, f64)>,
}

impl Squash {
    pub fn new(spec: SquashSpec) -> Self {
        Self { spec, landed: None }
    }

    /// It lands with `strength` (0 to 1, how hard: a fall's speed over the fastest you expect).
    pub fn land(&mut self, strength: f64) {
        if strength.is_finite() && strength > 0.0 {
            self.landed = Some((0.0, strength));
        }
    }

    pub fn advance(&mut self, dt: f64) {
        if let Some((since, s)) = self.landed
            && dt.is_finite()
            && dt > 0.0
        {
            let since = since + dt;
            self.landed = (since < self.spec.squash_time).then_some((since, s));
        }
    }

    /// How tall to draw it this frame, moving up or down at `vy`: a landing's squash while it lasts, else the
    /// stretch of its speed.
    pub fn tall(&self, vy: f64) -> f64 {
        match self.landed {
            Some((since, s)) => squash(&self.spec, since, s),
            None => stretch(&self.spec, vy),
        }
    }
}

/// One body's blink while it cannot be hurt: [`Blink::start`] for how long, [`Blink::advance`], then draw it at
/// [`Blink::alpha`]. The protection itself is the game's rule; this only shows it.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Blink {
    spec: SquashSpec,
    t: f64,
    left: f64,
}

impl Blink {
    pub fn new(spec: SquashSpec) -> Self {
        Self {
            spec,
            t: 0.0,
            left: 0.0,
        }
    }

    /// Blink for `secs` seconds from now.
    pub fn start(&mut self, secs: f64) {
        if secs.is_finite() && secs > 0.0 {
            self.t = 0.0;
            self.left = secs;
        }
    }

    pub fn advance(&mut self, dt: f64) {
        if dt.is_finite() && dt > 0.0 && self.left > 0.0 {
            self.t += dt;
            self.left = (self.left - dt).max(0.0);
        }
    }

    /// Still blinking.
    pub fn active(&self) -> bool {
        self.left > 0.0
    }

    /// How strongly to draw it this frame: 1, or faint on every other beat while it blinks.
    pub fn alpha(&self) -> f64 {
        if self.active() {
            blink(&self.spec, self.t)
        } else {
            1.0
        }
    }
}
