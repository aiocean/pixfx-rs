//! A body's shape and visibility answering how it moves, for any sprite: squash and stretch (it stretches along its
//! velocity when it moves fast, flattens as it lands and eases back, the other way the inverse so it keeps its
//! area) and a blink while it cannot be hurt. Pure numbers: the host draws its sprite through a [`Shape`] about the
//! feet (or [`Shape::axis_scale`] when it cannot skew), at [`Blink::alpha`]. The drawing stays the host's;
//! collision shapes keep their size.

use pixfx_draw::math::{clamp, ease, js_min};

/// How a body answers. `Default` suits a pixel-art body a few dozen pixels tall jumping at about 220 px/s.
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize))]
#[cfg_attr(feature = "ts", derive(tsify::Tsify))]
#[cfg_attr(
    feature = "serde",
    serde(rename_all = "camelCase", deny_unknown_fields, default)
)]
pub struct SquashSpec {
    /// Moving this fast any way (pixels a second), a body is stretched all the way.
    pub full_speed: f64,
    /// How long a body is drawn at most along its velocity while it moves fast, and how tall at least as it lands
    /// at full strength.
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
            most_stretch: 1.3,
            most_squash: 0.7,
            squash_time: 0.15,
            blink_rate: 7.5,
            blink_alpha: 0.35,
        }
    }
}

/// How long a body moving at `speed` (pixels a second) is drawn along its velocity against its own length: 1 at
/// rest, up to `most_stretch` at `full_speed` or faster.
pub fn stretch(spec: &SquashSpec, speed: f64) -> f64 {
    let k = if speed.is_finite() && spec.full_speed > 0.0 {
        js_min(1.0, speed.abs() / spec.full_speed)
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

/// A body's shape: how a point `(x, y)` from its feet (pixels, y down) is drawn, at `(a x + c y, b x + d y)` (a
/// canvas `transform(a, b, c, d, 0, 0)` about the feet). Every shape made here keeps the body's area (`a d - b c`
/// is 1) and leaves the feet where they are.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Shape {
    pub a: f64,
    pub b: f64,
    pub c: f64,
    pub d: f64,
}

impl Shape {
    /// The body as it is.
    pub const IDENTITY: Shape = Shape {
        a: 1.0,
        b: 0.0,
        c: 0.0,
        d: 1.0,
    };

    /// `tall` times its height and `1 / tall` its width.
    pub fn upright(tall: f64) -> Self {
        if tall.is_finite() && tall > 0.0 {
            Shape {
                a: 1.0 / tall,
                d: tall,
                ..Self::IDENTITY
            }
        } else {
            Self::IDENTITY
        }
    }

    /// `long` times its length along the direction `(dx, dy)` (y down) and `1 / long` across it.
    pub fn along(dx: f64, dy: f64, long: f64) -> Self {
        let n = libm::hypot(dx, dy);
        if !(n.is_finite() && n > 0.0 && long.is_finite() && long > 0.0) {
            return Self::IDENTITY;
        }
        let (ux, uy) = (dx / n, dy / n);
        let (across, k) = (1.0 / long, long - 1.0 / long);
        Shape {
            a: across + k * ux * ux,
            b: k * ux * uy,
            c: k * ux * uy,
            d: across + k * uy * uy,
        }
    }

    /// The point of the body drawn at `(x, y)` from its feet: where a host that fills each drawn pixel from the
    /// sprite reads it.
    pub fn source(&self, x: f64, y: f64) -> (f64, f64) {
        (self.d * x - self.c * y, self.a * y - self.b * x)
    }

    /// The nearest upright scale `(sx, sy)` with the area kept, for a host that can scale its sprite but not skew
    /// it: the shape's stretch along x and y, without its lean. A body moving exactly diagonally has none.
    pub fn axis_scale(&self) -> (f64, f64) {
        if self.a > 0.0 && self.d > 0.0 && self.a.is_finite() && self.d.is_finite() {
            let k = libm::sqrt(self.a / self.d);
            (k, 1.0 / k)
        } else {
            (1.0, 1.0)
        }
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
/// [`Squash::shape`] says how to draw it for how it moves.
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

    /// How to draw it this frame, moving at `(vx, vy)` pixels a second (y down): a landing's upright squash while
    /// it lasts, else stretched along its velocity for its speed.
    pub fn shape(&self, vx: f64, vy: f64) -> Shape {
        match self.landed {
            Some((since, s)) => Shape::upright(squash(&self.spec, since, s)),
            None => Shape::along(vx, vy, stretch(&self.spec, libm::hypot(vx, vy))),
        }
    }
}

/// A body's squash drawn by a host that cannot skew a sprite and may not know its velocity (a sprite without a
/// physics body): each frame [`Moving::step`] takes where it is and, when the game knows it, how fast it moves, and
/// says how to scale it.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Moving {
    pub squash: Squash,
    /// Where it was at the last step that passed time.
    last: Option<(f64, f64)>,
    /// The scale it was given then: a step that passes no time (a hit-stop) holds it.
    held: Option<(f64, f64)>,
}

impl Moving {
    pub fn new(spec: SquashSpec) -> Self {
        Self {
            squash: Squash::new(spec),
            last: None,
            held: None,
        }
    }

    /// Let `dt` seconds pass for a body now at `(x, y)`, moving at `velocity` px/s when the game knows it, else as
    /// fast as it moved since the last step, and return its upright scale `(sx, sy)`. A step that passes no time
    /// holds the last scale and moves nothing, so no speed spikes out of a hit-stop.
    pub fn step(&mut self, dt: f64, x: f64, y: f64, velocity: Option<(f64, f64)>) -> (f64, f64) {
        if !(dt.is_finite() && dt > 0.0) {
            return self.held.unwrap_or((1.0, 1.0));
        }
        let moved = self
            .last
            .map_or((0.0, 0.0), |(lx, ly)| ((x - lx) / dt, (y - ly) / dt));
        self.last = Some((x, y));
        self.squash.advance(dt);
        let (vx, vy) = velocity.unwrap_or(moved);
        let scale = self.squash.shape(vx, vy).axis_scale();
        self.held = Some(scale);
        scale
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
