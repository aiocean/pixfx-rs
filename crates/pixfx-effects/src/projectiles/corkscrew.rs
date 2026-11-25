//! An open ribbon winds around the projectile and unfurls behind it.

use super::flight::HEADING;
use super::ribbon::{COLOR, Ribbon, SPREAD, Style};
use pixfx_core::{Effect, EffectState, Meta, Options};

pub struct Corkscrew;
pub static CORKSCREW: Corkscrew = Corkscrew;

const PERIOD: f64 = 0.56;
const META: Meta = Meta {
    id: "projectiles/corkscrew",
    name: "Corkscrew",
    desc: "A broad, open spiral curls behind and in front of a moving sprite.",
    tags: &["projectiles", "skill", "spiral", "loop"],
    size: (160, 160),
    origin: (80, 80),
    duration: PERIOD,
    looping: true,
    fps: 50,
    seed: 1,
    state: None,
    options: &[HEADING.spec(), SPREAD.spec(), COLOR.spec()],
};

impl Effect for Corkscrew {
    fn meta(&self) -> &Meta {
        &META
    }

    fn wraps_actor(&self) -> bool {
        true
    }

    fn spawn(&self, opts: &Options, _: u64) -> Box<dyn EffectState> {
        Box::new(Ribbon::new(
            opts,
            Style {
                period: PERIOD,
                strands: 1,
                turns: 1.75,
                radius: 1.72,
                depth: 0.73,
                tail: 0.55,
                tip: 0.0,
                color: COLOR,
                pulse: 0.0,
            },
        ))
    }
}
