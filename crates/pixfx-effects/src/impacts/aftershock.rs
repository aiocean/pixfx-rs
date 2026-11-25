//! The struck surface flashes, settles, then erupts in a heavier second pulse.

use super::wave::{CONTACT_X, CONTACT_Y, REACH, Wave, contact_canvas, nearest, tint};
use crate::mask_heat::MaskHeat;
use pixfx_core::cue::{FEEL, TimedCue, impact};
use pixfx_core::{Actor, ColorOpt, Effect, EffectState, Layer, Meta, NumOpt, Options};
use pixfx_draw::Painter;
use pixfx_draw::color::Rgba;
use std::sync::Arc;
const DELAY: NumOpt = NumOpt::new("delay", 0.32).within(0.20, 0.48);
const TINT: ColorOpt = ColorOpt::new("color", Rgba::hex24(0xfa995d));
const SECS: f64 = 0.78;
pub struct Aftershock;
pub static AFTERSHOCK: Aftershock = Aftershock;
const META: Meta = Meta {
    id: "impacts/aftershock",
    name: "Delayed Aftershock",
    desc: "The struck surface flashes, settles, then erupts in a heavier second pulse.",
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
        DELAY.spec(),
    ],
};
impl Effect for Aftershock {
    fn meta(&self) -> &Meta {
        &META
    }
    fn canvas(&self, opts: &Options) -> pixfx_core::Canvas {
        contact_canvas(opts)
    }

    fn wraps_actor(&self) -> bool {
        true
    }
    fn cues(&self, opts: &Options) -> Vec<TimedCue> {
        let (x, y) = (opts.num(&CONTACT_X), opts.num(&CONTACT_Y));
        let mut cues = impact(2.0 / 60.0, 0.18, x, y, opts);
        cues.extend(impact(opts.num(&DELAY), 0.74, x, y, opts));
        cues
    }
    fn spawn(&self, opts: &Options, _: u64) -> Box<dyn EffectState> {
        let requested = (opts.num(&CONTACT_X), opts.num(&CONTACT_Y));
        let tint = tint(opts, &TINT);
        Box::new(State {
            heat: MaskHeat::new_at(requested, tint),
            requested,
            hit: None,
            reach: opts.num(&REACH),
            delay: opts.num(&DELAY),
            tint,
        })
    }
}
struct State {
    heat: MaskHeat,
    requested: (f64, f64),
    hit: Option<(f64, f64)>,
    reach: f64,
    delay: f64,
    tint: Rgba,
}
impl EffectState for State {
    fn set_actor(&mut self, actor: Option<Arc<Actor>>) {
        self.hit = actor.as_deref().and_then(|a| nearest(a, self.requested));
        self.heat.read(actor.as_deref(), Some(22));
    }
    fn draw(&self, p: &mut Painter, t: f64, layer: Layer) {
        if layer != Layer::Front {
            return;
        }
        let Some(hit) = self.hit else {
            return;
        };
        let first = (1.0 - t / 0.13).max(0.0);
        if first > 0.0 {
            self.heat.draw_front(p, 11.0 * (t / 0.10).min(1.0), first);
        }
        let age = (t - self.delay) / (SECS - self.delay);
        if (0.0..1.0).contains(&age) {
            let fade = 1.0 - age;
            self.heat.draw_front(p, (5.0 + age * 26.0).min(22.0), fade);
            let mut wave = Wave::full(hit, 3.0 + self.reach * age, self.tint);
            wave.flatten = 0.58;
            wave.draw(p, fade);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pixfx_core::{Cue, OptValue};

    #[test]
    fn the_heavier_cues_wait_for_the_aftershock() {
        let mut opts = Options::default();
        opts.extra.insert("delay".into(), OptValue::Num(0.4));
        let cues = AFTERSHOCK.cues(&opts);
        assert!(
            cues.iter()
                .any(|cue| cue.t < 0.1 && matches!(cue.cue, Cue::Shake { .. }))
        );
        assert!(
            cues.iter()
                .any(|cue| cue.t == 0.4 && matches!(cue.cue, Cue::Ripple { .. }))
        );
        assert!(
            cues.iter()
                .filter(|cue| matches!(cue.cue, Cue::Hitstop { .. }))
                .all(|cue| cue.t == 0.4)
        );
    }
}
