//! Cracks propagate in the sprite from the point hit. A broken shield releases its own painted fragments.

use std::sync::Arc;

use pixfx_core::cue::{FEEL, TimedCue};
use pixfx_core::{Actor, Effect, EffectState, Layer, Meta, NumOpt, Options};
use pixfx_draw::Painter;

use super::lib::{CONTACT_X, CONTACT_Y, FRACTURE_SECS, Fracture, fracture_cues};

const BREAK: NumOpt = NumOpt::new("shatter", 0.0).within(0.0, 1.0);

pub struct ShieldCrack;
pub static SHIELD_CRACK: ShieldCrack = ShieldCrack;

const META: Meta = Meta {
    id: "feedback/shield-crack",
    name: "Shield Crack",
    desc: "Fractures spread across the shield from the contact point. In shatter mode its painted fragments fly apart.",
    tags: &["feedback", "shield", "contact"],
    size: (160, 160),
    origin: (80, 80),
    duration: FRACTURE_SECS,
    looping: false,
    fps: 60,
    seed: 1,
    state: None,
    options: &[
        FEEL.spec(),
        CONTACT_X.spec(),
        CONTACT_Y.spec(),
        BREAK.spec(),
    ],
};
impl Effect for ShieldCrack {
    fn meta(&self) -> &Meta {
        &META
    }
    fn wraps_actor(&self) -> bool {
        true
    }
    fn cues(&self, opts: &Options) -> Vec<TimedCue> {
        fracture_cues(opts, opts.num(&BREAK) >= 0.5)
    }

    fn spawn(&self, opts: &Options, seed: u64) -> Box<dyn EffectState> {
        Box::new(State {
            fracture: Fracture::new(opts, seed),
            shatter: opts.num(&BREAK) >= 0.5,
        })
    }
}

struct State {
    fracture: Fracture,
    shatter: bool,
}

impl EffectState for State {
    fn update(&mut self, _: f64) {}
    fn set_actor(&mut self, actor: Option<Arc<Actor>>) {
        self.fracture.set_actor(actor.as_deref());
    }
    fn draw(&self, p: &mut Painter, t: f64, layer: Layer) {
        if layer != Layer::Front {
            return;
        }
        if self.shatter {
            self.fracture.draw_fragments(p, t);
        } else {
            self.fracture.draw_cracks(p, t);
        }
    }
}
