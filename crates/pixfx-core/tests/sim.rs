//! The simulation (pixelfx `src/core/sim.js`): particles, blazes, lights, debris and the sim shapes.

use pixfx_core::sim::SMOKE_RAMP;

/// pixelfx `SMOKE_RAMP` as packed words, printed by `tests/golden/smoke_ramp.mjs` at pixelfx 044e12a.
const PIXELFX_SMOKE_RAMP: [u32; 5] = [0xff726f7b, 0xff60565d, 0xff483d43, 0xff30262b, 0xff1e171a];

#[test]
fn smoke_ramp_matches_pixelfx() {
    let packed: Vec<u32> = SMOKE_RAMP.iter().map(|c| c.pack()).collect();
    assert_eq!(packed, PIXELFX_SMOKE_RAMP);
}

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

/// `(step, bit count, frame hash)`, printed by `tests/golden/sim_scene.mjs` at pixelfx 044e12a ("debris scene").
/// By step 120 the SHARD and the CHUNK rest on the floor.
const PIXELFX_DEBRIS: [(u32, usize, u64); 4] = [
    (0, 3, 0xc5696d57ba06f965),
    (60, 2, 0x4ef7b7d88fd10ce0),
    (120, 2, 0x0aa2499a39f58680),
    (240, 2, 0x1100fdb97cd50325),
];

/// Frame hash of the sim shapes, printed by `tests/golden/sim_scene.mjs` at pixelfx 044e12a ("shapes").
const PIXELFX_SHAPES: u64 = 0xe3daf26d1c5ebf77;

/// A 160x120 frame with a painter placed at (80, 100), as the golden script draws.
fn draw(f: impl FnOnce(&mut pixfx_core::Painter)) -> u64 {
    let mut frame = pixfx_core::Frame::new(160, 120, 0, 0);
    let mut p = pixfx_core::Painter::new(&mut frame);
    p.place(80.0, 100.0, false, 0.0);
    f(&mut p);
    pixfx_core::frame_hash(&frame)
}

#[test]
fn particle_scene_matches_pixelfx() -> Result<(), pixfx_core::ColorError> {
    use pixfx_core::color::hex;
    use pixfx_core::sim::{Blazes, Flags, Kind, Lights, Particles, ParticlesOpts, Spawn};
    use pixfx_core::{Layer, Rng};

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

#[test]
fn debris_scene_matches_pixelfx() -> Result<(), pixfx_core::ColorError> {
    use pixfx_core::color::hex;
    use pixfx_core::sim::{Bit, BitKind, Bits};

    let mut bits: Bits = Bits::new(0.0);
    bits.add(Bit {
        vx: 60.0,
        vy: -120.0,
        va: 9.0,
        s: 5.0,
        c: hex("#d8f4ff")?,
        bounce: 0.4,
        life: 2.0,
        ..Bit::new(BitKind::Shard, -20.0, -30.0)
    });
    bits.add(Bit {
        vx: -40.0,
        vy: -90.0,
        va: -5.0,
        s: 3.0,
        bounce: 0.3,
        drag: 0.5,
        life: 2.0,
        ..Bit::new(BitKind::Chunk, 0.0, -40.0)
    });
    bits.add(Bit {
        c: hex("#58b4ec")?,
        g: 0.5,
        life: 2.0,
        ..Bit::new(BitKind::Drip, 20.0, -50.0)
    });

    let mut got = Vec::new();
    for step in 0..=240u32 {
        if step > 0 {
            bits.update(1.0 / 120.0);
        }
        if PIXELFX_DEBRIS.iter().any(|&(s, _, _)| s == step) {
            got.push((step, bits.count(), draw(|p| bits.draw(p))));
        }
    }
    assert_eq!(got, PIXELFX_DEBRIS);
    Ok(())
}

#[test]
fn sim_shapes_match_pixelfx() -> Result<(), pixfx_core::ColorError> {
    use pixfx_core::color::hex;
    use pixfx_core::sim::{CrystalOpts, band, crystal, flare, sparkle};

    let ramp = |l: &[&str]| l.iter().map(|s| hex(s)).collect::<Result<Vec<_>, _>>();
    let r5 = ramp(&["#0c1430", "#1f4f94", "#58b4ec", "#d8f4ff", "#ffffff"])?;
    let r7 = ramp(&[
        "#0c1430", "#162c5c", "#1f4f94", "#2f7fcc", "#58b4ec", "#9bdcff", "#d8f4ff",
    ])?;
    let (gold, tip) = (hex("#ffe066")?, hex("#b07a1a")?);
    let hash = draw(|p| {
        sparkle(p, -40.0, -40.0, 5.0, gold, tip, 0.6);
        flare(p, 0.0, -60.0, 12.0, 0.7, &r5);
        band(
            p,
            30.0,
            -30.0,
            18.0,
            0.4,
            3.0,
            0.9,
            &r5,
            |g| libm::sin(g) < 0.0,
            true,
        );
        crystal(
            p,
            -20.0,
            -5.0,
            24.0,
            10.0,
            3.0,
            &r7,
            CrystalOpts {
                glint: 0.5,
                ..CrystalOpts::default()
            },
        );
    });
    assert_eq!(hash, PIXELFX_SHAPES);
    Ok(())
}

#[test]
fn draw_where_draws_the_system_minus_the_dropped_particles() {
    use pixfx_core::Layer;
    use pixfx_core::Rng;
    use pixfx_core::color::WHITE;
    use pixfx_core::sim::{Kind, Particles, ParticlesOpts, Spawn};

    // a mix of front kinds (spark, grit, mote) and a back kind (smoke), some overlapping
    let mut rng = Rng::new(3);
    let mut ps = Particles::new(ParticlesOpts {
        ground: 0.0,
        ..ParticlesOpts::default()
    });
    for i in 0..24 {
        let kind = [Kind::Spark, Kind::Grit, Kind::Smoke, Kind::Mote][i % 4];
        let (x, y) = (rng.range(-40.0, 40.0), rng.range(-60.0, -5.0));
        let (vx, vy) = (rng.range(-90.0, 90.0), rng.range(-120.0, 20.0));
        let size = rng.range(1.0, 6.0);
        ps.spawn(
            &mut rng,
            Spawn {
                size,
                col: if kind == Kind::Smoke {
                    None
                } else {
                    Some(WHITE)
                },
                ..Spawn::new(kind, x, y, vx, vy, 1.5)
            },
        );
    }
    for _ in 0..20 {
        ps.update(&mut rng, 1.0 / 120.0);
    }

    for (layer, dropped) in [
        (Layer::Front, Kind::Grit),
        (Layer::Both, Kind::Spark),
        (Layer::Back, Kind::Smoke),
    ] {
        let mut rest = ps.clone();
        rest.list_mut().retain(|q| q.kind != dropped);
        assert!(rest.count() < ps.count(), "{dropped:?} not in the scene");
        let filtered = draw(|p| ps.draw_where(p, layer, |q| q.kind != dropped));
        let expected = draw(|p| rest.draw(p, layer));
        let everything = draw(|p| ps.draw(p, layer));
        assert_eq!(filtered, expected, "{layer:?} without {dropped:?}");
        assert_ne!(
            filtered, everything,
            "dropping {dropped:?} changed nothing on {layer:?}"
        );
    }
}

#[test]
fn set_time_moves_the_twinkle_of_a_drawn_star() {
    use pixfx_core::Layer;
    use pixfx_core::Rng;
    use pixfx_core::color::WHITE;
    use pixfx_core::sim::{Kind, Particles, ParticlesOpts, Spawn};

    let star_at = |t: f64| {
        let mut ps = Particles::new(ParticlesOpts::default());
        ps.spawn(
            &mut Rng::new(1),
            Spawn {
                size: 0.5,
                col: Some(WHITE),
                ..Spawn::new(Kind::Star, 0.0, -30.0, 0.0, 0.0, 1.0)
            },
        );
        ps.set_time(t);
        draw(|p| ps.draw(p, Layer::Both))
    };
    assert_eq!(star_at(0.37), star_at(0.37));
    assert_ne!(star_at(0.0), star_at(0.37));
}
