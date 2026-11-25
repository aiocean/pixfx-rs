//! Particles without a simulation: each one's position is a closed-form function of its age (pixelfx `particles.js`).

use pixfx_draw::math::{Rng, rng};

/// Where a particle is after `t` seconds, launched from `(x, y)` at `(vx, vy)` px/s, under gravity `g` (px/s^2,
/// positive is down) and linear drag (1/s).
pub fn fly(x: f64, y: f64, vx: f64, vy: f64, t: f64, g: f64, drag: f64) -> (f64, f64) {
    if drag <= 0.0 {
        return (x + vx * t, y + vy * t + 0.5 * g * t * t);
    }
    let f = (1.0 - libm::exp(-drag * t)) / drag;
    let vt = g / drag;
    (x + vx * f, y + vt * t + (vy - vt) * f)
}

/// A particle's velocity after `t` seconds of [`fly`], for streaks that point along the motion (pixelfx `flyV`).
pub fn fly_v(vx: f64, vy: f64, t: f64, g: f64, drag: f64) -> (f64, f64) {
    if drag <= 0.0 {
        return (vx, vy + g * t);
    }
    let e = libm::exp(-drag * t);
    let vt = g / drag;
    (vx * e, vt + (vy - vt) * e)
}

/// Particles that repeat every `period` seconds, for loops with no seam. Particle `i` is born at its own phase and
/// lives `life(g)` seconds (`g` is its `rng(seed, i, 911)` after the birth draw; a fixed life ignores it). For every
/// live copy, `f(age, life, r, i)` is called with `r` a fresh `rng(seed, i)`.
pub fn cycle(
    t: f64,
    period: f64,
    count: usize,
    seed: f64,
    life: impl Fn(&mut Rng) -> f64,
    mut f: impl FnMut(f64, f64, &mut Rng, usize),
) {
    for i in 0..count {
        let mut g = rng(&[seed, i as f64, 911.0]);
        let born = g.next() * period;
        let l = life(&mut g);
        let mut age = (((t - born) % period) + period) % period;
        while age < l {
            f(age, l, &mut rng(&[seed, i as f64]), i);
            age += period;
        }
    }
}
