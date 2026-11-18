//! `disappear/dither`: the sprite it wraps thins away through an ordered 4x4 dither, an even checker sweeping across
//! it from one side (`angle`, the top by default) to the other, each pixel lighting toward white as it goes: a retro
//! exit, where `disappear/fade` crumbles in random grain. `appear/dither` is the same frames backward. Its golden
//! hashes pin it.
//!
//! It follows the sprite it is handed each frame. It does not replace its actor: the game hides the sprite when the
//! dissolve starts and may show it again later. With no actor it dissolves a stand-in disc.

use std::sync::Arc;

use pixfx_core::{Actor, Canvas, Effect, EffectState, Layer, Meta, Options};
use pixfx_draw::Painter;

use crate::dither::{ANGLE, Dither, H, SECS, SPREAD, W, WHITEN, canvas};

pub struct DitherOut;

pub static DITHER: DitherOut = DitherOut;

const META: Meta = Meta {
    id: "disappear/dither",
    name: "Dither",
    desc: "The sprite it wraps thins away through an ordered dither, an even checker sweeping across it from one side to the other: a retro exit.",
    tags: &[
        "disappear",
        "one-shot",
        "dither",
        "dissolve",
        "pixel",
        "retro",
    ],
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

impl Effect for DitherOut {
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
            self.0.draw(p, t / SECS);
        }
    }
}
