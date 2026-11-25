//! A projected ribbon wrapped along the visible span of a sprite.

use core::f64::consts::TAU;
use std::sync::Arc;

use super::flight::Axis;
use pixfx_core::{Actor, ColorOpt, EffectState, Layer, NumOpt, Options};
use pixfx_draw::color::{WHITE, mix};
use pixfx_draw::math::{lerp, smooth};
use pixfx_draw::{Painter, RULER, Rgba, Ruler};

pub(super) const SPREAD: NumOpt = NumOpt::new("spread", 1.0).within(0.7, 1.4);
#[cfg(any(feature = "projectiles-twin-sparks", feature = "projectiles-corkscrew"))]
pub(super) const COLOR: ColorOpt = ColorOpt::new("color", Rgba::hex24(0x41edff));

pub(super) struct Style {
    pub period: f64,
    pub strands: u32,
    pub turns: f64,
    pub radius: f64,
    pub depth: f64,
    pub tail: f64,
    pub pulse: f64,
    pub tip: f64,
    pub color: ColorOpt,
}

struct Span {
    tail: f64,
    nose: f64,
    center: f64,
    radius: f64,
}

pub(super) struct Ribbon {
    style: Style,
    axis: Axis,
    spread: f64,
    tint: Rgba,
    span: Option<Span>,
}

impl Ribbon {
    pub fn new(opts: &Options, style: Style) -> Self {
        Self {
            axis: Axis::new(opts),
            spread: opts.num(&SPREAD),
            tint: opts
                .palette
                .as_deref()
                .and_then(|ramp| ramp.get(ramp.len() / 2))
                .copied()
                .unwrap_or_else(|| opts.color(&style.color)),
            style,
            span: None,
        }
    }
}

impl EffectState for Ribbon {
    fn set_actor(&mut self, actor: Option<Arc<Actor>>) {
        self.span = actor.and_then(|actor| {
            let bounds = self.axis.bounds(&actor)?;
            let (tail, nose, low, high) = (bounds.tail, bounds.nose, bounds.low, bounds.high);
            // Half an eye of breathing room on each side.
            let gap = f64::from(Ruler::size(RULER.eye.1, 0.5));
            let length = (nose - tail + 1.0).max(gap);
            Some(Span {
                tail: tail - gap - length * self.style.tail,
                nose,
                center: (low + high) * 0.5,
                radius: ((high - low + 1.0) * 0.5 + gap) * self.style.radius * self.spread,
            })
        });
    }

    fn draw(&self, p: &mut Painter, t: f64, layer: Layer) {
        let Some(span) = &self.span else {
            return;
        };
        let near = layer == Layer::Front;
        let Axis(dx, dy) = self.axis;
        let dark = mix(self.tint, Rgba::hex24(0x16449e), 0.57);
        let steps = libm::ceil((span.nose - span.tail) * 1.6).clamp(24.0, 120.0) as u32;
        let clock = t / self.style.period;
        let flare = (1.0 - clock).fract();

        if near && self.style.tip > 0.0 {
            let length = f64::from(Ruler::size(RULER.blade, 0.27)) * self.style.tip;
            let join = span.nose - (span.nose - span.tail) * 0.18;
            let (x0, y0) = self.axis.point(join, span.center);
            let (x1, y1) = self.axis.point(span.nose + length, span.center);
            p.line(
                x0,
                y0,
                x1,
                y1,
                |u| mix(self.tint, WHITE, smooth(0.22, 0.92, u)),
                |u| smooth(0.0, 0.26, u),
            );
            p.light(x1, y1, self.tint, 0.42);
        }

        for strand in 0..self.style.strands {
            let offset = f64::from(strand) / f64::from(self.style.strands);
            let project = |u: f64| {
                let angle = TAU * (u * self.style.turns + clock + offset);
                let depth = libm::sin(angle);
                let envelope =
                    lerp(0.56, 1.0, smooth(0.0, 0.24, u)) * lerp(1.0, 0.12, smooth(0.68, 1.0, u));
                let radius = span.radius * envelope;
                let along = lerp(span.tail, span.nose, u) + radius * depth * self.style.depth;
                let across = span.center + radius * libm::cos(angle);
                let (x, y) = self.axis.point(along, across);
                (x, y, depth)
            };

            let mut last = project(0.0);
            for step in 1..=steps {
                let u = f64::from(step) / f64::from(steps);
                let next = project(u);
                let depth = (last.2 + next.2) * 0.5;
                if (depth >= 0.0) == near {
                    let opacity = smooth(0.0, 0.1, u) * (1.0 - smooth(0.86, 1.0, u));
                    let pulse = (1.0 - smooth(0.02, 0.26, (u - flare).abs())) * self.style.pulse;
                    let core = smooth(0.14, 0.84, depth) * 0.86 + pulse * 0.58;
                    let color = if near {
                        mix(self.tint, WHITE, core.min(1.0))
                    } else {
                        mix(dark, self.tint, smooth(0.03, 0.82, -depth))
                    };

                    if near && core > 0.53 {
                        p.line(
                            last.0 - dx,
                            last.1 - dy,
                            next.0 - dx,
                            next.1 - dy,
                            self.tint,
                            opacity * (0.65 + 0.3 * pulse),
                        );
                    }
                    p.line(last.0, last.1, next.0, next.1, color, opacity);
                    if near && pulse > 0.4 {
                        p.light(next.0, next.1, self.tint, pulse * 0.45);
                    }
                }
                last = next;
            }
        }
    }
}
