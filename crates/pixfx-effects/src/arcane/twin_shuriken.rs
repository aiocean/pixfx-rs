//! Twin shuriken: two comets slide in from left and right, swell into a spiked block that churns,
//! breaks into two hooked blobs and fades to petals and crescents. Made here, by hand, from "Free pack, Part 1/14.png" (no source
//! code, so no parity check): its nine rows are the nine [`crate::tone::RAMPS`].

use pixfx_core::{Effect, EffectState, Layer, Meta, Options};
use pixfx_draw::Painter;
use pixfx_draw::color::Rgba;

use super::shuriken::{self, MOMENT, MOMENTS};
use crate::tone::{self, VARIANT};

pub struct TwinShuriken;

pub static TWIN_SHURIKEN: TwinShuriken = TwinShuriken;

const META: Meta = Meta {
    id: "arcane/twin-shuriken",
    name: "Twin Shuriken",
    desc: "Two comets slide in from either side, swell into a spiked churning block, break into hooked blobs and fade.",
    tags: &["arcane", "magic", "comet", "shuriken", "one-shot"],
    size: (96, 96),
    origin: (48, 48),
    duration: MOMENTS * MOMENT,
    looping: false,
    fps: 50,
    seed: 1,
    state: None,
    options: &[VARIANT.spec()],
};

impl Effect for TwinShuriken {
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
        shuriken::paint(p, &self.ramp, s, 2, 1);
    }
}
