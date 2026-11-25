//! The hidden portion of a sprite is revealed over an occluder.

use std::sync::Arc;

use pixfx_core::{Actor, Effect, EffectState, Layer, Meta, Options};
use pixfx_core::{ColorOpt, NumOpt};
use pixfx_draw::{Painter, color::Rgba};

const LEFT: NumOpt = NumOpt::new("left", -512.0).within(-512.0, 512.0);
const TOP: NumOpt = NumOpt::new("top", -512.0).within(-512.0, 512.0);
const WIDTH: NumOpt = NumOpt::new("width", 1024.0).within(0.0, 1024.0);
const HEIGHT: NumOpt = NumOpt::new("height", 1024.0).within(0.0, 1024.0);
const COLOR: ColorOpt = ColorOpt::new("color", Rgba::hex24(0xadc6ff));
const OPACITY: NumOpt = NumOpt::new("opacity", 0.7).within(0.0, 1.0);

pub struct Occlusion;
pub static OCCLUSION: Occlusion = Occlusion;

const META: Meta = Meta {
    id: "feedback/occlusion",
    name: "Occlusion Silhouette",
    desc: "The hidden portion of a sprite is revealed over an occluder.",
    tags: &["feedback", "character", "loop"],
    size: (128, 128),
    origin: (64, 64),
    duration: 1.0,
    looping: true,
    fps: 1,
    seed: 1,
    state: None,
    options: &[
        LEFT.spec(),
        TOP.spec(),
        WIDTH.spec(),
        HEIGHT.spec(),
        COLOR.spec(),
        OPACITY.spec(),
    ],
};

impl Effect for Occlusion {
    fn meta(&self) -> &Meta {
        &META
    }

    fn wraps_actor(&self) -> bool {
        true
    }

    fn spawn(&self, opts: &Options, _seed: u64) -> Box<dyn EffectState> {
        Box::new(State {
            left: opts.num(&LEFT),
            top: opts.num(&TOP),
            width: opts.num(&WIDTH),
            height: opts.num(&HEIGHT),
            color: opts.color(&COLOR),
            opacity: opts.num(&OPACITY),
            cells: Vec::new(),
        })
    }
}

struct State {
    left: f64,
    top: f64,
    width: f64,
    height: f64,
    color: Rgba,
    opacity: f64,
    cells: Vec<(f64, f64)>,
}

impl EffectState for State {
    fn update(&mut self, _dt: f64) {}

    fn set_actor(&mut self, actor: Option<Arc<Actor>>) {
        self.cells.clear();
        if let Some(actor) = actor {
            self.cells.extend(actor.opaque().filter_map(|(x, y, _)| {
                let (x, y) = (f64::from(x), f64::from(y));
                (x >= self.left
                    && x < self.left + self.width
                    && y >= self.top
                    && y < self.top + self.height)
                    .then_some((x, y))
            }));
        }
    }

    fn draw(&self, p: &mut Painter, _t: f64, layer: Layer) {
        if layer != Layer::Front {
            return;
        }
        for &(x, y) in &self.cells {
            p.paint_cell(x, y, self.color, self.opacity);
        }
    }
}
