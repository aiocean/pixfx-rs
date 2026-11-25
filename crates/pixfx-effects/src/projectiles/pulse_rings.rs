//! Three luminous hoops travel from the nose toward the tail of the wrapped sprite.

use core::f64::consts::PI;
use std::sync::{Arc, OnceLock};

use pixfx_core::{Actor, ColorOpt, Effect, EffectState, Layer, Meta, NumOpt, Options};
use pixfx_draw::color::{WHITE, mix};
use pixfx_draw::math::{lerp, smooth};
use pixfx_draw::path::Path;
use pixfx_draw::{Painter, RULER, Rgba, Ruler};

pub struct PulseRings;
pub static PULSE_RINGS: PulseRings = PulseRings;

const PERIOD: f64 = 0.72;
const RINGS: u32 = 3;
const HEADING: NumOpt = NumOpt::new("heading", 0.0).within(0.0, 360.0);
const SPREAD: NumOpt = NumOpt::new("spread", 1.0).within(0.65, 1.45);
const COLOR: ColorOpt = ColorOpt::new("color", Rgba::hex24(0x32e8ff));
const META: Meta = Meta {
    id: "projectiles/pulse-rings",
    name: "Pulse Rings",
    desc: "Luminous hoops wrap a sprite and stream from its leading edge to its tail.",
    tags: &["projectiles", "skill", "ring", "loop"],
    size: (160, 160),
    origin: (80, 80),
    duration: PERIOD,
    looping: true,
    fps: 50,
    seed: 1,
    state: None,
    options: &[HEADING.spec(), SPREAD.spec(), COLOR.spec()],
};

impl Effect for PulseRings {
    fn meta(&self) -> &Meta {
        &META
    }

    fn wraps_actor(&self) -> bool {
        true
    }

    fn spawn(&self, opts: &Options, _: u64) -> Box<dyn EffectState> {
        let angle = opts.num(&HEADING).to_radians();
        Box::new(State {
            axis: (libm::cos(angle), libm::sin(angle)),
            spread: opts.num(&SPREAD),
            tint: opts
                .palette
                .as_deref()
                .and_then(|ramp| ramp.get(ramp.len() / 2))
                .copied()
                .unwrap_or_else(|| opts.color(&COLOR)),
            // Half an open eye in clearance on either side of the sprite.
            clearance: f64::from(Ruler::size(RULER.eye.1, 0.5)),
            span: None,
        })
    }
}

struct Span {
    tail: f64,
    nose: f64,
    middle: f64,
    radius: f64,
}

struct State {
    axis: (f64, f64),
    spread: f64,
    tint: Rgba,
    clearance: f64,
    span: Option<Span>,
}

impl EffectState for State {
    fn set_actor(&mut self, actor: Option<Arc<Actor>>) {
        self.span = actor.and_then(|actor| {
            let (dx, dy) = self.axis;
            let (mut tail, mut nose) = (f64::INFINITY, f64::NEG_INFINITY);
            let (mut low, mut high) = (f64::INFINITY, f64::NEG_INFINITY);
            for (x, y, _) in actor.opaque() {
                let (x, y) = (f64::from(x), f64::from(y));
                let along = x * dx + y * dy;
                let across = y * dx - x * dy;
                tail = tail.min(along);
                nose = nose.max(along);
                low = low.min(across);
                high = high.max(across);
            }
            tail.is_finite().then_some(Span {
                tail: tail - self.clearance,
                nose: nose + self.clearance,
                middle: (low + high) * 0.5,
                radius: ((high - low + 1.0) * 0.7 + self.clearance) * self.spread,
            })
        });
    }

    fn draw(&self, p: &mut Painter, t: f64, layer: Layer) {
        let Some(span) = &self.span else {
            return;
        };
        let near = layer == Layer::Front;
        let side = if near { -1.0 } else { 1.0 };
        let edge = mix(self.tint, WHITE, if near { 0.88 } else { 0.48 });
        let (dx, dy) = self.axis;

        for index in 0..RINGS {
            let phase = (t / PERIOD + (f64::from(index) + 0.15) / f64::from(RINGS)).fract();
            let along = lerp(span.nose, span.tail, phase);
            let reveal = smooth(0.0, 0.14, phase);
            let fade = 1.0 - smooth(0.79, 1.0, phase);
            let energy = reveal * fade;
            let radius = span.radius * lerp(0.28, lerp(1.08, 0.55, phase), reveal);
            let depth = radius * 0.28;
            let project = |(u, v): (f64, f64)| {
                let along = along + u * depth * side;
                let across = span.middle + v * radius;
                (along * dx - across * dy, along * dy + across * dx)
            };

            let mut last = None;
            for &point in half_circle() {
                let (x, y) = project(point);
                if let Some((px, py)) = last {
                    p.line(px - dx, py - dy, x - dx, y - dy, self.tint, energy * 0.58);
                    p.line(px, py, x, y, edge, energy);
                }
                last = Some((x, y));
            }
            if near {
                for tip in [-1.0, 1.0] {
                    let (x, y) = project((0.0, tip));
                    p.light(x, y, self.tint, energy * 0.45);
                }
            }
        }
    }
}

fn half_circle() -> &'static [(f64, f64); 25] {
    static POINTS: OnceLock<[(f64, f64); 25]> = OnceLock::new();
    POINTS.get_or_init(|| {
        let path = Path::begin((0.0, -1.0)).turn((0.0, 0.0), PI);
        std::array::from_fn(|index| path.at(path.length() * index as f64 / 24.0))
    })
}
