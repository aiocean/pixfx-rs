//! A crescent of wind bends off the sprite's leading edge.

use core::f64::consts::TAU;
use std::sync::Arc;

use super::flight::{Axis, Bounds, HEADING};
use pixfx_core::{Actor, ColorOpt, Effect, EffectState, Layer, Meta, NumOpt, Options};
use pixfx_draw::color::{WHITE, mix};
use pixfx_draw::math::{lerp, smooth};
use pixfx_draw::{Painter, RULER, Rgba, Ruler};

pub struct MoonWindblade;
pub static MOON_WINDBLADE: MoonWindblade = MoonWindblade;

const PERIOD: f64 = 0.6;
const SPREAD: NumOpt = NumOpt::new("spread", 1.0).within(0.7, 1.4);
const COLOR: ColorOpt = ColorOpt::new("color", Rgba::hex24(0xb7d68c));
const META: Meta = Meta {
    id: "projectiles/moon-windblade",
    name: "Moon Windblade",
    desc: "A bright crescent wraps the leading edge and sheds thin streaks of wind toward the tail.",
    tags: &["projectiles", "skill", "wind", "loop"],
    size: (160, 160),
    origin: (80, 80),
    duration: PERIOD,
    looping: true,
    fps: 50,
    seed: 1,
    state: None,
    options: &[HEADING.spec(), SPREAD.spec(), COLOR.spec()],
};

impl Effect for MoonWindblade {
    fn meta(&self) -> &Meta {
        &META
    }

    fn wraps_actor(&self) -> bool {
        true
    }

    fn spawn(&self, opts: &Options, _: u64) -> Box<dyn EffectState> {
        Box::new(State {
            axis: Axis::new(opts),
            bounds: None,
            spread: opts.num(&SPREAD),
            tint: opts
                .palette
                .as_deref()
                .and_then(|ramp| ramp.get(ramp.len() / 2))
                .copied()
                .unwrap_or_else(|| opts.color(&COLOR)),
            // Half an open eye of space between the sprite tip and the wind.
            gap: f64::from(Ruler::size(RULER.eye.1, 0.5)),
        })
    }
}

struct State {
    axis: Axis,
    bounds: Option<Bounds>,
    spread: f64,
    tint: Rgba,
    gap: f64,
}

impl EffectState for State {
    fn set_actor(&mut self, actor: Option<Arc<Actor>>) {
        self.bounds = actor.and_then(|actor| self.axis.bounds(&actor));
    }

    fn draw(&self, p: &mut Painter, t: f64, layer: Layer) {
        let Some(bounds) = &self.bounds else { return };
        let half = (bounds.high - bounds.low + 1.0) * 0.5;
        let radius = (half + self.gap * 1.5) * self.spread;
        let center = (bounds.low + bounds.high) * 0.5;
        let length = (bounds.nose - bounds.tail + 1.0).max(half);
        let curve = (length * 0.48 + radius * 0.27).min(radius * 1.45);
        let clock = t / PERIOD;
        let bend = |v: f64| {
            let ripple = libm::sin(TAU * (clock - v * 0.72)) * (1.0 - v * v) * 0.55;
            bounds.nose + self.gap - curve * v * v + ripple
        };

        if layer == Layer::Back {
            let dim = mix(self.tint, Rgba::hex24(0x32453c), 0.54);
            for i in 0..13 {
                let v = f64::from(i - 6) / 6.0;
                let phase = (clock + f64::from(i) * 0.381_966).fract();
                let reach = curve * lerp(0.2, 1.1, v.abs());
                let end = bend(v) - phase * reach * 0.57;
                let start = end - reach * lerp(0.24, 0.75, phase);
                let across = center + v * radius;
                let (x0, y0) = self.axis.point(start, across);
                let (x1, y1) = self.axis.point(end, across);
                p.line(x0, y0, x1, y1, dim, |u| {
                    smooth(0.0, 0.57, u) * (1.0 - smooth(0.48, 1.0, phase)) * 0.95
                });
            }
            return;
        }

        let steps = (libm::ceil(radius * 2.0) as u32).clamp(14, 128);
        for i in 0..steps {
            let v0 = -1.0 + 2.0 * f64::from(i) / f64::from(steps);
            let v1 = -1.0 + 2.0 * f64::from(i + 1) / f64::from(steps);
            let v = (v0 + v1) * 0.5;
            let edge = 1.0 - smooth(0.75, 1.0, v.abs());
            let gleam = 0.66 + 0.25 * libm::cos(TAU * (v * 1.12 - clock));
            let (x0, y0) = self.axis.point(bend(v0), center + v0 * radius);
            let (x1, y1) = self.axis.point(bend(v1), center + v1 * radius);
            let Axis(dx, dy) = self.axis;
            p.line(
                x0 - dx * 2.0,
                y0 - dy * 2.0,
                x1 - dx,
                y1 - dy,
                self.tint,
                edge * 0.55,
            );
            p.line(
                x0,
                y0,
                x1,
                y1,
                mix(self.tint, WHITE, gleam),
                0.37 + edge * 0.63,
            );
            if edge > 0.5 {
                p.light(x0, y0, self.tint, edge * 0.28);
            }
        }
    }
}
