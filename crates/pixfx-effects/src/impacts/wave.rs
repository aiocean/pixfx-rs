//! A pixel arc that travels across the ground or out from a sprite.

#[cfg(any(
    feature = "impacts-aftershock",
    feature = "impacts-directional-wave",
    feature = "impacts-charged-pulse"
))]
use libm::{cos, sin};
use pixfx_core::{Actor, ColorOpt, NumOpt, Options};
use pixfx_draw::Rgba;
#[cfg(any(
    feature = "impacts-aftershock",
    feature = "impacts-directional-wave",
    feature = "impacts-charged-pulse"
))]
use pixfx_draw::{
    Painter,
    color::{WHITE, mix},
};

pub(super) const CONTACT_X: NumOpt = NumOpt::new("contactX", 0.0).within(-48.0, 48.0);
pub(super) const CONTACT_Y: NumOpt = NumOpt::new("contactY", 0.0).within(-48.0, 48.0);
pub(super) const REACH: NumOpt = NumOpt::new("reach", 43.0).within(12.0, 54.0);

#[cfg(any(feature = "impacts-aftershock", feature = "impacts-directional-wave"))]
pub(super) fn contact_canvas(opts: &Options) -> pixfx_core::Canvas {
    let radius = opts.num(&REACH) + 6.0;
    let half = |offset: f64| (libm::ceil(offset.abs()) as u32 + libm::ceil(radius) as u32).max(76);
    let x = half(opts.num(&CONTACT_X));
    let y = half(opts.num(&CONTACT_Y));
    pixfx_core::Canvas {
        size: (x * 2, y * 2),
        origin: (x as i32, y as i32),
    }
}

pub(super) fn tint(opts: &Options, base: &ColorOpt) -> Rgba {
    opts.palette
        .as_deref()
        .and_then(|ramp| ramp.get(ramp.len() / 2))
        .copied()
        .unwrap_or_else(|| opts.color(base))
}

#[cfg(any(feature = "impacts-resonance", feature = "impacts-charged-pulse"))]
pub(super) fn center(actor: &Actor) -> Option<(f64, f64)> {
    let (x, y, n) = actor
        .opaque()
        .fold((0.0, 0.0, 0u32), |(ax, ay, n), (x, y, _)| {
            (ax + f64::from(x), ay + f64::from(y), n + 1)
        });
    (n > 0).then(|| (x / f64::from(n), y / f64::from(n)))
}

pub(super) fn nearest(actor: &Actor, at: (f64, f64)) -> Option<(f64, f64)> {
    actor
        .opaque()
        .map(|(x, y, _)| (f64::from(x), f64::from(y)))
        .min_by(|a, b| {
            let d = |(x, y): (f64, f64)| (x - at.0) * (x - at.0) + (y - at.1) * (y - at.1);
            d(*a).total_cmp(&d(*b))
        })
}

#[cfg(any(
    feature = "impacts-aftershock",
    feature = "impacts-directional-wave",
    feature = "impacts-charged-pulse"
))]
#[derive(Clone, Copy)]
pub(super) struct Wave {
    pub origin: (f64, f64),
    pub radius: f64,
    pub heading: f64,
    pub half_angle: f64,
    pub flatten: f64,
    pub color: Rgba,
}
#[cfg(any(
    feature = "impacts-aftershock",
    feature = "impacts-directional-wave",
    feature = "impacts-charged-pulse"
))]
impl Wave {
    #[cfg(any(feature = "impacts-aftershock", feature = "impacts-charged-pulse"))]
    pub(super) fn full(origin: (f64, f64), radius: f64, color: Rgba) -> Self {
        Self {
            origin,
            radius,
            heading: 0.0,
            half_angle: std::f64::consts::PI,
            flatten: 0.78,
            color,
        }
    }

    pub(super) fn draw(self, p: &mut Painter, fade: f64) {
        if fade <= 0.0 {
            return;
        }
        let count = (libm::ceil(self.radius * self.half_angle) as u32).clamp(6, 192);
        let point = |i: u32| {
            let angle =
                self.heading + self.half_angle * (2.0 * f64::from(i) / f64::from(count) - 1.0);
            (
                self.origin.0 + self.radius * cos(angle),
                self.origin.1 + self.radius * sin(angle) * self.flatten,
            )
        };
        let tint = mix(self.color, WHITE, fade * 0.52);
        let mut previous = point(0);
        for i in 1..=count {
            let next = point(i);
            p.line(previous.0, previous.1, next.0, next.1, tint, fade);
            if i % 4 == 0 {
                p.glow(next.0, next.1, 2.4, self.color, fade * 0.50);
            }
            previous = next;
        }
    }
}
