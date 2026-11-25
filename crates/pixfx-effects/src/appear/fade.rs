//! `appear/fade`: the sprite it wraps arrives pixel by pixel, its heart first and its edges last; each pixel lands
//! white and settles to its colour, at its own seeded moment: `disappear/fade`'s frames backward, a soft entrance.
//! Its golden hashes pin it.
//!
//! It follows the sprite it is handed each frame; with `grow` it starts swollen and settles to size. The game hides
//! the sprite while it plays and shows it once it ends. With no actor it builds a stand-in disc.

use std::sync::Arc;

use pixfx_core::{Actor, Canvas, Effect, EffectState, Layer, Meta, Options};
use pixfx_draw::Painter;

use crate::dissolve::{Dissolve, GROW, H, SECS, W, WHITEN, canvas};

pub struct Fade;

pub static FADE: Fade = Fade;

const META: Meta = Meta {
    id: "appear/fade",
    name: "Fade",
    desc: "The sprite it wraps arrives pixel by pixel, its heart first and its edges last, each pixel landing white and settling to its colour: a soft entrance.",
    tags: &["appear", "one-shot", "fade", "dissolve", "magic"],
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
            // the fade-out's frames backward
            self.0.draw(p, 1.0 - t / SECS);
        }
    }
}
