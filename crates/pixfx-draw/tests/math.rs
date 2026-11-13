//! Fractal, tiling and loop noise against pixelfx `src/core/math.js`, values printed by `tests/golden/math.mjs`.

use pixfx_draw::math::{fbm, loop_noise, tile_fbm, tile_noise};

const EPS: f64 = 1e-12;

#[test]
fn noise_matches_pixelfx() {
    let cases = [
        ("fbm", fbm(1.3, 2.7, 5.0, 3), 0.22283987753265405),
        (
            "tile_noise repeating",
            tile_noise(3.3, 7.6, 4.0, 8.0, 2.0),
            -0.03363736768955028,
        ),
        (
            "tile_noise plain",
            tile_noise(3.3, 7.6, 0.0, 0.0, 2.0),
            -0.5019162492718994,
        ),
        (
            "tile_fbm",
            tile_fbm(3.3, 7.6, 4.0, 8.0, 2.0, 3),
            -0.07741700167556775,
        ),
        (
            "loop_noise 0",
            loop_noise(0.0, 1.5, 4.0),
            -0.05786121403798461,
        ),
        (
            "loop_noise 0.37",
            loop_noise(0.37, 1.5, 4.0),
            -0.1010169040641491,
        ),
        (
            "loop_noise 1",
            loop_noise(1.0, 1.5, 4.0),
            -0.05786121403798461,
        ),
    ];
    for (name, got, want) in cases {
        assert!((got - want).abs() <= EPS, "{name} = {got}, pixelfx {want}");
    }
}

#[test]
fn perlin_is_smooth_bounded_and_never_rests() {
    use pixfx_draw::math::perlin;
    let step = 1.0 / 64.0;
    let mut prev = perlin(0.0, 7.0);
    for i in 1..=64 * 16 {
        let v = perlin(f64::from(i) * step, 7.0);
        assert!((-1.0..=1.0).contains(&v), "bounded: {v}");
        // its rate of change stays under 4 a cell, so a 1/64 step moves it a few hundredths; fresh random
        // values would jump by up to 2
        assert!(
            (v - prev).abs() <= 4.0 * step,
            "smooth at {i}: {prev} to {v}"
        );
        prev = v;
    }
    assert_eq!(perlin(3.0, 7.0), 0.0, "zero at whole numbers");
    // value noise comes to rest at every whole number (its slope there is 0); gradient noise passes through
    let resting = (0..64)
        .filter(|&n| (perlin(f64::from(n) + 1e-3, 7.0) - perlin(f64::from(n), 7.0)).abs() < 1e-5)
        .count();
    assert!(resting < 4, "{resting} of 64 lattice points at rest");
    assert_ne!(perlin(2.5, 1.0), perlin(2.5, 2.0), "seeds differ");
}
