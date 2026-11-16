//! Motion read off a trace (`pixfx_core::motion`): one way every effect that moves works out how it moves.

use pixfx_core::{Trace, TracePoint};

fn p(x: f64, y: f64, angle: f64, age: f64) -> TracePoint {
    TracePoint { x, y, angle, age }
}

/// Points every 0.01 s for `secs`, newest first, where `at(age)` puts them.
fn trace(secs: f64, at: impl Fn(f64) -> (f64, f64, f64)) -> Trace {
    let n = (secs / 0.01) as usize;
    Trace {
        points: (0..=n)
            .map(|i| {
                let age = i as f64 * 0.01;
                let (x, y, a) = at(age);
                p(x, y, a, age)
            })
            .collect(),
    }
}

fn near(a: f64, b: f64, e: f64) -> bool {
    (a - b).abs() <= e
}

#[test]
fn a_straight_trace_has_its_speed_and_heading() {
    // 200 px/s along +x: 2 px back every 0.01 s
    let t = trace(0.5, |age| (-200.0 * age, 0.0, 0.0));
    for age in [0.0, 0.1, 0.3] {
        assert!(
            near(t.speed(age, 0.1), 200.0, 1e-6),
            "speed at {age}: {}",
            t.speed(age, 0.1)
        );
        let h = t.heading(age, 0.1).unwrap_or(f64::NAN);
        assert!(near(h, 0.0, 1e-9), "heading at {age}: {h}");
    }
}

#[test]
fn a_circle_turns_at_a_constant_rate() {
    // round a circle of 20 px at 2 rad/s, counter-clockwise on screen (angle grows)
    let t = trace(1.0, |age| {
        let a = -2.0 * age;
        (20.0 * libm::cos(a), 20.0 * libm::sin(a), a)
    });
    for age in [0.1, 0.4, 0.7] {
        let turn = t.turn(age, 0.1);
        assert!(near(turn, 2.0, 0.05), "turn at {age}: {turn}");
        let spin = t.spin(age, 0.1);
        assert!(near(spin, 2.0, 0.05), "spin at {age}: {spin}");
        assert!(near(t.speed(age, 0.1), 40.0, 0.5));
    }
}

#[test]
fn a_still_trace_has_no_motion() {
    let t = trace(0.3, |_| (5.0, 5.0, 0.0));
    assert_eq!(t.velocity(0.0, 0.1), (0.0, 0.0));
    assert_eq!(t.heading(0.0, 0.1), None);
    assert_eq!(t.turn(0.1, 0.1), 0.0);
    assert_eq!(t.acceleration(0.1, 0.1), (0.0, 0.0));
    let one = Trace {
        points: vec![p(1.0, 2.0, 0.0, 0.0)],
    };
    assert_eq!(one.velocity(0.0, 0.1), (0.0, 0.0));
    assert_eq!(Trace::default().at(0.0), None);
}

#[test]
fn same_age_points_give_no_velocity() {
    // a hit-stop: time did not pass between the two points
    let t = Trace {
        points: vec![p(10.0, 0.0, 0.0, 0.0), p(0.0, 0.0, 0.0, 0.0)],
    };
    let v = t.velocity(0.0, 0.1);
    assert!(v.0.is_finite() && v.1.is_finite(), "{v:?}");
    assert_eq!(v, (0.0, 0.0));
}

#[test]
fn at_interpolates_between_points_and_holds_past_the_ends() {
    let t = Trace {
        points: vec![p(10.0, 0.0, 1.0, 0.0), p(0.0, 4.0, 0.0, 0.2)],
    };
    let (x, y, a) = t.at(0.1).unwrap_or((f64::NAN, f64::NAN, f64::NAN));
    assert!(near(x, 5.0, 1e-9) && near(y, 2.0, 1e-9) && near(a, 0.5, 1e-9));
    assert_eq!(t.at(5.0), Some((0.0, 4.0, 0.0)));
    assert_eq!(t.at(-1.0), Some((10.0, 0.0, 1.0)));
}

#[test]
fn poses_end_where_the_age_does() {
    let t = Trace {
        points: vec![
            p(0.0, 0.0, 0.0, 0.0),
            p(-10.0, 0.0, 0.0, 0.1),
            p(-20.0, 0.0, 0.0, 0.2),
        ],
    };
    let poses = t.poses(0.15);
    let got: Vec<(f64, f64)> = poses.iter().map(|q| (q.x, q.age)).collect();
    assert_eq!(got.len(), 3);
    assert!(
        near(got[2].0, -15.0, 1e-9) && near(got[2].1, 0.15, 1e-9),
        "{got:?}"
    );
    assert_eq!(t.poses(1.0).len(), 3);
}

#[test]
fn acceleration_reads_a_speeding_mover() {
    // x = 50 t^2 forward in time: accelerating at 100 px/s^2 along +x
    let t = trace(1.0, |age| {
        let time = 1.0 - age;
        (50.0 * time * time, 0.0, 0.0)
    });
    let (ax, ay) = t.acceleration(0.5, 0.1);
    assert!(near(ax, 100.0, 1.0) && near(ay, 0.0, 1e-9), "{ax} {ay}");
}
