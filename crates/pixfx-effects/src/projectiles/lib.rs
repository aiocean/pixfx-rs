//! Helpers for the projectiles: a steady emitter for loops.
//! Not an effect (the file name starts with _).
//!
//! Port of pixelfx `src/fx/projectiles/_lib.js`: the functions laser-bolt uses (`beat`).

use pixfx_core::math::{Rng, rng, wrap};

/// Like `cycle`, but particle i is born exactly at i * period / n: an emitter with a steady beat, so a trail
/// has no clumps or gaps. Loops seamlessly over period. `f(age, life, r, i)`, r the same rng every period.
pub fn beat(
    t: f64,
    period: f64,
    n: usize,
    seed: f64,
    life: impl Fn(&mut Rng) -> f64,
    mut f: impl FnMut(f64, f64, &mut Rng, usize),
) {
    for i in 0..n {
        let l = life(&mut rng(&[seed, i as f64, 911.0]));
        let mut age = wrap(t - (i as f64 * period) / n as f64 + 1e-7, period);
        while age < l {
            f(age, l, &mut rng(&[seed, i as f64]), i);
            age += period;
        }
    }
}
