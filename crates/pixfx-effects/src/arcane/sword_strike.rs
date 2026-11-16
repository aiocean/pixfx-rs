//! Sword strike: where a flying sword strikes, a four-point star flashes and shrinks, sparks fly back the way the
//! sword came (along -x: the scene turns it to the sword's heading) and three chips of the blade fall. It fires the
//! impact cue. Made here from the FX part of `pixfx-ref`'s `examples/specs/arcane/sword-formation.json`; its
//! golden hashes pin it.

use core::f64::consts::PI;

use pixfx_core::cue::{FEEL, TimedCue, impact};
use pixfx_core::{Effect, EffectState, Layer, Meta, Options};
use pixfx_draw::Painter;
use pixfx_draw::math::rand;

use pixfx_draw::color::Rgba;

/// The sword's palette (`super::sword`), the colours a strike shows.
const WHITE: Rgba = Rgba::hex24(0xf2fdfd);
const ICE: Rgba = Rgba::hex24(0xa4f7fd);
const AQUA: Rgba = Rgba::hex24(0x63eefd);
const CYAN: Rgba = Rgba::hex24(0x56cef8);
const SKY: Rgba = Rgba::hex24(0x56b6e2);
const GLOW: Rgba = Rgba::hex24(0x44b2c1);

pub struct SwordStrike;

pub static SWORD_STRIKE: SwordStrike = SwordStrike;

const META: Meta = Meta {
    id: "arcane/sword-strike",
    name: "Sword strike",
    desc: "A flying sword's strike: a shrinking four-point star, sparks thrown back the way it came, chips of the blade falling.",
    tags: &["arcane", "impact", "swords", "cultivation", "one-shot"],
    size: (48, 48),
    origin: (24, 24),
    duration: 0.45,
    looping: false,
    fps: 30,
    seed: 1,
    state: None,
    options: &[FEEL.spec()],
};

/// How long the star and the sparks last.
const FLASH: f64 = 0.3;

impl Effect for SwordStrike {
    fn meta(&self) -> &Meta {
        &META
    }
    fn cues(&self, opts: &Options) -> Vec<TimedCue> {
        impact(0.0, 0.3, 0.0, 0.0, opts)
    }
    fn spawn(&self, _opts: &Options, seed: u64) -> Box<dyn EffectState> {
        Box::new(Strike { seed: seed as f64 })
    }
}

struct Strike {
    seed: f64,
}

impl EffectState for Strike {
    fn draw(&self, p: &mut Painter, t: f64, layer: Layer) {
        if layer == Layer::Back {
            return;
        }
        if t < FLASH {
            let k = 1.0 - t / FLASH;
            let ray = libm::floor(7.0 * k);
            p.dot(0.0, 0.0, WHITE, 1.0);
            for r in 1..=ray as i32 {
                let r = f64::from(r);
                let c = if r < ray * 0.5 { WHITE } else { ICE };
                for (dx, dy) in [(1.0, 0.0), (-1.0, 0.0), (0.0, 1.0), (0.0, -1.0)] {
                    p.dot(dx * r, dy * r, c, 1.0);
                }
                if r <= ray * 0.35 {
                    for (dx, dy) in [(1.0, 1.0), (-1.0, 1.0), (1.0, -1.0), (-1.0, -1.0)] {
                        p.dot(dx * r, dy * r, AQUA, 1.0);
                    }
                }
            }
            p.glow(0.0, 0.0, 10.0, WHITE, 0.9 * k);
            p.glow(0.0, 0.0, 18.0, GLOW, 0.6 * k);
            for i in 0..6 {
                let fi = f64::from(i);
                // thrown back the way the sword came, fanning up and down
                let ang = PI + (rand(&[fi, self.seed, 29.0]) - 0.5) * 2.4;
                let speed = 50.0 + 50.0 * rand(&[fi, self.seed, 31.0]);
                let x = libm::cos(ang) * speed * t;
                let y = libm::sin(ang) * speed * t + 60.0 * t * t;
                let c = if i % 2 == 0 { WHITE } else { CYAN };
                p.dot(libm::floor(x), libm::floor(y), c, k);
                p.light(x, y, GLOW, 0.6 * k);
            }
        }
        // chips of the blade, falling and fading
        for i in 0..3 {
            let fi = f64::from(i);
            let vx = (rand(&[fi, self.seed, 41.0]) - 0.7) * 50.0;
            let vy = -30.0 - 30.0 * rand(&[fi, self.seed, 43.0]);
            let x = libm::floor(vx * t);
            let y = libm::floor(vy * t + 140.0 * t * t);
            let k = 1.0 - t / META.duration;
            p.dot(x, y, SKY, k);
            p.dot(x + 1.0, y, ICE, k);
        }
    }
}
