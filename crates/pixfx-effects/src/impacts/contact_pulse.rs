//! A brief hot spot and impact cues at the point where a sprite was struck.

use std::sync::Arc;

use pixfx_core::cue::{FEEL, TimedCue};
use pixfx_core::{Actor, Effect, EffectState, Layer, Meta, Options};
use pixfx_draw::Painter;
use pixfx_draw::ruler::{RULER, Ruler};

use crate::mask_heat::{CONTACT_X, CONTACT_Y, IMPACT_AT, MaskHeat, TINT, contact_cues};

const SECS: f64 = 0.34;

pub struct ContactPulse;
pub static CONTACT_PULSE: ContactPulse = ContactPulse;

const META: Meta = Meta {
    id: "impacts/contact-pulse",
    name: "Contact Pulse",
    desc: "A brief flash on the sprite at the hit point, with hit-stop, shake and a ripple.",
    tags: &["impacts", "contact", "one-shot"],
    size: (128, 128),
    origin: (64, 64),
    duration: SECS,
    looping: false,
    fps: 60,
    seed: 1,
    state: None,
    options: &[FEEL.spec(), CONTACT_X.spec(), CONTACT_Y.spec(), TINT.spec()],
};

impl Effect for ContactPulse {
    fn meta(&self) -> &Meta {
        &META
    }

    fn wraps_actor(&self) -> bool {
        true
    }

    fn cues(&self, opts: &Options) -> Vec<TimedCue> {
        contact_cues(opts)
    }

    fn spawn(&self, opts: &Options, _: u64) -> Box<dyn EffectState> {
        Box::new(State {
            heat: MaskHeat::new(opts),
            radius: Ruler::size(RULER.head, 1.2),
        })
    }
}

struct State {
    heat: MaskHeat,
    radius: u32,
}

impl EffectState for State {
    fn set_actor(&mut self, actor: Option<Arc<Actor>>) {
        self.heat.read(actor.as_deref(), Some(self.radius));
    }

    fn draw(&self, p: &mut Painter, t: f64, layer: Layer) {
        if layer == Layer::Front {
            let travel = (t / IMPACT_AT).clamp(0.0, 1.0);
            let fade = (1.0 - (t - IMPACT_AT).max(0.0) / (SECS - IMPACT_AT)).max(0.0);
            self.heat
                .draw_front(p, f64::from(self.radius) * travel, fade);
        }
    }
}

#[cfg(all(test, feature = "feedback-mask-ignition"))]
mod tests {
    use super::*;
    use pixfx_core::OptValue;

    #[test]
    fn pulse_and_ignition_emit_the_same_contact_cues() {
        let mut opts = Options::default();
        opts.extra.insert("contactX".into(), OptValue::Num(12.0));
        assert_eq!(
            CONTACT_PULSE.cues(&opts),
            crate::feedback::MASK_IGNITION.cues(&opts),
        );
        assert!(!CONTACT_PULSE.cues(&opts).is_empty());
        opts.extra.insert("feel".into(), OptValue::Num(0.0));
        assert!(CONTACT_PULSE.cues(&opts).is_empty());
    }
}
