//! A sprite cracks and flies apart in fragments that keep its painted pixels.

use std::sync::Arc;

use pixfx_core::cue::{FEEL, TimedCue};
use pixfx_core::{Actor, Effect, EffectState, Layer, Meta, Options};
use pixfx_draw::Painter;

use super::lib::{CONTACT_X, CONTACT_Y, FRACTURE_SECS, Fracture, fracture_cues};

pub struct SpriteShatter;
pub static SPRITE_SHATTER: SpriteShatter = SpriteShatter;

const META: Meta = Meta {
    id: "feedback/sprite-shatter",
    name: "Sprite Shatter",
    desc: "A fracture spreads from the contact point and the sprite breaks into fragments in its own colours.",
    tags: &["feedback", "shatter", "contact"],
    size: (160, 160),
    origin: (80, 80),
    duration: FRACTURE_SECS,
    looping: false,
    fps: 60,
    seed: 1,
    state: None,
    options: &[FEEL.spec(), CONTACT_X.spec(), CONTACT_Y.spec()],
};

impl Effect for SpriteShatter {
    fn meta(&self) -> &Meta {
        &META
    }

    fn wraps_actor(&self) -> bool {
        true
    }

    fn replaces_actor(&self) -> bool {
        true
    }

    fn cues(&self, opts: &Options) -> Vec<TimedCue> {
        fracture_cues(opts, true)
    }

    fn spawn(&self, opts: &Options, seed: u64) -> Box<dyn EffectState> {
        Box::new(State(Fracture::new(opts, seed)))
    }
}

struct State(Fracture);

impl EffectState for State {
    fn update(&mut self, _: f64) {}

    fn set_actor(&mut self, actor: Option<Arc<Actor>>) {
        self.0.set_actor(actor.as_deref());
    }

    fn draw(&self, p: &mut Painter, t: f64, layer: Layer) {
        if layer == Layer::Front {
            self.0.draw_fragments(p, t);
        }
    }
}
