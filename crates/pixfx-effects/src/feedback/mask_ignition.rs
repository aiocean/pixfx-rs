//! A flame front spreads from the contact point through the connected pixels of a supplied sprite.

use std::sync::Arc;

use pixfx_core::cue::{FEEL, TimedCue};
use pixfx_core::{Actor, Effect, EffectState, Layer, Meta, Options};
use pixfx_draw::Painter;

use crate::mask_heat::{CONTACT_X, CONTACT_Y, MaskHeat, TINT, contact_cues};

const SECS: f64 = 0.86;
pub struct MaskIgnition;
pub static MASK_IGNITION: MaskIgnition = MaskIgnition;
const META: Meta = Meta {
    id: "feedback/mask-ignition",
    name: "Mask Ignition",
    desc: "A flame front spreads from the contact point through the connected pixels of a supplied sprite.",
    tags: &["feedback", "contact"],
    size: (128, 128),
    origin: (64, 64),
    duration: SECS,
    looping: false,
    fps: 60,
    seed: 1,
    state: None,
    options: &[FEEL.spec(), CONTACT_X.spec(), CONTACT_Y.spec(), TINT.spec()],
};
impl Effect for MaskIgnition {
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
        Box::new(State(MaskHeat::new(opts)))
    }
}

struct State(MaskHeat);

impl EffectState for State {
    fn set_actor(&mut self, actor: Option<Arc<Actor>>) {
        self.0.read(actor.as_deref(), None);
    }

    fn draw(&self, p: &mut Painter, t: f64, layer: Layer) {
        if layer == Layer::Front {
            let front = t / 0.58 * (self.0.reach() + 4.0);
            let fade = (1.0 - t / SECS).clamp(0.0, 1.0);
            self.0.draw_front(p, front, fade);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pixfx_core::{OptValue, render_frames_with_actor};

    #[test]
    fn the_flame_stops_at_a_gap_in_the_mask() -> Result<(), pixfx_core::ActorError> {
        let actor = Arc::new(Actor::new(
            0,
            0,
            7,
            1,
            vec![
                0xffffffff, 0xffffffff, 0xffffffff, 0, 0xffffffff, 0xffffffff, 0xffffffff,
            ],
        )?);
        let mut opts = Options::default();
        opts.extra.insert("contactX".into(), OptValue::Num(0.0));
        let rendered =
            render_frames_with_actor(&MASK_IGNITION, &opts, 0, |_| Some(Arc::clone(&actor)));
        let density = opts.pixel_density.factor() as usize;
        let stride = rendered.w as usize;
        let origin = rendered.origin.1 as usize * stride + rendered.origin.0 as usize;
        let lit = |frame: &pixfx_draw::Frame, x: core::ops::Range<usize>| {
            (0..density).any(|dy| {
                let row = origin + dy * stride;
                frame.color[row + x.start * density..row + x.end * density]
                    .iter()
                    .any(|&pixel| pixel != 0)
            })
        };
        assert!(rendered.frames.iter().any(|f| lit(f, 0..3)));
        assert!(rendered.frames.iter().all(|f| !lit(f, 4..7)));
        Ok(())
    }
}
