//! `appear/dither`: the sprite it wraps fills in through an ordered 4x4 dither, an even checker sweeping across it,
//! each pixel landing white and settling to its colour: `disappear/dither`'s frames backward, a retro entrance
//! (with the default `angle` the feet arrive first and the head last). Its golden hashes pin it.
//!
//! It follows the sprite it is handed each frame. The game hides the sprite while it plays and shows it once it
//! ends. With no actor it builds a stand-in disc.

use std::sync::Arc;

use pixfx_core::{Actor, Canvas, Effect, EffectState, Layer, Meta, Options};
use pixfx_draw::Painter;

use crate::dither::{ANGLE, Dither, H, SECS, SPREAD, W, WHITEN, canvas};

pub struct DitherIn;

pub static DITHER: DitherIn = DitherIn;

const META: Meta = Meta {
    id: "appear/dither",
    name: "Dither",
    desc: "The sprite it wraps fills in through an ordered dither, an even checker sweeping across it, each pixel landing white and settling to its colour: a retro entrance.",
    tags: &["appear", "one-shot", "dither", "dissolve", "pixel", "retro"],
    // `canvas` sizes it from `w` and `h`; this is their default
    size: (128, 128),
    origin: (64, 64),
    duration: SECS,
    looping: false,
    fps: 30,
    seed: 1,
    state: None,
    options: &[
        ANGLE.spec(),
        SPREAD.spec(),
        WHITEN.spec(),
        W.spec(),
        H.spec(),
    ],
};

impl Effect for DitherIn {
    fn meta(&self) -> &Meta {
        &META
    }

    fn wraps_actor(&self) -> bool {
        true
    }

    fn canvas(&self, opts: &Options) -> Canvas {
        canvas(opts)
    }

    fn spawn(&self, opts: &Options, _seed: u64) -> Box<dyn EffectState> {
        Box::new(State(Dither::new(opts)))
    }
}

struct State(Dither);

impl EffectState for State {
    fn update(&mut self, _dt: f64) {}

    fn set_actor(&mut self, actor: Option<Arc<Actor>>) {
        self.0.set_actor(actor);
    }

    fn draw(&self, p: &mut Painter, t: f64, layer: Layer) {
        if layer == Layer::Front {
            // the fade-out's frames backward
            self.0.draw(p, 1.0 - t / SECS);
        }
    }
}
