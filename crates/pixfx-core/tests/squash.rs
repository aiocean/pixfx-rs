//! A body's squash, stretch and blink, driven the way a host drives them: it moves, lands, time passes.

use pixfx_core::squash::{Blink, Shape, Squash, SquashSpec};

const FRAME: f64 = 1.0 / 60.0;

fn close(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-9
}

fn area(s: Shape) -> f64 {
    s.a * s.d - s.b * s.c
}

/// How long a unit step along `(x, y)` is drawn.
fn length_along(s: Shape, x: f64, y: f64) -> f64 {
    let n = libm::hypot(x, y);
    let (x, y) = (x / n, y / n);
    libm::hypot(s.a * x + s.c * y, s.b * x + s.d * y)
}

#[test]
fn a_body_stretches_along_its_velocity_with_its_speed_and_keeps_its_area() {
    let s = Squash::new(SquashSpec::default());
    assert_eq!(s.shape(0.0, 0.0), Shape::IDENTITY, "at rest it is itself");

    let up = s.shape(0.0, -220.0);
    assert!(close(up.d, 1.3) && close(up.a, 1.0 / 1.3), "rising: {up:?}");
    assert_eq!(s.shape(0.0, 220.0), up, "falling stretches as rising does");
    assert!(close(s.shape(0.0, 5000.0).d, 1.3), "no longer than 1.3");
    assert!(s.shape(0.0, 110.0).d > 1.0 && s.shape(0.0, 110.0).d < 1.3);

    let dash = s.shape(220.0, 0.0);
    assert!(
        close(dash.a, 1.3) && close(dash.d, 1.0 / 1.3),
        "a dash: {dash:?}"
    );

    let leap = s.shape(200.0, -200.0);
    assert!(
        close(length_along(leap, 1.0, -1.0), 1.3),
        "long along the leap"
    );
    assert!(
        close(length_along(leap, 1.0, 1.0), 1.0 / 1.3),
        "thin across it"
    );
    for shape in [up, dash, leap, s.shape(37.0, 91.0)] {
        assert!(close(area(shape), 1.0), "area kept: {shape:?}");
    }
    assert_eq!(s.shape(f64::NAN, 3.0), Shape::IDENTITY);
}

#[test]
fn the_feet_stay_put_and_a_drawn_point_maps_back_to_its_source() {
    let leap = Squash::new(SquashSpec::default()).shape(120.0, -200.0);
    let (x, y) = (3.0, -17.0);
    let drawn = (leap.a * x + leap.c * y, leap.b * x + leap.d * y);
    let back = leap.source(drawn.0, drawn.1);
    assert!(close(back.0, x) && close(back.1, y), "{back:?}");
    assert_eq!(leap.source(0.0, 0.0), (0.0, 0.0), "the pivot is the feet");
}

#[test]
fn a_landing_flattens_it_to_0_7_harder_the_harder_it_lands_and_eases_back_within_0_15_s() {
    let mut hard = Squash::new(SquashSpec::default());
    let mut soft = hard;
    hard.land(1.0);
    soft.land(0.3);
    assert!(
        close(hard.shape(0.0, 0.0).d, 0.7),
        "flattest on the frame it lands"
    );
    assert!(
        close(hard.shape(0.0, 0.0).a, 1.0 / 0.7),
        "and as much wider"
    );
    assert!(soft.shape(0.0, 0.0).d > hard.shape(0.0, 0.0).d);
    assert_eq!(
        hard.shape(300.0, 300.0).b,
        0.0,
        "a landing squashes upright whatever the speed"
    );
    let mut last = hard.shape(0.0, 0.0).d;
    for _ in 0..8 {
        hard.advance(FRAME);
        let now = hard.shape(0.0, 0.0).d;
        assert!(now >= last && now < 1.0, "eases back: {now} after {last}");
        last = now;
    }
    hard.advance(0.15 - 8.0 * FRAME);
    assert_eq!(hard.shape(0.0, 0.0), Shape::IDENTITY, "itself 0.15 s on");
    assert!(
        close(hard.shape(0.0, 220.0).d, 1.3),
        "and stretches again as it moves"
    );
}

#[test]
fn a_host_that_cannot_skew_gets_the_nearest_upright_scale_with_the_area_kept() {
    let s = Squash::new(SquashSpec::default());
    let (sx, sy) = s.shape(0.0, 220.0).axis_scale();
    assert!(close(sx, 1.0 / 1.3) && close(sy, 1.3), "rising: tall");
    let (sx, sy) = s.shape(220.0, 0.0).axis_scale();
    assert!(close(sx, 1.3) && close(sy, 1.0 / 1.3), "dashing: wide");
    let (sx, sy) = s.shape(200.0, 80.0).axis_scale();
    assert!(
        sx > 1.0 && sx < 1.3 && close(sx * sy, 1.0),
        "mostly sideways: {sx} x {sy}"
    );
    let (sx, sy) = s.shape(150.0, 150.0).axis_scale();
    assert!(
        close(sx, 1.0) && close(sy, 1.0),
        "an exact diagonal has no upright stretch"
    );
}

#[test]
fn a_blink_shows_faint_on_every_other_beat_while_it_lasts() {
    let mut b = Blink::new(SquashSpec::default());
    assert_eq!(b.alpha(), 1.0);
    b.start(0.5);
    let mut seen = Vec::new();
    for _ in 0..30 {
        b.advance(FRAME);
        seen.push(b.alpha());
    }
    assert!(seen.contains(&1.0) && seen.contains(&0.35), "{seen:?}");
    b.advance(0.3);
    assert!(!b.active());
    assert_eq!(b.alpha(), 1.0, "whole once it ends");
}

#[test]
fn a_moving_body_holds_its_scale_through_a_step_with_no_time() {
    let mut m = pixfx_core::Moving::new(SquashSpec::default());
    m.step(1.0 / 60.0, 0.0, 0.0, None);
    let fast = m.step(1.0 / 60.0, 6.0, 0.0, None);
    assert_ne!(fast, (1.0, 1.0), "360 px/s stretches it");
    // a hit-stop: no time passes and the body may even be moved; nothing spikes
    assert_eq!(m.step(0.0, 500.0, 0.0, None), fast);
    assert_eq!(m.step(-1.0, 500.0, 0.0, None), fast);
}

#[test]
fn a_moving_body_without_velocity_reads_it_from_how_far_it_moved() {
    let spec = SquashSpec::default();
    let mut moved = pixfx_core::Moving::new(spec);
    let mut told = pixfx_core::Moving::new(spec);
    moved.step(0.1, 10.0, 10.0, None);
    told.step(0.1, 0.0, 0.0, Some((0.0, 0.0)));
    // 30 px down in 0.1 s is 300 px/s
    assert_eq!(
        moved.step(0.1, 10.0, 40.0, None),
        told.step(0.1, 0.0, 0.0, Some((0.0, 300.0)))
    );
    // a velocity the game gives wins over the distance
    assert_eq!(
        moved.step(0.1, 10.0, 40.0, Some((0.0, 300.0))),
        told.step(0.1, 0.0, 0.0, Some((0.0, 300.0)))
    );
}
