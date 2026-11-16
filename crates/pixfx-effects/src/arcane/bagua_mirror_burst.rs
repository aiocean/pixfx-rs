//! Bagua mirror burst: where the thrown bagua mirror lands it explodes. A white flash, a jade shock ring in the
//! mirror's octagon racing out, the eight trigrams thrown out from the middle each spinning away, bronze shards and
//! jade sparks falling, and a last glow. It fires a heavy impact cue. Made here from `pixfx-ref`'s
//! `examples/specs/shield/bagua-mirror.json` sheets; its golden hashes pin it.

use core::f64::consts::{FRAC_PI_4, PI, TAU};

use pixfx_core::cue::{FEEL, TimedCue, impact};
use pixfx_core::{Effect, EffectState, Layer, Meta, Options};
use pixfx_draw::Painter;
use pixfx_draw::halo::Halo;
use pixfx_draw::math::{clamp, rand};

use super::bagua;

pub struct BaguaMirrorBurst;

pub static BAGUA_MIRROR_BURST: BaguaMirrorBurst = BaguaMirrorBurst;

const META: Meta = Meta {
    id: "arcane/bagua-mirror-burst",
    name: "Bagua mirror burst",
    desc: "The thrown bagua mirror explodes: a white flash, a jade octagonal shock ring, its eight trigrams flung out spinning, bronze shards and sparks.",
    tags: &[
        "arcane",
        "impact",
        "explosion",
        "mirror",
        "cultivation",
        "one-shot",
    ],
    size: (112, 112),
    origin: (56, 56),
    duration: 0.8,
    looping: false,
    fps: 30,
    seed: 1,
    state: None,
    options: &[FEEL.spec()],
};

/// Seconds: the flash, the ring's run out, the trigrams' flight.
const FLASH: f64 = 0.12;
const RING: f64 = 0.35;
const FLY: f64 = 0.6;
/// Pixels: the ring's reach, how far the trigrams fly.
const REACH: f64 = 40.0;
const THROW: f64 = 34.0;
/// The trigrams, as on the mirror (`super::bagua`): a bit per line, innermost lowest.
const TRIGRAMS: [u8; 8] = [0b111, 0b011, 0b010, 0b001, 0b000, 0b100, 0b101, 0b110];

const W: usize = 112;
const H: usize = 112;
const OX: f64 = 56.0;
const OY: f64 = 56.0;

impl Effect for BaguaMirrorBurst {
    fn meta(&self) -> &Meta {
        &META
    }
    fn cues(&self, opts: &Options) -> Vec<TimedCue> {
        impact(0.0, 0.6, 0.0, 0.0, opts)
    }
    fn spawn(&self, _opts: &Options, seed: u64) -> Box<dyn EffectState> {
        Box::new(Burst { seed: seed as f64 })
    }
}

struct Burst {
    seed: f64,
}

impl EffectState for Burst {
    fn draw(&self, p: &mut Painter, t: f64, layer: Layer) {
        if layer == Layer::Back {
            return;
        }
        let mut halo = Halo::new(W, H, OX, OY);
        // the flash: a white disc shrinking
        if t < FLASH {
            let k = 1.0 - t / FLASH;
            let r = 6.0 + 8.0 * k;
            let mut y = -r;
            while y <= r {
                let mut x = -r;
                while x <= r {
                    if libm::hypot(x, y) <= r {
                        halo.stamp(x, y, 3.4, 1);
                        p.px(x, y, bagua::WHITE);
                    }
                    x += 1.0;
                }
                y += 1.0;
            }
        }
        self.ring(&mut halo, t);
        halo.spread(bagua::FALL);
        bagua::paint_halo(p, &halo);
        self.trigrams(p, t);
        bagua::shards(p, 0.0, 0.0, t, 18, self.seed, 0.75, 0.0);
        let k = clamp(1.0 - t / 0.5, 0.0, 1.0);
        if k > 0.0 {
            p.glow(0.0, 0.0, 16.0, bagua::WHITE, 1.2 * k);
            p.glow(0.0, 0.0, 34.0, bagua::GLOW, 0.8 * k);
        }
    }
}

impl Burst {
    /// The shock ring: the mirror's octagon racing out, its heat (so its halo) falling as it goes.
    fn ring(&self, halo: &mut Halo, t: f64) {
        if t > RING {
            return;
        }
        let s = t / RING;
        let r = 6.0 + (REACH - 6.0) * (1.0 - (1.0 - s) * (1.0 - s));
        let heat = (2.8 * (1.0 - s) + 0.6) as f32;
        let n = (r * 7.0) as i32;
        for i in 0..n {
            let a = f64::from(i) / f64::from(n) * TAU;
            // a point on the octagon: flat sides up, down and to the sides
            let side = libm::floor(a / FRAC_PI_4 + 0.5) * FRAC_PI_4;
            let d = r / libm::cos(a - side);
            halo.stamp(
                libm::floor(d * libm::cos(a)),
                libm::floor(d * libm::sin(a)),
                heat,
                1,
            );
        }
    }

    /// The eight trigrams flung out from the middle, each turning as it flies and fading.
    fn trigrams(&self, p: &mut Painter, t: f64) {
        if t > FLY {
            return;
        }
        let s = t / FLY;
        let k = 1.0 - s;
        for (i, bits) in TRIGRAMS.iter().enumerate() {
            let fi = i as f64;
            let way = fi * FRAC_PI_4 - PI / 2.0;
            let go = THROW * (1.0 - (1.0 - s) * (1.0 - s));
            let (cx, cy) = (
                libm::cos(way) * (6.0 + go),
                libm::sin(way) * (6.0 + go) + 20.0 * t * t,
            );
            // across the way it flies, turning
            let across = way + PI / 2.0 + (rand(&[fi, self.seed, 9.0]) - 0.5) * 6.0 * t;
            let (ux, uy) = (libm::cos(across), libm::sin(across));
            let (vx, vy) = (libm::cos(way), libm::sin(way));
            for line in 0..3 {
                let whole = bits >> line & 1 == 1;
                let off = f64::from(line) * 2.0 - 2.0;
                for j in -3..=3 {
                    if !whole && j == 0 {
                        continue;
                    }
                    let fj = f64::from(j);
                    let x = libm::floor(cx + ux * fj + vx * off);
                    let y = libm::floor(cy + uy * fj + vy * off);
                    let c = if s < 0.3 { bagua::WHITE } else { bagua::BAR };
                    p.dot(x, y, c, clamp(k * 1.6, 0.0, 1.0));
                    p.light(x, y, bagua::GLOW, 0.4 * k);
                }
            }
        }
    }
}
