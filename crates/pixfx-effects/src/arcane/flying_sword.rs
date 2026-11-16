//! Flying sword: one spirit sword in flight, pointing along +x. A projectile: the scene flies it and turns it to
//! face where it goes (`arcane/sword-formation`'s volley, which lays `projectiles/trail` under it, or a game moving
//! it itself). It opens once as it leaves (`OPEN`): the blade flashes white and specks burst off it; then it loops,
//! the blade glinting. Its streak is the trail's, so it bends with the flight. Made here from the SWORD part of
//! `pixfx-ref`'s `examples/specs/arcane/sword-formation.json`; no source, so no parity check; its golden hashes pin it.

use pixfx_core::{Effect, EffectState, Layer, Meta, Options};
use pixfx_draw::Painter;
use pixfx_draw::math::{rand, smooth};

use super::sword::{GLOW, GUARD, ICE, Pose, WHITE, bayer, sword};

pub struct FlyingSword;

pub static FLYING_SWORD: FlyingSword = FlyingSword;

const META: Meta = Meta {
    id: "arcane/flying-sword",
    name: "Flying sword",
    desc: "One spirit sword in flight, pointing along its way, flashing as it leaves: a projectile to fly and turn, under a trail.",
    tags: &["arcane", "projectile", "swords", "cultivation", "loop"],
    size: (40, 36),
    origin: (24, 18),
    duration: OPEN + LOOP,
    looping: true,
    fps: 30,
    seed: 1,
    state: None,
    options: &[],
};

/// The sword's pivot (its guard), behind the origin.
pub(super) const PIVOT: f64 = GUARD;
/// Seconds: the opening (leaving), then the loop.
const OPEN: f64 = 6.0 / 30.0;
const LOOP: f64 = 8.0 / 30.0;

impl Effect for FlyingSword {
    fn meta(&self) -> &Meta {
        &META
    }
    fn spawn(&self, _opts: &Options, _seed: u64) -> Box<dyn EffectState> {
        Box::new(Flying)
    }
    fn loop_from(&self) -> Option<f64> {
        Some(OPEN)
    }
}

struct Flying;

impl EffectState for Flying {
    fn draw(&self, p: &mut Painter, t: f64, layer: Layer) {
        if layer == Layer::Back {
            return;
        }
        let shown = smooth(0.0, OPEN, t);
        if t < OPEN {
            burst(p, t / OPEN);
        }
        sword(
            p,
            &Pose {
                x: PIVOT,
                y: 0.0,
                angle: 0.0,
                grown: 1.0,
                hot: 0.3 + 0.4 * (1.0 - shown),
                alpha: 1.0,
            },
        );
    }
}

/// Leaving: specks of light thrown off the blade, out and back, fading over the opening (`k` 0 to 1).
fn burst(p: &mut Painter, k: f64) {
    for i in 0..6 {
        let f = f64::from(i);
        let side = if i % 2 == 0 { -1.0 } else { 1.0 };
        let along = PIVOT + 6.0 - rand(&[f, 1.0]) * 12.0;
        let out = 2.0 + k * (4.0 + rand(&[f, 2.0]) * 5.0);
        let (x, y) = (along - k * 6.0, side * out);
        if bayer(x, y) < 1.0 - k {
            p.dot(x, y, if k < 0.5 { WHITE } else { ICE }, 1.0);
            p.light(x, y, GLOW, 0.4 * (1.0 - k));
        }
    }
}
