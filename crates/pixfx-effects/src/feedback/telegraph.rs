//! A highlight travels across the supplied sprite or part mask during wind-up.

use std::sync::Arc;

use libm::{cos, sin};
use pixfx_core::{Actor, Effect, EffectState, Layer, Meta, Options};
use pixfx_core::{ColorOpt, NumOpt};
use pixfx_draw::ruler::{RULER, Ruler};
use pixfx_draw::{Painter, color::Rgba};

const SWEEP_ANGLE: NumOpt = NumOpt::new("sweepAngle", 270.0).within(0.0, 360.0);
const COLOR: ColorOpt = ColorOpt::new("color", Rgba::hex24(0xffce67));
const PROGRESS: NumOpt = NumOpt::new("progress", -1.0).within(-1.0, 1.0);

pub struct Telegraph;
pub static TELEGRAPH: Telegraph = Telegraph;

const DURATION: f64 = 0.6;

const META: Meta = Meta {
    id: "feedback/telegraph",
    name: "Telegraph",
    desc: "A highlight travels across the supplied sprite or part mask during wind-up.",
    tags: &["feedback", "character"],
    size: (128, 128),
    origin: (64, 64),
    duration: DURATION,
    looping: false,
    fps: 30,
    seed: 1,
    state: None,
    options: &[SWEEP_ANGLE.spec(), COLOR.spec(), PROGRESS.spec()],
};

impl Effect for Telegraph {
    fn meta(&self) -> &Meta {
        &META
    }

    fn wraps_actor(&self) -> bool {
        true
    }

    fn spawn(&self, opts: &Options, _seed: u64) -> Box<dyn EffectState> {
        let angle = opts.num(&SWEEP_ANGLE).to_radians();
        Box::new(State {
            direction: (cos(angle), sin(angle)),
            color: opts.color(&COLOR),
            progress: opts.num(&PROGRESS),
            pixels: Vec::new(),
            span: (0.0, 0.0),
        })
    }
}

struct State {
    direction: (f64, f64),
    color: Rgba,
    progress: f64,
    pixels: Vec<(f64, f64, f64)>,
    span: (f64, f64),
}

impl EffectState for State {
    fn update(&mut self, _dt: f64) {}

    fn set_actor(&mut self, actor: Option<Arc<Actor>>) {
        self.pixels.clear();
        if let Some(actor) = actor {
            self.pixels.extend(actor.opaque().map(|(x, y, _)| {
                let (x, y) = (f64::from(x), f64::from(y));
                (x, y, x * self.direction.0 + y * self.direction.1)
            }));
        }
        self.span = (f64::INFINITY, f64::NEG_INFINITY);
        for &(_, _, projection) in &self.pixels {
            self.span.0 = self.span.0.min(projection);
            self.span.1 = self.span.1.max(projection);
        }
    }

    fn draw(&self, p: &mut Painter, t: f64, layer: Layer) {
        if layer != Layer::Front {
            return;
        }
        let progress = if self.progress < 0.0 {
            t / DURATION
        } else {
            self.progress
        };
        let head = self.span.0 + progress * (self.span.1 - self.span.0);
        let width = f64::from(Ruler::size(RULER.hand, 1.5));
        for &(x, y, projection) in &self.pixels {
            let band = (1.0 - (head - projection).abs() / width).max(0.0);
            let intensity = band * 0.85 + if projection <= head { 0.2 } else { 0.0 };
            p.each_game_cell(x, y, |p, sx, sy| p.add(sx, sy, self.color, intensity));
        }
    }
}
