//! Shared by the scifi effects (the leading _ keeps the tools from listing it as an effect).
//! Ghosts: the toolkit's simulated particles for loops (sparks that fall and bounce, smoke that rises and thins)
//! worked out from their birth each frame, so a loop is a function of t alone and has no seam. Their motion is the
//! toolkit's (`pixfx_core::sim`), which is adapted from Sprite Fusion's open-source pixel art VFX
//! (https://www.spritefusion.com/pixel-art-vfx), MIT License, Copyright (c) 2026 Hugo Duprez.
//!
//! Port of pixelfx `src/fx/scifi/_lib.js`: what jetpack uses of `Ghosts` (fire, smoke, spark and ember particles
//! over an unbounded floor; the other kinds, `ground`, stars and the flash and arc helpers arrive with the effects
//! that need them). JS `Ghosts` also builds an `Rng` for its `Particles`; nothing draws from it, so it is not kept.

use pixfx_core::color::{Rgba, WHITE};
use pixfx_core::math::{js_min, rand};
use pixfx_core::sim::{Flags, GRAVITY, Kind, Particle, Particles, ParticlesOpts};

const STEP: f64 = 1.0 / 240.0;

/// One toolkit step for one particle, with its own age standing in for the system clock (so it repeats).
fn step(q: &mut Particle, dt: f64, age: f64) {
    match q.kind {
        Kind::Fire => {
            q.vy -= 95.0 * dt;
            q.vx += libm::sin(age * 17.0 + q.seed) * 30.0 * dt;
            q.vx -= q.vx * 5.0 * dt;
            q.vy -= q.vy * 3.2 * dt;
        }
        // the floor is out of reach (ground = Infinity), so a spark never bounces
        Kind::Spark => {
            q.vy += GRAVITY * 0.45 * dt;
            q.vx -= q.vx * 2.2 * dt;
            q.vy -= q.vy * 2.2 * dt;
        }
        Kind::Ember => {
            q.vy += (-30.0 + libm::sin(age * 3.0 + q.seed) * 20.0) * dt;
            q.vx += libm::cos(age * 2.3 + q.seed) * 45.0 * dt;
            q.vx -= q.vx * 1.6 * dt;
            q.vy -= q.vy * 1.6 * dt;
        }
        Kind::Smoke => {
            let big = q.flags.contains(Flags::BIG);
            q.vy -= (if big { 23.0 } else { 16.0 }) * dt;
            q.vx += libm::sin(age * 1.3 + q.seed) * 8.0 * dt;
            q.vx -= q.vx * 1.6 * dt;
            q.vy -= q.vy * 1.2 * dt;
        }
        _ => {}
    }
    q.x += q.vx * dt;
    q.y += q.vy * dt;
}

/// A particle's birth: when in the period, and how.
#[derive(Clone, Debug)]
struct Spec {
    born: f64,
    kind: Kind,
    x: f64,
    y: f64,
    vx: f64,
    vy: f64,
    life: f64,
    col: Rgba,
    size: f64,
    flags: Flags,
    seed: f64,
}

/// A particle's spawn arguments, as pixelfx `Ghosts.add` takes them after `born`.
#[derive(Clone, Copy, Debug)]
pub struct Birth {
    pub kind: Kind,
    pub x: f64,
    pub y: f64,
    pub vx: f64,
    pub vy: f64,
    pub life: f64,
    pub col: Rgba,
    pub size: f64,
    pub flags: Flags,
}

impl Birth {
    /// A birth with pixelfx's defaults: white, size 1, no flags.
    pub fn new(kind: Kind, x: f64, y: f64, vx: f64, vy: f64, life: f64) -> Self {
        Self {
            kind,
            x,
            y,
            vx,
            vy,
            life,
            col: WHITE,
            size: 1.0,
            flags: Flags::NONE,
        }
    }
}

/// Simulated particles for loops. [`Ghosts::add`] once, at build time, for each particle born `born` seconds into
/// the period; [`Ghosts::at`] returns a [`Particles`] holding every particle alive at t, each moved there from its
/// birth by the toolkit's physics, ready for `draw`. A particle born late in the period carries on into the next
/// one, so the loop has no seam. Particles outside `box` (`[x0, y0, x1, y1]`) are dropped (keeps sparks on the
/// canvas).
#[derive(Clone, Debug)]
pub struct Ghosts {
    period: f64,
    bounds: [f64; 4],
    ps: Particles,
    specs: Vec<Spec>,
}

impl Ghosts {
    /// `sparks`: the SPARK and EMBER colours, head to tail.
    pub fn new(period: f64, sparks: Vec<Rgba>, bounds: [f64; 4]) -> Self {
        Self {
            period,
            bounds,
            ps: Particles::new(ParticlesOpts {
                sparks,
                ..ParticlesOpts::default()
            }),
            specs: Vec::new(),
        }
    }

    pub fn add(&mut self, born: f64, b: Birth) {
        let n = self.specs.len() as f64;
        let d = self.period;
        self.specs.push(Spec {
            born: ((born % d) + d) % d,
            kind: b.kind,
            x: b.x,
            y: b.y,
            vx: b.vx,
            vy: b.vy,
            life: b.life,
            col: b.col,
            size: b.size,
            flags: b.flags,
            seed: rand(&[n, 41.0]) * 100.0,
        });
    }

    /// The state of one particle `age` seconds after its birth, or None if it's gone.
    fn state(&self, s: &Spec, age: f64) -> Option<Particle> {
        let mut q = Particle {
            kind: s.kind,
            x: s.x,
            y: s.y,
            vx: s.vx,
            vy: s.vy,
            life: s.life,
            max: s.life,
            col: s.col,
            size: s.size,
            seed: s.seed,
            delay: 0.0,
            flags: s.flags,
        };
        let mut a = 0.0;
        while a < age - 1e-9 {
            let dt = js_min(STEP, age - a);
            step(&mut q, dt, a);
            a += dt;
            q.life -= dt;
            if q.life <= 0.0 {
                // a fire puff with SMOKES turns into smoke when it dies, as in the toolkit
                if q.kind != Kind::Fire || !q.flags.contains(Flags::SMOKES) {
                    return None;
                }
                let big = q.flags.contains(Flags::BIG);
                let l = if big {
                    1.3 + rand(&[s.seed * 7.0, 1.0])
                } else {
                    0.8 + rand(&[s.seed * 7.0, 1.0]) * 0.6
                };
                q.kind = Kind::Smoke;
                q.vx *= 0.6;
                q.vy = q.vy * 0.6 - 6.0;
                q.life = l;
                q.max = l;
                q.col = Rgba::NONE;
                q.size *= if big { 0.85 } else { 0.7 };
                q.flags = if big { Flags::BIG } else { Flags::NONE };
            }
        }
        let [x0, y0, x1, y1] = self.bounds;
        if q.x < x0 || q.x > x1 || q.y < y0 || q.y > y1 {
            return None;
        }
        Some(q)
    }

    /// Every particle alive at t.
    pub fn at(&self, t: f64) -> Particles {
        let d = self.period;
        let mut ps = self.ps.clone();
        ps.set_time(t);
        let list = ps.list_mut();
        for s in &self.specs {
            // its age now, and in each earlier period it may still be alive from (smoke from fire lives longer)
            let span = if s.kind == Kind::Fire && s.flags.contains(Flags::SMOKES) {
                s.life + 2.4
            } else {
                s.life
            };
            let mut age = (((t - s.born) % d) + d) % d;
            while age < span {
                if let Some(q) = self.state(s, age) {
                    list.push(q);
                }
                age += d;
            }
        }
        ps
    }
}
