//! Two strands braid into a bright point at the leading edge.

use super::flight::HEADING;
use super::ribbon::{Ribbon, SPREAD, Style};
use pixfx_core::{ColorOpt, Effect, EffectState, Meta, Options};
use pixfx_draw::Rgba;

pub struct AetherPierce;
pub static AETHER_PIERCE: AetherPierce = AetherPierce;

const PERIOD: f64 = 0.48;
const COLOR: ColorOpt = ColorOpt::new("color", Rgba::hex24(0x4eeafa));
const META: Meta = Meta {
    id: "projectiles/aether-pierce",
    name: "Aether Pierce",
    desc: "Two fine strands braid around a projectile and converge on a bright needle at the tip.",
    tags: &["projectiles", "skill", "braid", "loop"],
    size: (160, 160),
    origin: (80, 80),
    duration: PERIOD,
    looping: true,
    fps: 50,
    seed: 1,
    state: None,
    options: &[HEADING.spec(), SPREAD.spec(), COLOR.spec()],
};

impl Effect for AetherPierce {
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
                turns: 1.08,
                radius: 0.9,
                depth: 0.11,
                tail: 0.66,
                pulse: 0.0,
                tip: 1.0,
                color: COLOR,
            },
        ))
    }
}
