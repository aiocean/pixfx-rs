//! Small hits brighten a mark on the target; a full charge releases a pulse from its current position.

use super::wave::{CONTACT_X, CONTACT_Y, REACH, Wave, center, nearest, tint};
use pixfx_core::cue::{FEEL, TimedCue, impact};
use pixfx_core::{Actor, ColorOpt, Effect, EffectState, Layer, Meta, NumOpt, Options};
use pixfx_draw::Painter;
use pixfx_draw::color::{Rgba, WHITE};
use std::sync::Arc;
const CHARGE: NumOpt = NumOpt::new("charge", 1.0).within(0.0, 1.0);
const TINT: ColorOpt = ColorOpt::new("color", Rgba::hex24(0x80b5ff));
const RELEASE: f64 = 5.0 / 60.0;
const SECS: f64 = 0.46;
pub struct ChargedPulse;
pub static CHARGED_PULSE: ChargedPulse = ChargedPulse;
const META: Meta = Meta {
    id: "impacts/charged-pulse",
    name: "Charged Pulse",
    desc: "Small hits brighten a mark on the target; a full charge releases a pulse from its current position.",
    tags: &["impacts", "pulse"],
    size: (152, 152),
    origin: (76, 76),
    duration: SECS,
    looping: false,
    fps: 60,
    seed: 1,
    state: None,
    options: &[
        FEEL.spec(),
        CONTACT_X.spec(),
        CONTACT_Y.spec(),
        REACH.spec(),
        TINT.spec(),
        CHARGE.spec(),
    ],
};
impl Effect for ChargedPulse {
    fn meta(&self) -> &Meta {
        &META
    }
    fn wraps_actor(&self) -> bool {
        true
    }
    fn cues(&self, opts: &Options) -> Vec<TimedCue> {
        if opts.num(&CHARGE) >= 1.0 {
            // The outward pulse follows the current world position of this instance.
            impact(RELEASE, 0.66, 0.0, 0.0, opts)
        } else {
            Vec::new()
        }
    }
    fn spawn(&self, opts: &Options, _: u64) -> Box<dyn EffectState> {
        Box::new(State {
            charge: opts.num(&CHARGE),
            requested: (opts.num(&CONTACT_X), opts.num(&CONTACT_Y)),
            origin: None,
            mark: None,
            reach: opts.num(&REACH),
            tint: tint(opts, &TINT),
        })
    }
}
struct State {
    charge: f64,
    requested: (f64, f64),
    origin: Option<(f64, f64)>,
    mark: Option<(f64, f64)>,
    reach: f64,
    tint: Rgba,
}
impl EffectState for State {
    fn set_actor(&mut self, actor: Option<Arc<Actor>>) {
        self.origin = actor.as_deref().and_then(center);
        self.mark = actor.as_deref().and_then(|a| nearest(a, self.requested));
    }
    fn draw(&self, p: &mut Painter, t: f64, layer: Layer) {
        if layer != Layer::Front {
            return;
        }
        let (Some((cx, cy)), Some((mx, my))) = (self.origin, self.mark) else {
            return;
        };
        let mark = (1.0 - t / SECS).max(0.0) * self.charge;
        p.glow(mx, my, 3.0 + 5.0 * self.charge, self.tint, mark);
        p.dot(mx, my, WHITE, mark);
        if self.charge >= 1.0 && t >= RELEASE {
            let age = (t - RELEASE) / (SECS - RELEASE);
            Wave::full((cx, cy), 5.0 + self.reach * age, self.tint).draw(p, 1.0 - age);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pixfx_core::{Cue, OptValue};

    #[test]
    fn only_the_full_charge_releases_a_shockwave() {
        let mut opts = Options::default();
        opts.extra.insert("charge".into(), OptValue::Num(0.75));
        assert!(CHARGED_PULSE.cues(&opts).is_empty());
        opts.extra.insert("charge".into(), OptValue::Num(1.0));
        let cues = CHARGED_PULSE.cues(&opts);
        assert!(
            cues.iter()
                .any(|cue| cue.t == RELEASE && matches!(cue.cue, Cue::Ripple { .. }))
        );
    }
}
