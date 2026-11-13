//! Simulation constants (pixelfx `src/core/sim.js`) the fire effects use.

use pixfx_core::sim::SMOKE_RAMP;

/// pixelfx `SMOKE_RAMP` as packed words, printed by `tests/golden/smoke_ramp.mjs` at pixelfx 044e12a.
const PIXELFX_SMOKE_RAMP: [u32; 5] = [0xff726f7b, 0xff60565d, 0xff483d43, 0xff30262b, 0xff1e171a];

#[test]
fn smoke_ramp_matches_pixelfx() {
    let packed: Vec<u32> = SMOKE_RAMP.iter().map(|c| c.pack()).collect();
    assert_eq!(packed, PIXELFX_SMOKE_RAMP);
}

/// `(step, count, frame hash)` after each checkpoint, printed by `tests/golden/sim_scene.mjs` at pixelfx 044e12a.
const PIXELFX_SCENE: [(u32, usize, u64); 5] = [
    (0, 15, 0xc2934ea98445957e),
    (12, 21, 0x709b9536dc00eb7a),
    (60, 43, 0x6a4feeea2b346fc8),
    (150, 24, 0x1fff72027a3d285b),
    (300, 0, 0x1100fdb97cd50325),
];

#[test]
fn particle_scene_matches_pixelfx() -> Result<(), pixfx_core::ColorError> {
    use pixfx_core::color::hex;
    use pixfx_core::sim::{Flags, Kind, Particles, ParticlesOpts, Spawn};
    use pixfx_core::{Frame, Layer, Painter, Rng, frame_hash};

    // pixelfx `P` numbers 0..=11, in order
    const KINDS: [Kind; 12] = [
        Kind::Spark,
        Kind::Mote,
        Kind::Streak,
        Kind::Flake,
        Kind::Vapour,
        Kind::Dust,
        Kind::Star,
        Kind::Ember,
        Kind::Smoke,
        Kind::Fire,
        Kind::Brand,
        Kind::Grit,
    ];
    let gold = hex("#ffcc66")?;
    let mut rng = Rng::new(5);
    let mut ps = Particles::new(ParticlesOpts {
        ground: 0.0,
        haze: Some(hex("#88ccff")?),
        ..ParticlesOpts::default()
    });
    for (i, kind) in KINDS.into_iter().enumerate() {
        let k = i as f64;
        let spawn = Spawn {
            kind,
            x: (k - 6.0) * 8.0,
            y: -20.0,
            vx: rng.range(-80.0, 80.0),
            vy: rng.range(-160.0, -40.0),
            life: rng.range(0.6, 1.4),
            col: if matches!(kind, Kind::Smoke | Kind::Fire) {
                None
            } else {
                Some(gold)
            },
            size: rng.range(0.8, 2.6),
            delay: k * 0.02,
            flags: match kind {
                Kind::Fire => Flags::SMOKES,
                Kind::Smoke => Flags::BIG,
                _ => Flags::NONE,
            },
        };
        ps.spawn(&mut rng, spawn);
    }
    ps.spawn(&mut rng, Spawn::new(Kind::Mote, 0.0, -10.0, 0.0, 0.0, 1.0));
    ps.spawn(
        &mut rng,
        Spawn {
            flags: Flags::BACK,
            ..Spawn::new(Kind::Spark, 4.0, -30.0, 50.0, -50.0, 1.0)
        },
    );
    ps.spawn(
        &mut rng,
        Spawn::new(Kind::Brand, -10.0, -40.0, 30.0, -80.0, 2.0),
    );

    let mut got = Vec::new();
    for step in 0..=300u32 {
        if step > 0 {
            ps.update(&mut rng, 1.0 / 120.0);
        }
        if PIXELFX_SCENE.iter().any(|&(s, _, _)| s == step) {
            let mut frame = Frame::new(160, 120, 0, 0);
            let mut p = Painter::new(&mut frame);
            p.place(80.0, 100.0, false, 0.0);
            ps.draw(&mut p, Layer::Back);
            ps.draw(&mut p, Layer::Front);
            got.push((step, ps.count(), frame_hash(&frame)));
        }
    }
    assert_eq!(got, PIXELFX_SCENE);
    Ok(())
}
