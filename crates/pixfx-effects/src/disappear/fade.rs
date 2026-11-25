//! `disappear/fade`: the sprite it wraps goes pixel by pixel, its edges first and its heart last; each pixel stays
//! solid, lights toward white and is gone, at its own seeded moment: a soft exit for a spell that has spent itself,
//! where `explosions/body-burst` blows a body apart. `appear/fade` is the same frames backward. Its golden hashes pin
//! it.
//!
//! It follows the sprite it is handed each frame (a shape still spinning keeps spinning as it fades); the swell
//! (`grow`, off by default) and the fade run from when it started. It does not replace its actor: the game hides the
//! sprite when the fade starts and may show it again later. With no actor it fades a stand-in disc.

use std::sync::Arc;

use pixfx_core::{Actor, Canvas, Effect, EffectState, Layer, Meta, Options};
use pixfx_draw::Painter;

use crate::dissolve::{Dissolve, GROW, H, SECS, W, WHITEN, canvas};

pub struct Fade;

pub static FADE: Fade = Fade;

const META: Meta = Meta {
    id: "disappear/fade",
    name: "Fade",
    desc: "The sprite it wraps goes pixel by pixel, its edges first and its heart last, each pixel lighting toward white just before it is gone: a soft exit, nothing broken.",
    tags: &["disappear", "one-shot", "fade", "dissolve", "magic"],
    // `canvas` sizes it from `w` and `h`; this is their default
    size: (128, 128),
    origin: (64, 64),
    duration: SECS,
    looping: false,
    fps: 30,
    seed: 1,
    state: None,
    options: &[GROW.spec(), WHITEN.spec(), W.spec(), H.spec()],
};

impl Effect for Fade {
    fn meta(&self) -> &Meta {
        &META
    }

    fn wraps_actor(&self) -> bool {
        true
    }

    fn canvas(&self, opts: &Options) -> Canvas {
        canvas(opts)
    }

    fn spawn(&self, opts: &Options, seed: u64) -> Box<dyn EffectState> {
        Box::new(State(Dissolve::new(opts, seed)))
    }
}

struct State(Dissolve);

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
