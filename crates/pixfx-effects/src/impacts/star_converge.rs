//! Star converge: four teardrops slide in from the edges and spiral toward the centre, meet in a bright
//! star and scatter as thin curling blades that fade. Made here, by hand, from "Free pack, Part 1/24.png" (no source
//! code, so no parity check): its nine rows are the nine [`crate::tone::RAMPS`].

use pixfx_core::{Effect, EffectState, Layer, Meta, Options};
use pixfx_draw::Painter;
use pixfx_draw::color::Rgba;

use super::drops::{self, MOMENT, MOMENTS};
use crate::tone::{self, VARIANT};

pub struct StarConverge;

pub static STAR_CONVERGE: StarConverge = StarConverge;

const META: Meta = Meta {
    id: "impacts/star-converge",
    name: "Star Converge",
    desc: "Four teardrops spiral in from the edges, meet in a bright star and scatter as thin curling blades.",
    tags: &["impact", "magic", "converge", "star", "one-shot"],
    size: (96, 96),
    origin: (48, 48),
    duration: MOMENTS * MOMENT,
    looping: false,
    fps: 50,
    seed: 1,
    state: None,
    options: &[VARIANT.spec()],
};

impl Effect for StarConverge {
    fn meta(&self) -> &Meta {
        &META
    }
    fn spawn(&self, opts: &Options, _seed: u64) -> Box<dyn EffectState> {
        Box::new(State {
            ramp: tone::ramp(opts),
        })
    }
}

struct State {
    ramp: Vec<Rgba>,
}

impl EffectState for State {
    fn draw(&self, p: &mut Painter, t: f64, layer: Layer) {
        let s = t / MOMENT;
        if layer == Layer::Back || s >= MOMENTS {
            return;
        }
        drops::paint(p, &self.ramp, s, 4, 0, (0.0, 0.0));
    }
}
