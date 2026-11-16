//! Held sword: one spirit sword hanging still, pointing along +x, glinting as it takes aim: what stands where a
//! sword of `arcane/sword-formation` was while it waits its turn to leave (the volley's `held` piece), turned by
//! the scene from the way it pointed in the ring to the way it will fly. Drawn exactly as the ring draws a sword, so
//! the handover shows no seam. Made here; its golden hashes pin it.

use pixfx_core::{Effect, EffectState, Layer, Meta, Options};
use pixfx_draw::Painter;

use super::sword::{GLOW, GUARD as PIVOT, ICE, Pose, bayer, sword};

pub struct HeldSword;

pub static HELD_SWORD: HeldSword = HeldSword;

const META: Meta = Meta {
    id: "arcane/held-sword",
    name: "Held sword",
    desc: "One spirit sword hanging still, glinting as it takes aim: a flying sword waiting to leave.",
    tags: &["arcane", "projectile", "swords", "cultivation", "loop"],
    size: (40, 40),
    origin: (24, 20),
    duration: LOOP,
    looping: true,
    fps: 30,
    seed: 1,
    state: None,
    options: &[],
};

/// Eight frames a glint: one runs down the blade and back.
const LOOP: f64 = 8.0 / 30.0;

impl Effect for HeldSword {
    fn meta(&self) -> &Meta {
        &META
    }
    fn spawn(&self, _opts: &Options, _seed: u64) -> Box<dyn EffectState> {
        Box::new(Held)
    }
}

struct Held;

impl EffectState for Held {
    fn draw(&self, p: &mut Painter, t: f64, layer: Layer) {
        if layer == Layer::Back {
            return;
        }
        let frame = libm::floor(t * 30.0 + 1e-6);
        // the glint: hot from a little, up to a third, and down again
        let k = 1.0 - libm::fabs(frame / 4.0 - 1.0);
        sword(
            p,
            &Pose {
                x: PIVOT,
                y: 0.0,
                angle: 0.0,
                grown: 1.0,
                hot: 0.1 + 0.25 * k,
                alpha: 1.0,
            },
        );
        // a few motes of power drawn in to the guard, a pixel a frame
        for i in 0..4 {
            let f = f64::from(i);
            let r = 6.0 - (frame + f * 2.0) % 6.0;
            let side = if i % 2 == 0 { -1.0 } else { 1.0 };
            let (x, y) = (PIVOT - r * 0.5, side * r);
            if bayer(x + frame, y) < 0.6 {
                p.dot(x, y, ICE, 1.0);
            }
        }
        p.glow(PIVOT + 4.0, 0.0, 7.0, GLOW, 0.2 + 0.2 * k);
    }
}
