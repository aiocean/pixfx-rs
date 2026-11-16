//! Walking a trace back from its newest point (`Trace::steps`).

use pixfx_core::{Trace, TracePoint};

fn p(x: f64, y: f64, age: f64) -> TracePoint {
    TracePoint {
        x,
        y,
        angle: 0.0,
        age,
    }
}

#[test]
fn a_trace_is_walked_a_pixel_at_a_time_round_its_bends_up_to_its_age() {
    // the mover went right along y = 0, then turned down: newest first
    let t = Trace {
        points: vec![p(10.0, 10.0, 0.0), p(10.0, 0.0, 0.1), p(0.0, 0.0, 0.2)],
    };
    let all = t.steps(1.0);
    assert_eq!(all.len(), 21, "{all:?}");
    assert!(
        all.windows(2)
            .all(|w| (w[1].dist - w[0].dist - 1.0).abs() < 1e-9)
    );
    // heading down where it came down, right where it came along
    assert!((all[3].dy - 1.0).abs() < 1e-9 && all[3].dx.abs() < 1e-9);
    assert!((all[15].dx - 1.0).abs() < 1e-9);
    // a younger cut stops halfway down the first leg: a faster mover leaves a longer trace in the same time
    let short = t.steps(0.05);
    assert_eq!(short.len(), 6);
    assert!(short.iter().all(|s| s.age <= 0.05 + 1e-9));
}

#[test]
fn an_empty_or_still_trace_has_no_steps() {
    assert!(Trace::default().steps(1.0).is_empty());
    let still = Trace {
        points: vec![p(3.0, 3.0, 0.0), p(3.0, 3.0, 0.1)],
    };
    assert!(still.steps(1.0).is_empty());
}
