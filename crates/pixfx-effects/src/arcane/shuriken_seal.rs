//! Shuriken seal: four comets fall in from the corners and swell into a spiked shuriken square,
//! which breaks into four hooked blobs that turn and fade to petals and crescents. Made here, by hand, from "Free pack, Part 1/15.png" (no source
//! code, so no parity check): its nine rows are the nine [`crate::tone::RAMPS`].

use pixfx_core::{Effect, EffectState, Layer, Meta, Options};
use pixfx_draw::Painter;
use pixfx_draw::color::Rgba;

use super::shuriken::{self, MOMENT, MOMENTS};
use crate::tone::{self, VARIANT};

pub struct ShurikenSeal;

pub static SHURIKEN_SEAL: ShurikenSeal = ShurikenSeal;

const META: Meta = Meta {
    id: "arcane/shuriken-seal",
    name: "Shuriken Seal",
    desc: "Four comets fall in, swell into a spiked shuriken square, then break into hooked blobs that turn and fade.",
    tags: &["arcane", "magic", "seal", "shuriken", "one-shot"],
    size: (96, 96),
    origin: (48, 48),
    duration: MOMENTS * MOMENT,
    looping: false,
    fps: 50,
    seed: 1,
    state: None,
    options: &[VARIANT.spec()],
};

impl Effect for ShurikenSeal {
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
        shuriken::paint(p, &self.ramp, s, 4, 0);
    }
}
