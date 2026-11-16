//! How a moving thing moves, read off its [`Trace`]: where it was and how it was turned at any age, its velocity,
//! speed and heading, how fast its way turns and how fast it spins, and its acceleration. Every effect that reacts
//! to motion (a trail, a flame carried along, sparks thrown back) reads it here, so they agree.
//!
//! Each rate is a difference across a `window` of seconds centred on the age asked (cut to the trace), so one
//! jittery point does not swing it. A trace of one point, a still one, or one whose points share an age (a
//! hit-stop) has no motion: zero, never NaN.

use core::f64::consts::{PI, TAU};

use crate::trace::Trace;

/// Below this many pixels a second a thing is still: it has no heading.
const STILL: f64 = 1e-6;

/// The shortest turn from angle `a` to angle `b`, -PI to PI.
fn turn_to(a: f64, b: f64) -> f64 {
    let d = libm::fmod(b - a + PI, TAU);
    (if d < 0.0 { d + TAU } else { d }) - PI
}

impl Trace {
    /// Where it was `age` seconds ago and how it was turned: `(x, y, angle)`, between two points by their ages
    /// (the angle the shortest way), held at the newest and the oldest point past either end. None when empty.
    pub fn at(&self, age: f64) -> Option<(f64, f64, f64)> {
        let first = self.points.first()?;
        if age <= first.age {
            return Some((first.x, first.y, first.angle));
        }
        for pair in self.points.windows(2) {
            let (a, b) = (pair[0], pair[1]);
            if age <= b.age {
                let span = b.age - a.age;
                let u = if span > 1e-12 {
                    (age - a.age) / span
                } else {
                    1.0
                };
                return Some((
                    a.x + (b.x - a.x) * u,
                    a.y + (b.y - a.y) * u,
                    a.angle + turn_to(a.angle, b.angle) * u,
                ));
            }
        }
        self.points.last().map(|l| (l.x, l.y, l.angle))
    }

    /// The ages `window` seconds round `age`, cut to the trace: (newer, older), or None when they meet.
    fn span(&self, age: f64, window: f64) -> Option<(f64, f64)> {
        let (newest, oldest) = (self.points.first()?.age, self.points.last()?.age);
        let half = window.max(0.0) / 2.0;
        let (a, b) = ((age - half).max(newest), (age + half).min(oldest));
        (b - a > 1e-9).then_some((a, b))
    }

    /// Pixels a second it moved, `(vx, vy)`, round `age`.
    pub fn velocity(&self, age: f64, window: f64) -> (f64, f64) {
        let Some((a, b)) = self.span(age, window) else {
            return (0.0, 0.0);
        };
        match (self.at(a), self.at(b)) {
            // it went from the older place to the newer one
            (Some(new), Some(old)) => ((new.0 - old.0) / (b - a), (new.1 - old.1) / (b - a)),
            _ => (0.0, 0.0),
        }
    }

    /// Pixels a second round `age`.
    pub fn speed(&self, age: f64, window: f64) -> f64 {
        let (vx, vy) = self.velocity(age, window);
        libm::hypot(vx, vy)
    }

    /// The way it went round `age` (radians, 0 along +x, y down), or None while it was still.
    pub fn heading(&self, age: f64, window: f64) -> Option<f64> {
        let (vx, vy) = self.velocity(age, window);
        (libm::hypot(vx, vy) > STILL).then(|| libm::atan2(vy, vx))
    }

    /// How fast its way turned round `age`, radians a second (0 while it was still).
    pub fn turn(&self, age: f64, window: f64) -> f64 {
        let Some((a, b)) = self.span(age, window) else {
            return 0.0;
        };
        match (self.heading(a, window), self.heading(b, window)) {
            (Some(new), Some(old)) => turn_to(old, new) / (b - a),
            _ => 0.0,
        }
    }

    /// How fast it spun (its own angle) round `age`, radians a second.
    pub fn spin(&self, age: f64, window: f64) -> f64 {
        let Some((a, b)) = self.span(age, window) else {
            return 0.0;
        };
        match (self.at(a), self.at(b)) {
            (Some(new), Some(old)) => turn_to(old.2, new.2) / (b - a),
            _ => 0.0,
        }
    }

    /// How its velocity changed round `age`, pixels a second a second.
    pub fn acceleration(&self, age: f64, window: f64) -> (f64, f64) {
        let Some((a, b)) = self.span(age, window) else {
            return (0.0, 0.0);
        };
        let (new, old) = (self.velocity(a, window), self.velocity(b, window));
        ((new.0 - old.0) / (b - a), (new.1 - old.1) / (b - a))
    }

    /// The trace up to `secs` old as poses for [`pixfx_draw::sweep::sweep`], newest first: every point younger
    /// than `secs`, then the one at `secs` exactly (between the two points round it), so a streak ends where its
    /// age does.
    pub fn poses(&self, secs: f64) -> Vec<pixfx_draw::sweep::Pose> {
        let pose =
            |(x, y, angle): (f64, f64, f64), age: f64| pixfx_draw::sweep::Pose { x, y, angle, age };
        let mut out: Vec<_> = self
            .points
            .iter()
            .take_while(|q| q.age < secs)
            .map(|q| pose((q.x, q.y, q.angle), q.age))
            .collect();
        let older = self.points.iter().any(|q| q.age >= secs);
        if older && let Some(end) = self.at(secs) {
            out.push(pose(end, secs));
        }
        out
    }
}
