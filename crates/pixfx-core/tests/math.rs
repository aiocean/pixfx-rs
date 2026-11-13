//! Easing, fractal and tiling noise (pixelfx `src/core/math.js`) and the `PAL` ramps (`src/core/color.js`).

use pixfx_core::color::pal;
use pixfx_core::math::{ease, fbm, loop_noise, tile_fbm, tile_noise};

const EPS: f64 = 1e-12;

const EASE_T: [f64; 7] = [-0.1, 0.0, 0.3, 0.5, 0.9, 1.0, 1.2];
type EaseCase = (&'static str, fn(f64) -> f64, [f64; 7]);

/// Printed by `tests/golden/math.mjs` at pixelfx 044e12a.
fn ease_cases() -> Vec<EaseCase> {
    vec![
        ("linear", ease::linear, [-0.1, 0.0, 0.3, 0.5, 0.9, 1.0, 1.2]),
        (
            "in_quad",
            ease::in_quad,
            [0.010000000000000002, 0.0, 0.09, 0.25, 0.81, 1.0, 1.44],
        ),
        (
            "out_quad",
            ease::out_quad,
            [-0.2100000000000002, 0.0, 0.51, 0.75, 0.99, 1.0, 0.96],
        ),
        (
            "in_out_quad",
            ease::in_out_quad,
            [0.020000000000000004, 0.0, 0.18, 0.5, 0.98, 1.0, 0.92],
        ),
        (
            "in_cubic",
            ease::in_cubic,
            [
                -0.0010000000000000002,
                0.0,
                0.027,
                0.125,
                0.7290000000000001,
                1.0,
                1.728,
            ],
        ),
        (
            "out_cubic",
            ease::out_cubic,
            [-0.3310000000000004, 0.0, 0.657, 0.875, 0.999, 1.0, 1.008],
        ),
        (
            "in_out_cubic",
            ease::in_out_cubic,
            [-0.004000000000000001, 0.0, 0.108, 0.5, 0.996, 1.0, 1.032],
        ),
        (
            "in_quart",
            ease::in_quart,
            [
                0.00010000000000000003,
                0.0,
                0.0081,
                0.0625,
                0.6561000000000001,
                1.0,
                2.0736,
            ],
        ),
        (
            "out_quart",
            ease::out_quart,
            [
                -0.4641000000000004,
                0.0,
                0.7599,
                0.9375,
                0.9999,
                1.0,
                0.9984,
            ],
        ),
        (
            "in_expo",
            ease::in_expo,
            [0.0, 0.0, 0.0078125, 0.03125, 0.5, 1.0, 4.0],
        ),
        (
            "out_expo",
            ease::out_expo,
            [-1.0, 0.0, 0.875, 0.96875, 0.998046875, 1.0, 1.0],
        ),
        (
            "in_sine",
            ease::in_sine,
            [
                0.01231165940486223,
                0.0,
                0.1089934758116321,
                0.2928932188134524,
                0.843565534959769,
                0.9999999999999999,
                1.3090169943749475,
            ],
        ),
        (
            "out_sine",
            ease::out_sine,
            [
                -0.15643446504023087,
                0.0,
                0.4539904997395468,
                0.7071067811865475,
                0.9876883405951378,
                1.0,
                0.9510565162951536,
            ],
        ),
        (
            "in_out_sine",
            ease::in_out_sine,
            [
                0.024471741852423234,
                0.0,
                0.20610737385376343,
                0.49999999999999994,
                0.9755282581475768,
                1.0,
                0.9045084971874737,
            ],
        ),
        (
            "in_back",
            |t| ease::in_back(t, ease::BACK_S),
            [
                0.0,
                0.0,
                -0.08019953999999999,
                -0.08769750000000004,
                0.5911720200000001,
                1.0,
                1.0,
            ],
        ),
        (
            "out_back",
            |t| ease::out_back(t, ease::BACK_S),
            [
                0.0,
                0.0,
                0.9071322600000001,
                1.0876975,
                1.01431422,
                1.0,
                1.0,
            ],
        ),
        (
            "out_elastic",
            ease::out_elastic,
            [0.0, 0.0, 0.875, 1.015625, 0.998046875, 1.0, 1.0],
        ),
        (
            "out_bounce",
            ease::out_bounce,
            [
                0.07562500000000001,
                0.0,
                0.6806249999999999,
                0.765625,
                0.9881249999999999,
                1.0,
                1.4399999999999997,
            ],
        ),
    ]
}

#[test]
fn ease_matches_pixelfx() {
    for (name, f, want) in ease_cases() {
        for (t, w) in EASE_T.iter().zip(want) {
            let got = f(*t);
            assert!(
                (got - w).abs() <= EPS,
                "ease::{name}({t}) = {got}, pixelfx {w}"
            );
        }
    }
}

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
fn loop_noise_seamless() {
    for (radius, seed) in [(1.5, 4.0), (1.0, 0.0), (3.0, 9.0)] {
        let (a, b) = (loop_noise(0.0, radius, seed), loop_noise(1.0, radius, seed));
        assert!(
            (a - b).abs() <= EPS,
            "radius {radius} seed {seed}: {a} vs {b}"
        );
    }
}

#[test]
fn pal_matches_pixelfx() {
    let ramps: [(&str, &[u32], &[u32]); 27] = [
        (
            "fire",
            &pal::FIRE.map(|c| c.pack()),
            &[
                0xff100b1f, 0xff18114a, 0xff1c1d8c, 0xff1a3acc, 0xff1f70f2, 0xff35aeff, 0xff7ae2ff,
                0xffd6f8ff,
            ],
        ),
        (
            "ember",
            &pal::EMBER.map(|c| c.pack()),
            &[
                0xff0e0a1a, 0xff12103d, 0xff141a70, 0xff1a30a8, 0xff1e56e0, 0xff2a8aff, 0xff6ac4ff,
            ],
        ),
        (
            "lava",
            &pal::LAVA.map(|c| c.pack()),
            &[
                0xff0a0614, 0xff100a38, 0xff10126e, 0xff1024ac, 0xff1a50e8, 0xff2a8cff, 0xff5ad2ff,
            ],
        ),
        (
            "smoke",
            &pal::SMOKE.map(|c| c.pack()),
            &[
                0xff120c0e, 0xff221a1d, 0xff352a2e, 0xff4b3e43, 0xff64565c, 0xff84747b, 0xffab9ca3,
            ],
        ),
        (
            "dust",
            &pal::DUST.map(|c| c.pack()),
            &[
                0xff10161c, 0xff1c2833, 0xff2c3f4f, 0xff405a6e, 0xff587a92, 0xff789fb8, 0xffa2c8dc,
            ],
        ),
        (
            "ice",
            &pal::ICE.map(|c| c.pack()),
            &[
                0xff30140c, 0xff5c2c16, 0xff944f1f, 0xffcc7f2f, 0xffecb458, 0xffffdc9b, 0xfffff4d8,
                0xffffffff,
            ],
        ),
        (
            "water",
            &pal::WATER.map(|c| c.pack()),
            &[
                0xff301808, 0xff5a2e0e, 0xff8c5014, 0xffbc761c, 0xffe0a236, 0xfff4cc78, 0xffffecc4,
                0xffffffff,
            ],
        ),
        (
            "electric",
            &pal::ELECTRIC.map(|c| c.pack()),
            &[
                0xff2e0f0a, 0xff782a17, 0xffd05025, 0xffff8c3f, 0xffffc87c, 0xfffff0c8, 0xffffffff,
            ],
        ),
        (
            "volt",
            &pal::VOLT.map(|c| c.pack()),
            &[
                0xff0a1624, 0xff0c3a5e, 0xff0f68a8, 0xff18a2e8, 0xff3cd2ff, 0xff8af0ff, 0xffffffff,
            ],
        ),
        (
            "poison",
            &pal::POISON.map(|c| c.pack()),
            &[
                0xff0c1a0d, 0xff143a17, 0xff185e26, 0xff1e8a3f, 0xff2ab86c, 0xff4ae0a8, 0xff8affe0,
            ],
        ),
        (
            "holy",
            &pal::HOLY.map(|c| c.pack()),
            &[
                0xff081c2a, 0xff103e5a, 0xff186c9a, 0xff28a4d8, 0xff4ad8ff, 0xff9af0ff, 0xffe6fbff,
                0xffffffff,
            ],
        ),
        (
            "heal",
            &pal::HEAL.map(|c| c.pack()),
            &[
                0xff1a2008, 0xff2c3f0f, 0xff3e6617, 0xff4e9224, 0xff66c43e, 0xff8ae87e, 0xffc4ffc8,
                0xffffffff,
            ],
        ),
        (
            "arcane",
            &pal::ARCANE.map(|c| c.pack()),
            &[
                0xff260a14, 0xff52122b, 0xff881c48, 0xffc02c6b, 0xffec489a, 0xffff84c6, 0xffffc8ec,
                0xffffffff,
            ],
        ),
        (
            "shadow",
            &pal::SHADOW.map(|c| c.pack()),
            &[
                0xff080305, 0xff180a0f, 0xff30121c, 0xff4a1a2c, 0xff682443, 0xff90325e, 0xffb84880,
                0xffe070a9,
            ],
        ),
        (
            "blood",
            &pal::BLOOD.map(|c| c.pack()),
            &[
                0xff0a0414, 0xff140a3a, 0xff1c106a, 0xff2418a0, 0xff302ad4, 0xff505aff, 0xff8a9aff,
            ],
        ),
        (
            "earth",
            &pal::EARTH.map(|c| c.pack()),
            &[
                0xff0a0e14, 0xff141e2a, 0xff1e3247, 0xff2a4969, 0xff38668e, 0xff528ab8, 0xff7cb4dc,
            ],
        ),
        (
            "stone",
            &pal::STONE.map(|c| c.pack()),
            &[
                0xff120e0e, 0xff241c1c, 0xff382c2c, 0xff4f4242, 0xff6b5c5c, 0xff8c7d7d, 0xffb2a4a4,
                0xffdad0d0,
            ],
        ),
        (
            "gold",
            &pal::GOLD.map(|c| c.pack()),
            &[
                0xff06121e, 0xff082c4a, 0xff0c5286, 0xff1284c4, 0xff24b4f0, 0xff66e0ff, 0xffc4f6ff,
                0xffffffff,
            ],
        ),
        (
            "leaf",
            &pal::LEAF.map(|c| c.pack()),
            &[
                0xff0e1a0c, 0xff183416, 0xff245222, 0xff307535, 0xff3a9a4f, 0xff48c07c, 0xff70e0b4,
                0xffb0ffe4,
            ],
        ),
        (
            "wood",
            &pal::WOOD.map(|c| c.pack()),
            &[
                0xff060c16, 0xff0c1a2e, 0xff142c4a, 0xff1e426a, 0xff2c5c8e, 0xff407cb4, 0xff64a4d8,
            ],
        ),
        (
            "metal",
            &pal::METAL.map(|c| c.pack()),
            &[
                0xff140e0c, 0xff30221c, 0xff48362e, 0xff68544a, 0xff907a6e, 0xffbaa69a, 0xffe6d8d0,
                0xffffffff,
            ],
        ),
        (
            "bone",
            &pal::BONE.map(|c| c.pack()),
            &[
                0xff14181a, 0xff2c353a, 0xff48565e, 0xff6c808a, 0xff96adb8, 0xffc8dce4, 0xfff0faff,
            ],
        ),
        (
            "neon",
            &pal::NEON.map(|c| c.pack()),
            &[
                0xff16051a, 0xff330a3d, 0xff601570, 0xff8f22b0, 0xffd13cf5, 0xffe686ff, 0xfff6d2ff,
                0xffffffff,
            ],
        ),
        (
            "cyber",
            &pal::CYBER.map(|c| c.pack()),
            &[
                0xff181404, 0xff3a3008, 0xff60520c, 0xff888010, 0xffb0b418, 0xffd8e84c, 0xfff0ffb4,
                0xffffffff,
            ],
        ),
        (
            "matrix",
            &pal::MATRIX.map(|c| c.pack()),
            &[
                0xff040a02, 0xff0c2005, 0xff163c0a, 0xff246610, 0xff349a1a, 0xff50d63c, 0xffa8ffa4,
                0xffe8ffe8,
            ],
        ),
        (
            "candy",
            &pal::CANDY.map(|c| c.pack()),
            &[
                0xff5a4aff, 0xff2a9aff, 0xff3ce0ff, 0xff4ad85a, 0xffffc83c, 0xffff6c7a, 0xffd13cf5,
            ],
        ),
        (
            "night",
            &pal::NIGHT.map(|c| c.pack()),
            &[
                0xff0d0607, 0xff180a0c, 0xff261013, 0xff34171b, 0xff441f24, 0xff56292f,
            ],
        ),
    ];
    for (name, got, want) in ramps {
        assert_eq!(got, want, "pal::{}", name.to_uppercase());
    }
}
