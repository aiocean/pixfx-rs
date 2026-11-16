//! Fluids: particles that push on each other, for liquid that splashes, pools and drips instead of flying alone.
//! Two solvers, each written here from what dimforge's simulators do (`references/dimforge`):
//!
//! - [`Sph`]: smoothed-particle hydrodynamics, particles only (salva's kernels and weakly compressible pressure).
//!   Light, splashy liquid: water thrown and breaking into droplets.
//! - [`Mpm`]: the moving least squares material point method with affine velocities (sparkl and slosh). Particles
//!   carried on a small grid; thick liquid that keeps its body: blood, mud, slime.
//!
//! Both are deterministic: `f64`, `libm`, sums in a fixed order, no hashing by address. Time comes in through
//! `step(dt)`, which splits `dt` into sub-steps of at most `max_dt`; `dt = 0` (a hit-stop) changes nothing.
//! Positions are in an effect's local pixels, y down.

mod mpm;
mod sph;

pub use mpm::{Mpm, MpmOpts, MpmParticle};
pub use sph::{Drop, Sph, SphOpts};

/// The number of equal sub-steps that cover `dt` with none longer than `max_dt`, and their length. None for a
/// `dt` that is zero, negative or not finite, or a `max_dt` that is not positive.
pub(crate) fn substeps(dt: f64, max_dt: f64) -> Option<(usize, f64)> {
    if !(dt.is_finite() && dt > 0.0 && max_dt.is_finite() && max_dt > 0.0) {
        return None;
    }
    let n = libm::ceil(dt / max_dt).clamp(1.0, 10_000.0) as usize;
    Some((n, dt / n as f64))
}

/// A floor at `y`: a particle below it is put back on it, its fall bounced by `restitution` and its slide slowed
/// by `friction`: the share of its sliding speed lost every 1/60 s on the floor (0 slides freely, 1 sticks), the
/// same whatever the sub-step.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Floor {
    pub y: f64,
    pub restitution: f64,
    pub friction: f64,
}

impl Floor {
    /// What a sliding speed is multiplied by over `dt` seconds on the floor.
    pub fn slide(&self, dt: f64) -> f64 {
        libm::pow(1.0 - self.friction.clamp(0.0, 1.0), dt * 60.0)
    }
}
