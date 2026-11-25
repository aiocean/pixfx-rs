//! A band of elemental colour sweeps across the projectile from its leading edge.

use libm::{cos, sin};
use pixfx_core::cue::{FEEL, TimedCue, impact};
use pixfx_core::{Actor, ColorOpt, Effect, EffectState, Layer, Meta, NumOpt, Options};
use pixfx_draw::color::{WHITE, mix};
use pixfx_draw::{Painter, Rgba};
use std::sync::Arc;
const HEADING: NumOpt = NumOpt::new("heading", 0.0).within(0.0, 360.0);
const TINT: ColorOpt = ColorOpt::new("color", Rgba::hex24(0xff8138));
const SECS: f64 = 0.85;
pub struct ElementalCharge;
pub static ELEMENTAL_CHARGE: ElementalCharge = ElementalCharge;
const META: Meta = Meta {
    id: "feedback/elemental-charge",
    name: "Elemental Charge",
    desc: "A band of elemental colour sweeps across the projectile from its leading edge.",
    tags: &["feedback", "contact"],
    size: (128, 128),
    origin: (64, 64),
    duration: SECS,
    looping: false,
    fps: 60,
    seed: 1,
    state: None,
    options: &[FEEL.spec(), HEADING.spec(), TINT.spec()],
};
impl Effect for ElementalCharge {
    fn meta(&self) -> &Meta {
        &META
    }
    fn wraps_actor(&self) -> bool {
        true
    }
    fn cues(&self, opts: &Options) -> Vec<TimedCue> {
        impact(5.0 / 60.0, 0.06, 0.0, 0.0, opts)
    }

    fn spawn(&self, opts: &Options, _: u64) -> Box<dyn EffectState> {
        let angle = opts.num(&HEADING).to_radians();
        Box::new(State {
            direction: (cos(angle), sin(angle)),
            tint: opts
                .palette
                .as_deref()
                .and_then(|ramp| ramp.get(ramp.len() / 2))
                .copied()
                .unwrap_or_else(|| opts.color(&TINT)),
            pixels: Vec::new(),
            span: (0.0, 0.0),
        })
    }
}
struct State {
    direction: (f64, f64),
    tint: Rgba,
    pixels: Vec<(f64, f64, Rgba, f64)>,
    span: (f64, f64),
}
impl EffectState for State {
    fn set_actor(&mut self, actor: Option<Arc<Actor>>) {
        self.pixels.clear();
        if let Some(actor) = actor {
            self.pixels.extend(actor.opaque().map(|(x, y, color)| {
                let (x, y) = (f64::from(x), f64::from(y));
                (x, y, color, x * self.direction.0 + y * self.direction.1)
            }));
        }
        self.span = self
            .pixels
            .iter()
            .map(|px| px.3)
            .fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), v| {
                (lo.min(v), hi.max(v))
            });
    }
    fn draw(&self, p: &mut Painter, t: f64, layer: Layer) {
        if layer != Layer::Front {
            return;
        }
        let width = 4.0;
        let boundary =
            self.span.1 + width - (self.span.1 - self.span.0 + 2.0 * width) * (t / 0.32).min(1.0);
        for &(x, y, color, key) in &self.pixels {
            let charge = ((key - boundary) / width).clamp(0.0, 1.0);
            if charge <= 0.0 {
                continue;
            }
            let gleam = 4.0 * charge * (1.0 - charge);
            let tint = mix(mix(color, self.tint, 0.8), WHITE, gleam * 0.6);
            p.each_game_cell(x, y, |p, sx, sy| {
                p.put(sx, sy, tint, charge);
                p.add(sx, sy, self.tint, 0.38 * charge + 0.3 * gleam);
            });
        }
    }
}
