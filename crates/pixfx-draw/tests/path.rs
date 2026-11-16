//! A path read by distance along it: lines, curves, turns and spirals joined end to end.

use core::f64::consts::{PI, TAU};

use pixfx_draw::path::Path;

fn near(a: (f64, f64), b: (f64, f64), eps: f64) -> bool {
    libm::hypot(a.0 - b.0, a.1 - b.1) <= eps
}

#[test]
fn a_line_is_read_by_pixels_along_it() {
    let p = Path::line((0.0, 0.0), (30.0, 40.0));
    assert_eq!(p.length(), 50.0);
    assert!(near(p.at(25.0), (15.0, 20.0), 1e-9));
    assert!(near(p.at(-5.0), (0.0, 0.0), 1e-9));
    assert!(near(p.at(80.0), (30.0, 40.0), 1e-9));
    assert!((p.heading(10.0) - libm::atan2(40.0, 30.0)).abs() < 1e-12);
}

#[test]
fn a_turn_goes_round_its_centre_at_its_radius() {
    // from (10, 0) once round the origin, clockwise on screen (y down)
    let p = Path::begin((10.0, 0.0)).turn((0.0, 0.0), TAU);
    assert!((p.length() - TAU * 10.0).abs() < 1e-9);
    // a quarter of the way round it is below the centre, heading left
    let q = p.at(p.length() / 4.0);
    assert!(near(q, (0.0, 10.0), 1e-9), "{q:?}");
    assert!((p.heading(p.length() / 4.0) - PI).abs() < 1e-9);
    // and back where it began
    assert!(near(p.end(), (10.0, 0.0), 1e-9));
    for k in 0..=40 {
        let at = p.at(p.length() * f64::from(k) / 40.0);
        assert!((libm::hypot(at.0, at.1) - 10.0).abs() < 1e-9);
    }
}

#[test]
fn a_negative_sweep_turns_the_other_way() {
    let p = Path::begin((10.0, 0.0)).turn((0.0, 0.0), -TAU / 4.0);
    assert!(near(p.end(), (0.0, -10.0), 1e-9));
}

#[test]
fn a_spiral_closes_in_on_its_centre() {
    let p = Path::begin((20.0, 0.0)).spiral((0.0, 0.0), 2.0 * TAU, 2.0);
    assert!(near(p.end(), (2.0, 0.0), 1e-9));
    let mut last = f64::INFINITY;
    for k in 0..=50 {
        let at = p.at(p.length() * f64::from(k) / 50.0);
        let r = libm::hypot(at.0, at.1);
        assert!(r <= last + 1e-9, "the radius only shrinks");
        last = r;
    }
    // longer than a circle at its inner radius twice round, shorter than at its outer
    assert!(p.length() > 2.0 * TAU * 2.0 && p.length() < 2.0 * TAU * 20.0);
}

#[test]
fn pieces_join_end_to_end_and_steps_along_them_are_even() {
    let p = Path::begin((0.0, 0.0))
        .line_to((20.0, 0.0))
        .turn((20.0, 10.0), TAU)
        .quad_to((40.0, 0.0), (60.0, 30.0));
    assert!(near(p.at(20.0), (20.0, 0.0), 1e-9));
    assert!(near(p.end(), (60.0, 30.0), 1e-9));
    // a steady mover covers about a pixel per pixel of distance on the straight, the turn and the curve
    let n = 400;
    let mut last = p.at(0.0);
    for k in 1..=n {
        let at = p.at(p.length() * f64::from(k) / f64::from(n));
        let step = libm::hypot(at.0 - last.0, at.1 - last.1);
        assert!(
            (step - p.length() / f64::from(n)).abs() < 0.02,
            "uneven step {step} at {k}"
        );
        last = at;
    }
}

#[test]
fn points_every_step_include_both_ends() {
    let p = Path::begin((5.0, 0.0)).turn((0.0, 0.0), PI);
    let pts = p.points(1.0);
    assert!(near(
        *pts.first().unwrap_or(&(f64::NAN, 0.0)),
        (5.0, 0.0),
        1e-9
    ));
    assert!(near(
        *pts.last().unwrap_or(&(f64::NAN, 0.0)),
        (-5.0, 0.0),
        1e-9
    ));
    assert!(pts.len() >= 16);
}

#[test]
fn a_path_that_goes_nowhere_stays_at_its_start() {
    let p = Path::begin((3.0, 4.0));
    assert_eq!(p.length(), 0.0);
    assert_eq!(p.at(10.0), (3.0, 4.0));
    assert_eq!(p.heading(10.0), 0.0);
    assert_eq!(Path::through(&[]).length(), 0.0);
    assert!((Path::through(&[(0.0, 0.0), (3.0, 4.0), (3.0, 10.0)]).length() - 11.0).abs() < 1e-12);
}
