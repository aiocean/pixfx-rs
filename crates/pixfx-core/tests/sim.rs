//! The simulation (pixelfx `src/core/sim.js`): the particle scene, step by step.

/// `(step, particle count, frame hash)` after each checkpoint, printed by `tests/golden/sim_scene.mjs` at pixelfx
/// 044e12a ("particle scene").
const PIXELFX_SCENE: [(u32, usize, u64); 5] = [
    (0, 15, 0xb12d278ab184caf5),
    (12, 21, 0x99f641facd19f32e),
    (60, 45, 0x57a8cc69176e22e8),
    (150, 41, 0x3366d66f50f7fa62),
    (300, 12, 0xd6ab0b5058d65dce),
];

/// The most blazes burning at once in the particle scene (both BRANDs land), printed by `tests/golden/sim_scene.mjs`
/// at pixelfx 044e12a ("most blazes").
const PIXELFX_MOST_BLAZES: usize = 2;

/// A 160x120 frame with a painter placed at (80, 100), as the golden script draws.
fn draw(f: impl FnOnce(&mut pixfx_draw::Painter)) -> u64 {
    let mut frame = pixfx_draw::Frame::new(160, 120, 0, 0);
    let mut p = pixfx_draw::Painter::new(&mut frame);
    p.place(80.0, 100.0, false, 0.0);
    f(&mut p);
    pixfx_draw::frame_hash(&frame)
}

#[test]
fn particle_scene_matches_pixelfx() -> Result<(), pixfx_draw::ColorError> {
    use pixfx_core::Layer;
    use pixfx_core::sim::{Blazes, Flags, Kind, Lights, Particles, ParticlesOpts, Spawn};
    use pixfx_draw::Rng;
    use pixfx_draw::color::hex;

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

    let mut blazes = Blazes::new();
    let mut lights = Lights::new();
    lights.add(0.0, -20.0, 30.0, hex("#ffaa44")?, 0.8, 0.5);

    let dt = 1.0 / 120.0;
    let (mut got, mut most_blazes) = (Vec::new(), 0);
    for step in 0..=300u32 {
        if step > 0 {
            ps.update_with(&mut rng, dt, |rng, x, y| {
                // JS evaluates both ranges before `add` draws its own
                let life = rng.range(0.9, 1.5);
                let power = rng.range(0.55, 0.85);
                blazes.add(rng, x, y - 0.25, life, power);
            });
            blazes.update(&mut ps, &mut rng, dt);
            lights.update(dt);
            most_blazes = most_blazes.max(blazes.count());
        }
        if PIXELFX_SCENE.iter().any(|&(s, _, _)| s == step) {
            let hash = draw(|p| {
                blazes.draw_back(p);
                ps.draw(p, Layer::Back);
                ps.draw(p, Layer::Front);
                lights.draw(p);
            });
            got.push((step, ps.count(), hash));
        }
    }
    assert_eq!(got, PIXELFX_SCENE);
    assert_eq!(most_blazes, PIXELFX_MOST_BLAZES);
    Ok(())
}
