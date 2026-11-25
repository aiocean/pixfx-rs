//! A tight flash spreads from a contact point across the sprite.

use std::sync::Arc;

use libm::hypot;
use pixfx_core::cue::{FEEL, TimedCue, impact};
use pixfx_core::{Actor, Effect, EffectState, Layer, Meta, Options};
use pixfx_core::{ColorOpt, NumOpt};
use pixfx_draw::ruler::{RULER, Ruler};
use pixfx_draw::{Painter, color::Rgba};

const CONTACT_X: NumOpt = NumOpt::new("contactX", -7.0).within(-256.0, 256.0);
const CONTACT_Y: NumOpt = NumOpt::new("contactY", -28.0).within(-256.0, 256.0);
const COLOR: ColorOpt = ColorOpt::new("color", Rgba::hex24(0xffedde));

pub struct HitFlash;
pub static HIT_FLASH: HitFlash = HitFlash;

const DURATION: f64 = 0.2;

const META: Meta = Meta {
    id: "feedback/hit-flash",
    name: "Hit Flash",
    desc: "A tight flash spreads from a contact point across the sprite.",
    tags: &["feedback", "character"],
    size: (128, 128),
    origin: (64, 64),
    duration: DURATION,
    looping: false,
    fps: 60,
    seed: 1,
    state: None,
    options: &[
        FEEL.spec(),
        CONTACT_X.spec(),
        CONTACT_Y.spec(),
        COLOR.spec(),
    ],
};

impl Effect for HitFlash {
    fn meta(&self) -> &Meta {
        &META
    }

    fn wraps_actor(&self) -> bool {
        true
    }

    fn cues(&self, opts: &Options) -> Vec<TimedCue> {
        impact(
            1.0 / 60.0,
            0.34,
            opts.num(&CONTACT_X),
            opts.num(&CONTACT_Y),
            opts,
        )
    }

    fn spawn(&self, opts: &Options, _seed: u64) -> Box<dyn EffectState> {
        Box::new(State {
            contact: (opts.num(&CONTACT_X), opts.num(&CONTACT_Y)),
            color: opts.color(&COLOR),
            pixels: Vec::new(),
            reach: 0.0,
        })
    }
}

struct State {
    contact: (f64, f64),
    color: Rgba,
    pixels: Vec<(f64, f64, f64)>,
    reach: f64,
}

impl EffectState for State {
    fn update(&mut self, _dt: f64) {}

    fn set_actor(&mut self, actor: Option<Arc<Actor>>) {
        self.pixels.clear();
        if let Some(actor) = actor {
            self.pixels.extend(actor.opaque().map(|(x, y, _)| {
                let (x, y) = (f64::from(x), f64::from(y));
                (x, y, hypot(x - self.contact.0, y - self.contact.1))
            }));
        }
        self.reach = self
            .pixels
            .iter()
            .map(|&(_, _, distance)| distance)
            .fold(0.0, f64::max);
    }

    fn draw(&self, p: &mut Painter, t: f64, layer: Layer) {
        if layer != Layer::Front {
            return;
        }
        let width = f64::from(Ruler::size(RULER.head, 1.0));
        let t = t / DURATION;
        let front = t * (self.reach + width);
        for &(x, y, distance) in &self.pixels {
            let pulse = (1.0 - (distance - front).abs() / width).max(0.0) * (1.0 - t);
            p.each_game_cell(x, y, |p, sx, sy| {
                p.put(sx, sy, self.color, pulse);
                p.add(sx, sy, self.color, pulse * 0.32);
            });
        }
    }
}
