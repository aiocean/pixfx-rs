//! The fluid solvers as an effect drives them: liquid added, stepped at frame rate, read back.

use pixfx_core::fluid::{Floor, Mpm, MpmOpts, Sph, SphOpts};

const FLOOR: Floor = Floor {
    y: 0.0,
    restitution: 0.0,
    friction: 0.05,
};

fn sph() -> Sph {
    let mut s = Sph::new(SphOpts {
        floor: Some(FLOOR),
        bounds: (-200.0, -120.0, 200.0, 40.0),
        ..SphOpts::default()
    });
    s.add_disc(0.0, -30.0, 8.0, 0.0, 0.0, 0);
    s
}

fn mpm(viscosity: f64) -> Mpm {
    let mut m = Mpm::new(MpmOpts {
        origin: (-100.0, -80.0),
        cols: 100,
        rows: 50,
        viscosity,
        floor: Some(FLOOR),
        ..MpmOpts::default()
    });
    m.add_disc(0.0, -30.0, 8.0, 0.0, 0.0, 0);
    m
}

fn run(mut step: impl FnMut(f64), secs: f64) {
    for _ in 0..(secs * 60.0) as usize {
        step(1.0 / 60.0);
    }
}

fn width(xs: impl Iterator<Item = f64> + Clone) -> f64 {
    let lo = xs.clone().fold(f64::INFINITY, f64::min);
    let hi = xs.fold(f64::NEG_INFINITY, f64::max);
    hi - lo
}

#[test]
fn sph_ball_falls_onto_the_floor_and_spreads_into_a_puddle() {
    let mut s = sph();
    let n = s.drops().len();
    run(|dt| s.step(dt), 2.5);
    let d = s.drops();
    assert_eq!(d.len(), n, "nothing leaves the bounds");
    assert!(
        d.iter().all(|p| p.y <= 0.0 && p.y > -40.0),
        "it lies on the floor"
    );
    assert!(
        width(d.iter().map(|p| p.x)) > 24.0,
        "wider than the 16 px ball"
    );
    let mean_x = d.iter().map(|p| p.x).sum::<f64>() / n as f64;
    assert!(
        mean_x.abs() < 1.0,
        "a symmetric ball spreads both ways, mean x {mean_x}"
    );
}

#[test]
fn sph_replays_bit_for_bit_and_holds_still_at_dt_zero() {
    let (mut a, mut b) = (sph(), sph());
    run(|dt| a.step(dt), 1.0);
    run(|dt| b.step(dt), 1.0);
    assert_eq!(a.drops(), b.drops());
    let before = a.drops().to_vec();
    a.step(0.0);
    assert_eq!(a.drops(), &before[..]);
}

#[test]
fn sph_drops_particles_that_leave_the_bounds() {
    let mut s = Sph::new(SphOpts {
        bounds: (-10.0, -10.0, 10.0, 10.0),
        ..SphOpts::default()
    });
    s.add(0.0, 0.0, 600.0, 0.0, 0);
    s.step(0.1);
    assert!(s.drops().is_empty());
}

#[test]
fn mpm_blob_settles_on_the_floor_and_keeps_its_volume() {
    let mut m = mpm(0.0);
    let n = m.particles().len();
    run(|dt| m.step(dt), 3.0);
    let p = m.particles();
    assert_eq!(p.len(), n);
    // the grid holds the floor to within a cell (2 px)
    assert!(
        p.iter().all(|q| q.y <= 2.0),
        "nothing sinks through the floor"
    );
    assert!(width(p.iter().map(|q| q.x)) > 20.0, "it spreads");
    let mean_j = p.iter().map(|q| q.j).sum::<f64>() / n as f64;
    assert!(
        (0.85..1.15).contains(&mean_j),
        "volume kept, mean J {mean_j}"
    );
    let speed = p.iter().map(|q| libm::hypot(q.vx, q.vy)).sum::<f64>() / n as f64;
    assert!(speed < 10.0, "settled, mean speed {speed}");
}

#[test]
fn mpm_thick_liquid_spreads_less_than_water() {
    let (mut water, mut blood) = (mpm(0.0), mpm(6.0));
    run(|dt| water.step(dt), 0.8);
    run(|dt| blood.step(dt), 0.8);
    let ww = width(water.particles().iter().map(|q| q.x));
    let wb = width(blood.particles().iter().map(|q| q.x));
    assert!(wb < ww, "blood {wb} px wide, water {ww} px");
}

#[test]
fn mpm_replays_bit_for_bit_and_holds_still_at_dt_zero() {
    let (mut a, mut b) = (mpm(2.0), mpm(2.0));
    run(|dt| a.step(dt), 1.0);
    run(|dt| b.step(dt), 1.0);
    assert_eq!(a.particles(), b.particles());
    let before = a.particles().to_vec();
    a.step(0.0);
    assert_eq!(a.particles(), &before[..]);
}

fn sph_spread(max_dt: f64) -> f64 {
    let mut s = Sph::new(SphOpts {
        viscosity: 0.6,
        sound: 600.0,
        max_dt,
        floor: Some(FLOOR),
        bounds: (-200.0, -120.0, 200.0, 40.0),
        ..SphOpts::default()
    });
    s.add_disc(0.0, -30.0, 8.0, 0.0, 0.0, 0);
    run(|dt| s.step(dt), 0.5);
    width(s.drops().iter().map(|p| p.x))
}

#[test]
fn sph_viscosity_does_not_depend_on_the_sub_step() {
    let (coarse, fine) = (sph_spread(1.0 / 600.0), sph_spread(1.0 / 2400.0));
    assert!(
        (coarse - fine).abs() < 0.1 * coarse,
        "spread {coarse} px at 1/600 s, {fine} px at 1/2400 s"
    );
}

#[test]
fn sph_surface_tension_pulls_two_drops_together() {
    let gap = |tension: f64| {
        let mut s = Sph::new(SphOpts {
            tension,
            gravity: 0.0,
            ..SphOpts::default()
        });
        // edges 2 px apart: inside each other's 4 px reach, not squeezed
        s.add_disc(-3.0, 0.0, 2.0, 0.0, 0.0, 0);
        s.add_disc(3.0, 0.0, 2.0, 0.0, 0.0, 1);
        run(|dt| s.step(dt), 0.5);
        let mean = |tag| {
            let xs: Vec<f64> = s
                .drops()
                .iter()
                .filter(|d| d.tag == tag)
                .map(|d| d.x)
                .collect();
            xs.iter().sum::<f64>() / xs.len() as f64
        };
        mean(1) - mean(0)
    };
    let (loose, held) = (gap(0.0), gap(600.0));
    assert!(
        held < loose - 1.0,
        "with tension {held} px apart, without {loose} px"
    );
}

#[test]
fn mpm_liquid_rests_on_the_floor_without_bouncing_higher_than_it_fell() {
    let mut m = mpm(0.0);
    let start_top = m.particles().iter().map(|q| q.y).fold(f64::MAX, f64::min);
    let mut highest_after = f64::MAX;
    for f in 0..180 {
        m.step(1.0 / 60.0);
        if f > 40 {
            let top = m.particles().iter().map(|q| q.y).fold(f64::MAX, f64::min);
            highest_after = highest_after.min(top);
        }
    }
    assert!(
        highest_after > start_top + 10.0,
        "rose to {highest_after}, fell from {start_top}"
    );
    let lowest = m.particles().iter().map(|q| q.y).fold(f64::MIN, f64::max);
    assert!(
        lowest > -1.0,
        "the pool lies on the floor, lowest particle at {lowest}"
    );
}
