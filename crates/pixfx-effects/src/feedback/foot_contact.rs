//! A planted sole stamps dust or snow, or throws droplets from wet ground.

use std::sync::Arc;

use libm::{cos, sin};
use pixfx_core::cue::{FEEL, TimedCue, impact};
use pixfx_core::{Actor, Effect, EffectState, Layer, Meta, Options};
use pixfx_core::{Area, NumOpt, Silhouette};
use pixfx_draw::ruler::{RULER, Ruler};
use pixfx_draw::{Painter, color::Rgba};

const CONTACT_X: NumOpt = NumOpt::new("contactX", 0.0).within(-256.0, 256.0);
const CONTACT_Y: NumOpt = NumOpt::new("contactY", -1.0).within(-256.0, 256.0);
const SURFACE: NumOpt = NumOpt::new("surface", 0.0).within(0.0, 2.0);
const HEADING: NumOpt = NumOpt::new("heading", 0.0).within(0.0, 360.0);

pub struct FootContact;
pub static FOOT_CONTACT: FootContact = FootContact;

const DURATION: f64 = 0.5;

const META: Meta = Meta {
    id: "feedback/foot-contact",
    name: "Foot Contact",
    desc: "A planted sole stamps dust or snow, or throws droplets from wet ground.",
    tags: &["feedback", "character"],
    size: (128, 128),
    origin: (64, 64),
    duration: DURATION,
    looping: false,
    fps: 30,
    seed: 1,
    state: None,
    options: &[
        FEEL.spec(),
        CONTACT_X.spec(),
        CONTACT_Y.spec(),
        SURFACE.spec(),
        HEADING.spec(),
    ],
};

impl Effect for FootContact {
    fn meta(&self) -> &Meta {
        &META
    }

    fn wraps_actor(&self) -> bool {
        true
    }

    fn cues(&self, opts: &Options) -> Vec<TimedCue> {
        impact(
            1.0 / 30.0,
            0.02,
            opts.num(&CONTACT_X),
            opts.num(&CONTACT_Y),
            opts,
        )
    }

    fn spawn(&self, opts: &Options, _seed: u64) -> Box<dyn EffectState> {
        let angle = opts.num(&HEADING).to_radians();
        Box::new(State {
            contact: (opts.num(&CONTACT_X), opts.num(&CONTACT_Y)),
            surface: match opts.num(&SURFACE) {
                n if n >= 1.5 => Surface::Snow,
                n if n >= 0.5 => Surface::Wet,
                _ => Surface::Dust,
            },
            heading: (cos(angle), sin(angle)),
            sole: Vec::new(),
        })
    }
}

enum Surface {
    Dust,
    Wet,
    Snow,
}

struct State {
    contact: (f64, f64),
    surface: Surface,
    heading: (f64, f64),
    sole: Vec<(f64, f64)>,
}

impl EffectState for State {
    fn update(&mut self, _dt: f64) {}

    fn set_actor(&mut self, actor: Option<Arc<Actor>>) {
        self.sole.clear();
        if let Some(actor) = actor {
            let reach = f64::from(Ruler::size(RULER.limb, 1.5));
            let (cx, cy) = self.contact;
            let body = Silhouette::of(
                &actor,
                Area {
                    x: actor.x(),
                    y: actor.y(),
                    w: actor.w(),
                    h: actor.h(),
                },
            );
            self.sole.extend(body.edge().filter_map(|(x, y)| {
                let (px, py) = (f64::from(x), f64::from(y));
                (!body.covers(x, y + 1) && (px - cx).abs() <= reach && (py - cy).abs() <= reach)
                    .then_some((px, cy + 1.0 + (px - cx) * self.heading.1))
            }));
            self.sole.sort_by(|a, b| a.0.total_cmp(&b.0));
            self.sole.dedup();
        }
    }

    fn draw(&self, p: &mut Painter, t: f64, layer: Layer) {
        if layer != Layer::Back {
            return;
        }
        let t = t / DURATION;
        let (color, lift, stamp) = match self.surface {
            Surface::Dust => (Rgba::hex24(0xbca68c), t * 0.45, 0.65),
            Surface::Wet => (Rgba::hex24(0x80bce8), 4.0 * t * (1.0 - t), 0.35),
            Surface::Snow => (Rgba::hex24(0xd2e4f4), 1.7 * t * (1.0 - t), 0.95),
        };
        let reach = f64::from(Ruler::size(RULER.limb, 2.0));
        for (i, &(x, y)) in self.sole.iter().enumerate() {
            p.paint_cell(x, y, color, (1.0 - t) * stamp);
            let spread = if i % 2 == 0 { -1.0 } else { 1.0 };
            p.dot(
                x + reach * t * (spread - self.heading.0 * 0.45),
                y - lift * reach,
                color,
                1.0 - t,
            );
        }
    }
}
