//! Sparks leave the projectile's contact edge on the reflected side of the surface.

use std::sync::Arc;

use libm::{cos, hypot, sin};
use pixfx_core::cue::{FEEL, TimedCue, impact};
use pixfx_core::{Actor, Effect, EffectState, Layer, Meta, NumOpt, Options};
use pixfx_draw::{
    Painter, Rgba,
    color::{WHITE, mix},
    ruler::{RULER, Ruler},
};

const NORMAL: NumOpt = NumOpt::new("normalAngle", 180.0).within(0.0, 360.0);
const INCOMING: NumOpt = NumOpt::new("incomingAngle", 20.0).within(0.0, 360.0);
const CONTACT_X: NumOpt = NumOpt::new("contactX", 0.0).within(-64.0, 64.0);
const CONTACT_Y: NumOpt = NumOpt::new("contactY", 0.0).within(-64.0, 64.0);
const SECS: f64 = 0.32;

pub struct Ricochet;
pub static RICOCHET: Ricochet = Ricochet;

const META: Meta = Meta {
    id: "feedback/ricochet",
    name: "Ricochet Sparks",
    desc: "Sparks pick up the projectile's colours and fan out from its contact edge along its reflected direction.",
    tags: &["feedback", "projectile", "contact"],
    size: (128, 128),
    origin: (64, 64),
    duration: SECS,
    looping: false,
    fps: 60,
    seed: 1,
    state: None,
    options: &[
        FEEL.spec(),
        NORMAL.spec(),
        INCOMING.spec(),
        CONTACT_X.spec(),
        CONTACT_Y.spec(),
    ],
};

impl Effect for Ricochet {
    fn meta(&self) -> &Meta {
        &META
    }
    fn wraps_actor(&self) -> bool {
        true
    }
    fn cues(&self, opts: &Options) -> Vec<TimedCue> {
        let separation = (opts.num(&INCOMING) - opts.num(&NORMAL)).to_radians();
        let incidence = (-cos(separation)).max(0.0);
        impact(
            1.0 / 60.0,
            0.1 + 0.23 * incidence,
            opts.num(&CONTACT_X),
            opts.num(&CONTACT_Y),
            opts,
        )
    }

    fn spawn(&self, opts: &Options, _: u64) -> Box<dyn EffectState> {
        let angle = opts.num(&NORMAL).to_radians();
        let normal = (cos(angle), sin(angle));
        let angle = opts.num(&INCOMING).to_radians();
        let incoming = (cos(angle), sin(angle));
        let incidence = (incoming.0 * normal.0 + incoming.1 * normal.1).min(0.0);
        Box::new(State {
            contact: (opts.num(&CONTACT_X), opts.num(&CONTACT_Y)),
            normal,
            reflected: (
                incoming.0 - 2.0 * incidence * normal.0,
                incoming.1 - 2.0 * incidence * normal.1,
            ),
            samples: Vec::new(),
        })
    }
}

struct State {
    contact: (f64, f64),
    normal: (f64, f64),
    reflected: (f64, f64),
    samples: Vec<(f64, f64, Rgba)>,
}

impl EffectState for State {
    fn update(&mut self, _: f64) {}
    fn set_actor(&mut self, actor: Option<Arc<Actor>>) {
        self.samples.clear();
        if let Some(actor) = actor {
            let distance = |x: f64, y: f64| hypot(x - self.contact.0, y - self.contact.1);
            self.samples.extend(
                actor
                    .opaque()
                    .map(|(x, y, color)| (f64::from(x), f64::from(y), color)),
            );
            self.samples
                .sort_by(|a, b| distance(a.0, a.1).total_cmp(&distance(b.0, b.1)));
            self.samples.truncate(12);
        }
    }
    fn draw(&self, p: &mut Painter, t: f64, layer: Layer) {
        if layer != Layer::Front {
            return;
        }
        let u = t / SECS;
        let reach = f64::from(Ruler::size(RULER.blade, 1.6));
        let (nx, ny) = self.normal;
        let (rx, ry) = self.reflected;
        for (i, &(x, y, color)) in self.samples.iter().enumerate() {
            let speed = reach * (0.5 + (i % 5) as f64 * 0.15);
            let spread = ((i * 7 % 11) as f64 - 5.0) * 0.06;
            let (mut dx, mut dy) = (rx - ry * spread, ry + rx * spread);
            let inward = (dx * nx + dy * ny).min(0.0);
            dx -= inward * nx;
            dy -= inward * ny;
            for segment in 0..3 {
                let age = (u - f64::from(segment) * 0.045).max(0.0);
                let (px, py) = (x + dx * speed * age, y + dy * speed * age);
                let tint = mix(color, WHITE, (1.0 - u) * (1.0 - 0.25 * f64::from(segment)));
                p.dot(px, py, tint, 1.0 - u);
                p.light(px, py, tint, (1.0 - u) * 0.55);
            }
        }
    }
}
