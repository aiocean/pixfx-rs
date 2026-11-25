//! Project the visible pixels onto and across a projectile's heading.

use pixfx_core::{Actor, NumOpt, Options};

pub(super) const HEADING: NumOpt = NumOpt::new("heading", 0.0).within(0.0, 360.0);

pub(super) struct Axis(pub f64, pub f64);

pub(super) struct Bounds {
    pub tail: f64,
    pub nose: f64,
    pub low: f64,
    pub high: f64,
}

impl Axis {
    pub fn new(opts: &Options) -> Self {
        let heading = opts.num(&HEADING).to_radians();
        Self(libm::cos(heading), libm::sin(heading))
    }

    pub fn bounds(&self, actor: &Actor) -> Option<Bounds> {
        let Self(dx, dy) = *self;
        let mut bounds = Bounds {
            tail: f64::INFINITY,
            nose: f64::NEG_INFINITY,
            low: f64::INFINITY,
            high: f64::NEG_INFINITY,
        };
        for (x, y, _) in actor.opaque() {
            let (x, y) = (f64::from(x), f64::from(y));
            let along = x * dx + y * dy;
            let across = y * dx - x * dy;
            bounds.tail = bounds.tail.min(along);
            bounds.nose = bounds.nose.max(along);
            bounds.low = bounds.low.min(across);
            bounds.high = bounds.high.max(across);
        }
        bounds.tail.is_finite().then_some(bounds)
    }

    pub fn point(&self, along: f64, across: f64) -> (f64, f64) {
        let Self(dx, dy) = *self;
        (along * dx - across * dy, along * dy + across * dx)
    }
}
