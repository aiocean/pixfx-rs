//! Additive light catches the edge of a sprite facing the source.

use std::sync::Arc;

use libm::{cos, hypot, sin};
use pixfx_core::{Actor, Effect, EffectState, Layer, Meta, Options};
use pixfx_core::{Area, ColorOpt, NumOpt, Silhouette};
use pixfx_draw::{Painter, color::Rgba};

const SOURCE_ANGLE: NumOpt = NumOpt::new("sourceAngle", 335.0).within(0.0, 360.0);
const COLOR: ColorOpt = ColorOpt::new("color", Rgba::hex24(0xffbd73));
const STRENGTH: NumOpt = NumOpt::new("strength", 0.8).within(0.0, 2.0);

pub struct RimLight;
pub static RIM_LIGHT: RimLight = RimLight;

const META: Meta = Meta {
    id: "feedback/rim-light",
    name: "Rim Light",
    desc: "Additive light catches the edge of a sprite facing the source.",
    tags: &["feedback", "character", "loop"],
    size: (128, 128),
    origin: (64, 64),
    duration: 1.0,
    looping: true,
    fps: 1,
    seed: 1,
    state: None,
    options: &[SOURCE_ANGLE.spec(), COLOR.spec(), STRENGTH.spec()],
};

impl Effect for RimLight {
    fn meta(&self) -> &Meta {
        &META
    }

    fn wraps_actor(&self) -> bool {
        true
    }

    fn spawn(&self, opts: &Options, _seed: u64) -> Box<dyn EffectState> {
        let angle = opts.num(&SOURCE_ANGLE).to_radians();
        Box::new(State {
            direction: (cos(angle), sin(angle)),
            color: opts.color(&COLOR),
            strength: opts.num(&STRENGTH),
            edges: Vec::new(),
        })
    }
}

struct State {
    direction: (f64, f64),
    color: Rgba,
    strength: f64,
    edges: Vec<(f64, f64, f64)>,
}

impl EffectState for State {
    fn update(&mut self, _dt: f64) {}

    fn set_actor(&mut self, actor: Option<Arc<Actor>>) {
        self.edges.clear();
        if let Some(actor) = actor {
            let body = Silhouette::of(
                &actor,
                Area {
                    x: actor.x(),
                    y: actor.y(),
                    w: actor.w(),
                    h: actor.h(),
                },
            );
            self.edges.extend(body.edge().filter_map(|(x, y)| {
                let (mut nx, mut ny) = (0.0, 0.0);
                for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                    if !body.covers(x + dx, y + dy) {
                        nx += f64::from(dx);
                        ny += f64::from(dy);
                    }
                }
                let strength =
                    (nx * self.direction.0 + ny * self.direction.1) / hypot(nx, ny).max(1.0);
                (strength > 0.0).then_some((f64::from(x), f64::from(y), strength))
            }));
        }
    }

    fn draw(&self, p: &mut Painter, _t: f64, layer: Layer) {
        if layer != Layer::Front {
            return;
        }
        for &(x, y, incidence) in &self.edges {
            p.each_game_cell(x, y, |p, sx, sy| {
                p.add(sx, sy, self.color, incidence * self.strength);
            });
        }
    }
}
