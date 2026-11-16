//! Heal sparkle: a green cross pops up over a ripple on the ground, spins away, bursts into a four-pointed star
//! and leaves motes that fade where they hang. Drawn after a reference gif (itch.io, `Aghj2J.gif`: 17 frames of
//! 70 ms, 2x art); every shape is painted here from its frame's numbers, never from the gif's pixels.
//!
//! It plays on a character from the feet, as every effect on a body does, and draws over the head: the ripple
//! under the cross sits on the crown of a fighter of [`RULER`] height.

use pixfx_core::{Effect, EffectState, Layer, Meta, Options};
use pixfx_draw::Painter;
use pixfx_draw::RULER;
use pixfx_draw::color::Rgba;
use pixfx_draw::math::{js_round, rand};

pub struct HealSparkle;
pub static HEAL_SPARKLE: HealSparkle = HealSparkle;

/// How long the reference holds each frame, and how many it has.
const STEP: f64 = 0.07;
const STEPS: usize = 17;
/// Rows from the star's centre down to the middle of the ripple under the cross.
const RIPPLE: i32 = 10;
/// How far above the feet the star's centre is drawn: the ripple on the crown.
const LIFT: i32 = RULER.height as i32 + RIPPLE;
/// Half the star's reach, in pixels from its centre.
const REACH: i32 = 23;

const META: Meta = Meta {
    id: "feedback/heal-sparkle",
    name: "Heal sparkle",
    desc: "A green cross pops up over a ripple, spins away, bursts into a four-pointed star and leaves fading motes.",
    tags: &["feedback", "heal", "buff", "sparkle", "loop"],
    // the star's reach on every side, down to the feet
    size: (REACH as u32 * 2 + 2, (REACH + LIFT + 1) as u32),
    origin: (REACH, REACH + LIFT),
    duration: STEP * STEPS as f64,
    looping: true,
    fps: 30,
    seed: 1,
    state: None,
    options: &[],
};

/// Dark green to white; heat 0..1 picks a step.
const PALETTE: [Rgba; 6] = [
    Rgba::hex24(0x09bb05),
    Rgba::hex24(0x53e909),
    Rgba::hex24(0xa7ff10),
    Rgba::hex24(0xccff10),
    Rgba::hex24(0xffff10),
    Rgba::hex24(0xffffff),
];

/// The cross on each of its frames: arm reach (x, y), half thickness, how far the turn has gone (0 upright, 1 an X),
/// how hot its core is (0..1), and the ripple's half width and half height (0 for none).
struct Cross {
    reach: (f64, f64),
    half: f64,
    turn: f64,
    core: f64,
    ripple: (f64, f64),
    broken: f64,
}

const CROSS: [Cross; 7] = [
    Cross {
        reach: (5.0, 4.1),
        half: 2.0,
        turn: 0.0,
        core: 0.85,
        ripple: (2.0, 1.0),
        broken: 0.0,
    },
    Cross {
        reach: (10.0, 9.0),
        half: 3.0,
        turn: 0.0,
        core: 1.0,
        ripple: (4.5, 2.0),
        broken: 0.0,
    },
    Cross {
        reach: (11.0, 10.0),
        half: 3.0,
        turn: 0.0,
        core: 1.0,
        ripple: (7.5, 3.0),
        broken: 0.0,
    },
    Cross {
        reach: (11.0, 10.0),
        half: 3.0,
        turn: 0.0,
        core: 1.0,
        ripple: (8.5, 3.0),
        broken: 0.0,
    },
    Cross {
        reach: (9.0, 9.0),
        half: 3.0,
        turn: 0.7,
        core: 0.9,
        ripple: (8.5, 3.0),
        broken: 0.4,
    },
    Cross {
        reach: (6.0, 6.0),
        half: 2.0,
        turn: 1.0,
        core: 0.75,
        ripple: (9.0, 3.5),
        broken: 0.75,
    },
    Cross {
        reach: (3.0, 3.0),
        half: 2.0,
        turn: 1.0,
        core: 0.15,
        ripple: (10.0, 4.0),
        broken: 0.9,
    },
];

/// The motes the star leaves, from its centre, each fading where it hangs.
const MOTES: [(i32, i32); 8] = [
    (0, -15),
    (6, -13),
    (-8, -9),
    (4, -3),
    (-2, 0),
    (7, 2),
    (-6, 5),
    (9, 6),
];

impl Effect for HealSparkle {
    fn meta(&self) -> &Meta {
        &META
    }
    fn spawn(&self, opts: &Options, seed: u64) -> Box<dyn EffectState> {
        Box::new(State {
            ramp: opts.palette_or(&PALETTE),
            seed: seed as f64,
        })
    }
}

struct State {
    ramp: Vec<Rgba>,
    seed: f64,
}

impl State {
    /// The ramp colour for heat 0..1, or none below the darkest step.
    fn shade(&self, heat: f64) -> Option<Rgba> {
        if heat < 0.08 {
            return None;
        }
        let n = self.ramp.len();
        let i = (heat * n as f64) as usize;
        self.ramp.get(i.min(n.saturating_sub(1))).copied()
    }

    /// One pixel at (x, y) from the star's centre.
    fn put(&self, p: &mut Painter, x: i32, y: i32, heat: f64) {
        if let Some(c) = self.shade(heat) {
            p.px(f64::from(x), f64::from(y - LIFT), c);
        }
    }

    /// The cross, upright or turned toward an X, hottest at its middle; centred between pixels, as drawn.
    fn cross(&self, p: &mut Painter, c: &Cross, k: usize) {
        let a = c.turn * core::f64::consts::FRAC_PI_4;
        let (s, co) = (libm::sin(a), libm::cos(a));
        let r = c.reach.0.max(c.reach.1) as i32 + 2;
        for y in -r..=r {
            for x in -r..=r {
                let (fx, fy) = (f64::from(x) - 0.5, f64::from(y) + 0.5);
                let (u, v) = (fx * co + fy * s, -fx * s + fy * co);
                let along = if u.abs() <= c.half {
                    v.abs() / c.reach.1
                } else {
                    2.0
                };
                let across = if v.abs() <= c.half {
                    u.abs() / c.reach.0
                } else {
                    2.0
                };
                let d = along.min(across);
                if d > 1.0 {
                    continue;
                }
                if c.broken > 0.5
                    && rand(&[f64::from(x), f64::from(y), k as f64, self.seed])
                        < (c.broken - 0.5) * d
                {
                    continue;
                }
                let heat = c.core * (1.0 - 0.75 * d) - 0.06 * u.abs().min(v.abs());
                self.put(p, x, y, heat.max(0.05));
            }
        }
    }

    /// The ripple under the cross: an ellipse's rim, its near half brighter.
    fn ripple(&self, p: &mut Painter, c: &Cross, k: usize) {
        let (rx, ry) = c.ripple;
        if rx <= 0.0 {
            return;
        }
        let n = (rx * 6.0) as i32;
        for i in 0..n {
            let a = core::f64::consts::TAU * f64::from(i) / f64::from(n);
            let x = js_round(libm::cos(a) * rx + 0.5) as i32;
            let y = js_round(libm::sin(a) * ry) as i32 + RIPPLE;
            if c.broken > 0.0 && rand(&[f64::from(i), k as f64, 7.0, self.seed]) < c.broken {
                continue;
            }
            let near = libm::sin(a).max(0.0);
            self.put(p, x, y, c.core * (0.55 + 0.45 * near));
        }
    }

    /// The four-pointed star: an astroid reaching `core`, white in the middle, and its two spines reaching
    /// `spine`, each pixel of a spine kept with chance `keep` as it breaks up.
    fn star(&self, p: &mut Painter, core: f64, spine: f64, keep: f64, heat: f64, k: usize) {
        let r = spine as i32;
        for y in -r..=r {
            for x in -r..=r {
                let (ax, ay) = (f64::from(x.abs()) / core, f64::from(y.abs()) / core);
                let d = libm::sqrt(ax) + libm::sqrt(ay);
                if d <= 1.0 {
                    self.put(p, x, y, (heat * (1.0 - 0.55 * d)).max(0.12));
                    continue;
                }
                if x != 0 && y != 0 {
                    continue;
                }
                let along = f64::from(x.abs() + y.abs()) / spine;
                if keep < 1.0
                    && rand(&[f64::from(x), f64::from(y), k as f64, 3.0, self.seed]) > keep
                {
                    continue;
                }
                self.put(p, x, y, (heat * (1.0 - along) + 0.3).max(0.12));
            }
        }
    }

    /// The motes: each a bright head over a dimmer tail, cooling a step a frame.
    fn motes(&self, p: &mut Painter, age: f64) {
        for (i, &(x, y)) in MOTES.iter().enumerate() {
            let heat = 1.05 - age * 0.22 - 0.04 * i as f64;
            self.put(p, x, y, heat);
            self.put(p, x, y + 1, heat - 0.2);
        }
    }
}

impl EffectState for State {
    fn draw(&self, p: &mut Painter, t: f64, layer: Layer) {
        if layer == Layer::Back {
            return;
        }
        let k = libm::floor(t / STEP + 1e-9).max(0.0) as usize % STEPS;
        if let Some(c) = CROSS.get(k) {
            self.cross(p, c, k);
            self.ripple(p, c, k);
            return;
        }
        match k {
            7 => self.star(p, f64::from(REACH), f64::from(REACH), 1.0, 1.0, k),
            8 => self.star(p, 9.0, f64::from(REACH), 0.75, 0.9, k),
            9 => self.star(p, 6.0, f64::from(REACH), 0.45, 0.7, k),
            10 => self.star(p, 3.0, f64::from(REACH), 0.25, 0.5, k),
            _ => {}
        }
        if (8..16).contains(&k) {
            self.motes(p, (k as f64 - 11.0).max(0.0));
        }
    }
}
