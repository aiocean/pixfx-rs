//! A low shockwave opens in the direction of the strike and leaves the rear clear.

use super::wave::{CONTACT_X, CONTACT_Y, REACH, Wave, contact_canvas, nearest, tint};
use pixfx_core::cue::{FEEL, TimedCue, impact};
use pixfx_core::{Actor, ColorOpt, Effect, EffectState, Layer, Meta, NumOpt, Options};
use pixfx_draw::Painter;
use pixfx_draw::color::{Rgba, WHITE};
use std::sync::Arc;
const HEADING: NumOpt = NumOpt::new("heading", 0.0).within(-180.0, 180.0);
const SPREAD: NumOpt = NumOpt::new("spread", 96.0).within(20.0, 170.0);
const TINT: ColorOpt = ColorOpt::new("color", Rgba::hex24(0xffa482));
const SECS: f64 = 0.58;
pub struct DirectionalWave;
pub static DIRECTIONAL_WAVE: DirectionalWave = DirectionalWave;
const META: Meta = Meta {
    id: "impacts/directional-wave",
    name: "Directional Wave",
    desc: "A low shockwave opens in the direction of the strike and leaves the rear clear.",
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
        HEADING.spec(),
        SPREAD.spec(),
    ],
};
impl Effect for DirectionalWave {
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
        impact(
            1.0 / 60.0,
            0.48,
            opts.num(&CONTACT_X),
            opts.num(&CONTACT_Y),
            opts,
        )
    }
    fn spawn(&self, opts: &Options, _: u64) -> Box<dyn EffectState> {
        Box::new(State {
            requested: (opts.num(&CONTACT_X), opts.num(&CONTACT_Y)),
            hit: None,
            reach: opts.num(&REACH),
            heading: opts.num(&HEADING).to_radians(),
            half_angle: (opts.num(&SPREAD) * 0.5).to_radians(),
            tint: tint(opts, &TINT),
        })
    }
}
struct State {
    requested: (f64, f64),
    hit: Option<(f64, f64)>,
    reach: f64,
    heading: f64,
    half_angle: f64,
    tint: Rgba,
}
impl EffectState for State {
    fn set_actor(&mut self, actor: Option<Arc<Actor>>) {
        self.hit = actor.as_deref().and_then(|a| nearest(a, self.requested));
    }
    fn draw(&self, p: &mut Painter, t: f64, layer: Layer) {
        if layer != Layer::Front {
            return;
        }
        let Some(hit) = self.hit else {
            return;
        };
        let age = (t / 0.44).min(1.0);
        let fade = (1.0 - t / SECS).max(0.0);
        let radius = 4.0 + self.reach * age;
        let wave = Wave {
            origin: hit,
            radius,
            heading: self.heading,
            half_angle: self.half_angle,
            flatten: 1.0,
            color: self.tint,
        };
        wave.draw(p, fade);
        if radius > 9.0 {
            Wave {
                radius: radius - 7.0,
                ..wave
            }
            .draw(p, fade * 0.42);
        }
        p.glow(hit.0, hit.1, 6.0, WHITE, (1.0 - t / 0.12).max(0.0));
    }
}
