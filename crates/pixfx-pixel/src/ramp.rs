//! Ramps: one material's colours from dark to light, made from one base colour, after klad's palette ramps
//! (tools/klad, crates/core/src/palettes.rs). Darkening by adding black and lightening by adding white looks like
//! plastic in fog; a ramp shifts the hue with the value instead: darker steps lean cool (blue), lighter steps lean
//! warm (yellow). Steps are even in OKLab lightness, so each looks one step from the next.

use crate::Rgba;
use crate::oklab::{Lab, lab, rgb};

/// OKLab lightness between two steps of a ramp.
pub const STEP: f64 = 0.09;
/// A ramp's steps, at least and at most.
pub const MIN_STEPS: usize = 2;
pub const MAX_STEPS: usize = 16;

/// `c` moved `by` in lightness, its hue leaning warm as it lightens and cool as it darkens; alpha kept.
pub fn value_step(c: Rgba, by: f64) -> Rgba {
    let p = lab(c);
    rgb(
        Lab {
            l: (p.l + by).clamp(0.0, 1.0),
            a: p.a + by * 0.03,
            b: p.b + by * 0.15,
        },
        c.3,
    )
}

/// `steps` colours (clamped to `MIN_STEPS..=MAX_STEPS`) from dark to light, evenly round `base`: the middle step is
/// `base` itself when `steps` is odd.
pub fn ramp(base: Rgba, steps: usize) -> Vec<Rgba> {
    let steps = steps.clamp(MIN_STEPS, MAX_STEPS);
    let middle = (steps - 1) as f64 / 2.0;
    (0..steps)
        .map(|i| value_step(base, (i as f64 - middle) * STEP))
        .collect()
}
