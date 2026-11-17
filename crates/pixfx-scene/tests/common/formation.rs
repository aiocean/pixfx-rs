//! Test effects that take orders (`#[path = "common/formation.rs"] mod formation;`): no effect in `pixfx-effects`
//! takes one, so the scene's volley is driven by these. [`FORMATION`] holds eight blades in a ring 30 pixels above
//! where it plays and answers an order with all eight, a tenth of a second apart, trailed by `projectiles/trail`;
//! [`THROW`] holds one and throws it once, landing on the point, where a [`SPARK`] shakes the screen. It needs only
//! `pixfx-core`, `pixfx-draw` and `pixfx-effects`, so `pixfx-gpui`'s tests take it too.

use core::f64::consts::{FRAC_1_SQRT_2, FRAC_PI_2, FRAC_PI_4, PI};

use pixfx_core::volley::{Launch, Order, Shot, Tail, Volley};
use pixfx_core::{Cue, Effect, EffectState, Layer, Meta, Options, TimedCue};
use pixfx_draw::{Painter, Rgba};

const WHITE: Rgba = Rgba(255, 255, 255, 255);

const fn meta(id: &'static str, looping: bool, duration: f64) -> Meta {
    Meta {
        id,
        name: id,
        desc: "A test effect for orders.",
        tags: &[],
        size: (64, 64),
        origin: (32, 48),
        duration,
        looping,
        fps: 30,
        seed: 1,
        state: None,
        options: &[],
    }
}

/// Draws a blade as a 3 x 3 square on its origin, or one per shot of `shots` round the ring.
struct Blades {
    shots: Vec<Shot>,
}

impl EffectState for Blades {
    fn draw(&self, p: &mut Painter, _t: f64, layer: Layer) {
        if layer == Layer::Back {
            return;
        }
        for s in &self.shots {
            p.rect(s.at.0 - 1.0, s.at.1 - 1.0, 3.0, 3.0, WHITE, 1.0);
        }
    }
}

fn blades(shots: Vec<Shot>) -> Box<dyn EffectState> {
    Box::new(Blades { shots })
}

/// One blade: what flies.
pub struct Blade;

pub static BLADE: Blade = Blade;

const BLADE_META: Meta = meta("test/blade", true, 1.0);

impl Effect for Blade {
    fn meta(&self) -> &Meta {
        &BLADE_META
    }
    fn spawn(&self, _opts: &Options, _seed: u64) -> Box<dyn EffectState> {
        blades(vec![Shot {
            at: (0.0, 0.0),
            heading: 0.0,
        }])
    }
}

/// A short one-shot that shakes the screen as it starts: what plays where a thrown blade lands.
pub struct Spark;

pub static SPARK: Spark = Spark;

const SPARK_META: Meta = meta("test/spark", false, 0.2);

impl Effect for Spark {
    fn meta(&self) -> &Meta {
        &SPARK_META
    }
    fn spawn(&self, _opts: &Options, _seed: u64) -> Box<dyn EffectState> {
        blades(Vec::new())
    }
    fn cues(&self, _opts: &Options) -> Vec<TimedCue> {
        vec![TimedCue {
            t: 0.0,
            cue: Cue::Shake { trauma: 0.5 },
            pan: None,
        }]
    }
}

/// How both test effects fly their blades.
const VOLLEY: Volley = Volley {
    piece: &BLADE,
    held: None,
    tail: None,
    hit: None,
    poise: 0.2,
    gap: 0.1,
    speed: 600.0,
    min_secs: 0.25,
    max_secs: 1.2,
    nose: 8.0,
    sink: 2.0,
    bend: 0.3,
    bend_min: 20.0,
    bend_max: 80.0,
    spread: (6.0, 16.0),
    ease: 1.3,
    beyond: 200.0,
    lands: false,
    orbit_turns: 0.0,
    orbit_radius: 0.0,
};

/// The trail behind each blade of the formation.
fn tail_opts() -> Options {
    Options::default()
}

fn trail() -> Option<Tail> {
    pixfx_effects::effect("projectiles/trail").map(|effect| Tail {
        effect,
        opts: tail_opts,
    })
}

/// Eight shots on a ring of radius `r` round `centre` off the effect's origin, from the top clockwise, each pointing
/// out, mirrored when `flip`, on the stage where the effect stands at `at`. No trigonometry: the eighth turns are a
/// table, so the fixture needs no `libm`.
fn ring(r: f64, centre: (f64, f64), at: (f64, f64), flip: bool) -> Vec<Shot> {
    let d = FRAC_1_SQRT_2;
    let dirs = [
        (0.0, -1.0),
        (d, -d),
        (1.0, 0.0),
        (d, d),
        (0.0, 1.0),
        (-d, d),
        (-1.0, 0.0),
        (-d, -d),
    ];
    let m = if flip { -1.0 } else { 1.0 };
    (0..8u32)
        .zip(dirs)
        .map(|(k, (dx, dy))| {
            let a = -FRAC_PI_2 + FRAC_PI_4 * f64::from(k);
            Shot {
                at: (at.0 + m * (centre.0 + dx * r), at.1 + centre.1 + dy * r),
                heading: if flip { PI - a } else { a },
            }
        })
        .collect()
}

/// Eight blades in a ring 20 pixels round a point 30 above where it plays; ordered, it launches all eight.
pub struct Formation;

pub static FORMATION: Formation = Formation;

const FORMATION_META: Meta = meta("test/formation", true, 1.0);

impl Effect for Formation {
    fn meta(&self) -> &Meta {
        &FORMATION_META
    }
    fn spawn(&self, _opts: &Options, _seed: u64) -> Box<dyn EffectState> {
        blades(ring(20.0, (0.0, -30.0), (0.0, 0.0), false))
    }
    fn order(
        &self,
        opts: &Options,
        _t: f64,
        at: (f64, f64),
        flip: bool,
        order: &Order,
    ) -> Vec<Launch> {
        let v = Volley {
            tail: trail(),
            ..VOLLEY
        };
        let seed = opts.seed.unwrap_or(self.meta().seed) as f64;
        v.aim(&ring(20.0, (0.0, -30.0), at, flip), order, seed)
    }
}

/// One blade held 10 right of and 20 above where it plays; ordered, it throws it once onto the point, where a
/// [`SPARK`] plays.
pub struct Throw;

pub static THROW: Throw = Throw;

const THROW_META: Meta = meta("test/throw", true, 1.0);

impl Effect for Throw {
    fn meta(&self) -> &Meta {
        &THROW_META
    }
    fn spawn(&self, _opts: &Options, _seed: u64) -> Box<dyn EffectState> {
        blades(vec![Shot {
            at: (10.0, -20.0),
            heading: 0.0,
        }])
    }
    fn order(
        &self,
        _opts: &Options,
        _t: f64,
        at: (f64, f64),
        flip: bool,
        order: &Order,
    ) -> Vec<Launch> {
        let v = Volley {
            hit: Some(&SPARK),
            spread: (0.0, 0.0),
            speed: 400.0,
            lands: true,
            ..VOLLEY
        };
        let m = if flip { -1.0 } else { 1.0 };
        let shot = Shot {
            at: (at.0 + m * 10.0, at.1 - 20.0),
            heading: if flip { PI } else { 0.0 },
        };
        v.aim(&[shot], order, 1.0)
    }
}
