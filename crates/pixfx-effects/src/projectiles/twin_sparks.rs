//! Two opposed filaments with a travelling flash.

use super::flight::HEADING;
use super::ribbon::{COLOR, Ribbon, SPREAD, Style};
use pixfx_core::{Effect, EffectState, Meta, Options};

pub struct TwinSparks;
pub static TWIN_SPARKS: TwinSparks = TwinSparks;

const PERIOD: f64 = 0.4;
const META: Meta = Meta {
    id: "projectiles/twin-sparks",
    name: "Twin Sparks",
    desc: "Two tight filaments cross around a sprite as a white spark runs back through them.",
    tags: &["projectiles", "skill", "electric", "loop"],
    size: (160, 160),
    origin: (80, 80),
    duration: PERIOD,
    looping: true,
    fps: 50,
    seed: 1,
    state: None,
    options: &[HEADING.spec(), SPREAD.spec(), COLOR.spec()],
};

impl Effect for TwinSparks {
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
                strands: 2,
                turns: 1.44,
                radius: 0.98,
                depth: 0.23,
                tail: 0.26,
                tip: 0.0,
                color: COLOR,
                pulse: 1.0,
            },
        ))
    }
}
