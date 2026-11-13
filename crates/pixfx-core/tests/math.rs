//! Fractal, tiling and loop noise against pixelfx `src/core/math.js`, values printed by `tests/golden/math.mjs`.

use pixfx_core::math::{fbm, loop_noise, tile_fbm, tile_noise};

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
