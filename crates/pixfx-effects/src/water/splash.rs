//! Splash: water bursts up where something struck it. A column shoots up from the middle and a crown opens round it;
//! surface tension pulls the sheets into round droplets, which rain back down, run together into a puddle and soak
//! away. Made here, on the SPH liquid (`pixfx_core::fluid::Sph`, with Akinci surface tension): only the first moment
//! is set (where the water is and how it is thrown); the droplets, the rain and the puddle are the physics, so no two
//! splashes are alike. Painted as one body with `pixfx_draw::blob` in the water ramp: a lit top, a dark outline, the
//! body, deeper where it is thick, and white foam on fast lone droplets. Its golden hashes pin it.

use pixfx_core::cue::{FEEL, TimedCue, impact};
use pixfx_core::fluid::{Floor, Sph, SphOpts};
use pixfx_core::{Effect, EffectState, Layer, Meta, NumOpt, Options};
use pixfx_draw::Painter;
use pixfx_draw::blob::{Blob, BlobLook};
use pixfx_draw::color::Rgba;
use pixfx_draw::math::rand;

use super::lib::WATER;

pub struct Splash;

pub static SPLASH: Splash = Splash;

/// Radius of the water thrown up: 9 px by default, a little over the fighter's 7-pixel face (`RULER.head`, ratio 1.3).
const SIZE: NumOpt = NumOpt::new("size", 9.0).within(4.0, 14.0);

const META: Meta = Meta {
    id: "water/splash",
    name: "Splash",
    desc: "Water bursts up where something struck it: a column shoots up from the middle and a crown opens round it, breaks into round droplets that rain back down, runs together into a puddle and soaks away. Simulated liquid: no two splashes alike.",
    tags: &["water", "liquid", "splash", "impact", "one-shot"],
    size: (160, 96),
    origin: (80, 76),
    duration: 2.0,
    looping: false,
    fps: 30,
    seed: 3,
    state: None,
    options: &[SIZE.spec(), FEEL.spec()],
};

/// The crown: speed up the middle column, up at the rim, and out sideways at the rim (px/s).
const COLUMN: f64 = 230.0;
const RIM_UP: f64 = 80.0;
const RIM_OUT: f64 = 95.0;
const GRAVITY: f64 = 320.0;
/// When the puddle starts soaking in, and how long the last of it takes.
const SOAK_AT: f64 = 0.9;
const SOAK_FOR: f64 = 0.95;
/// A droplet this fast (px/s) and this alone (density share) carries foam: spray, never the body of water.
const FOAM_SPEED: f64 = 110.0;
const FOAM_FILL: f64 = 0.55;

impl Effect for Splash {
    fn meta(&self) -> &Meta {
        &META
    }
    fn cues(&self, opts: &Options) -> Vec<TimedCue> {
        impact(0.0, 0.35, 0.0, 0.0, opts)
    }
    fn spawn(&self, opts: &Options, seed: u64) -> Box<dyn EffectState> {
        let r = opts.num(&SIZE);
        let (w, h) = (f64::from(META.size.0), f64::from(META.size.1));
        let (ox, oy) = (f64::from(META.origin.0), f64::from(META.origin.1));
        let mut sph = Sph::new(SphOpts {
            // a pixel apart: one particle a game pixel, so a puddle a pixel deep still reads as water
            h: 2.4,
            spacing: 1.2,
            // ten times the fastest the water moves, so it does not squash like a sponge
            sound: 500.0,
            max_dt: 1.0 / 1000.0,
            viscosity: 0.87,
            tension: 1500.0,
            gravity: GRAVITY,
            floor: Some(Floor {
                y: 0.0,
                restitution: 0.1,
                friction: 0.5,
            }),
            bounds: (-ox, -oy, w - ox, h - oy),
        });
        // the water at the moment of the blow: a body sitting on the ground, thrown up fastest in the middle (a
        // column) and out and up at its sides (the crown's walls), each drop a little off its neighbours
        sph.add_disc(0.0, -r, r, 0.0, 0.0, 0);
        let s = seed as f64;
        for (i, d) in sph.drops_mut().iter_mut().enumerate() {
            d.tag = i as u32;
            let a = (d.x / r).clamp(-1.0, 1.0);
            let k = 1.0 - a.abs();
            let jit = 0.8 + 0.4 * rand(&[s, i as f64, 1.0]);
            d.vx = a * RIM_OUT * jit;
            d.vy = -(RIM_UP + COLUMN * k * k)
                * (0.85 + 0.3 * rand(&[s, i as f64, 2.0]))
                * (0.6 + 0.4 * (-d.y / (2.0 * r)));
        }
        let ramp = opts.palette_or(&WATER);
        let at = |i: usize| {
            ramp.get(i.min(ramp.len().saturating_sub(1)))
                .copied()
                .unwrap_or(WATER[3])
        };
        Box::new(State {
            sph,
            t: 0.0,
            seed: seed as f64,
            look: BlobLook {
                edge: at(1),
                body: at(3),
                deep: at(2),
                rim: at(6),
                iso: 0.5,
                deep_at: 2.4,
                shine: 0.25,
            },
            foam: at(7),
        })
    }
}

struct State {
    sph: Sph,
    t: f64,
    seed: f64,
    look: BlobLook,
    foam: Rgba,
}

impl EffectState for State {
    fn update(&mut self, dt: f64) {
        if dt.is_nan() || dt <= 0.0 {
            return;
        }
        self.t += dt;
        self.sph.step(dt);
        // each drop soaks in at its own moment once the puddle has settled
        let (t, seed) = (self.t, self.seed);
        self.sph
            .drops_mut()
            .retain(|d| t < SOAK_AT + SOAK_FOR * rand(&[seed, f64::from(d.tag)]));
    }

    fn draw(&self, p: &mut Painter, _t: f64, layer: Layer) {
        if layer == Layer::Back {
            return;
        }
        let drops = self.sph.drops();
        let Some(first) = drops.first() else { return };
        const R: f64 = 1.9;
        let (mut x0, mut y0, mut x1, mut y1) = (first.x, first.y, first.x, first.y);
        for d in drops {
            x0 = x0.min(d.x);
            y0 = y0.min(d.y);
            x1 = x1.max(d.x);
            y1 = y1.max(d.y);
        }
        let mut blob = Blob::new(x0 - R, y0 - R, x1 + R + 1.0, y1 + R + 1.0);
        for d in drops {
            blob.splat(d.x, d.y, R, 1.0);
        }
        blob.paint(p, &self.look);
        for d in drops {
            if d.fill < FOAM_FILL && libm::hypot(d.vx, d.vy) > FOAM_SPEED {
                p.px(d.x, d.y, self.foam);
            }
        }
    }
}
