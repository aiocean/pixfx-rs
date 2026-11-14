//! The foe seen late: what it does reaches the mind `reaction` seconds after it starts; where it is, at once.

mod common;
use common::*;
use pixfx_mind::*;

/// Senses every 1/60 s from 0 to `until`; the foe starts a blade at t = 1.0.
fn watch(until: f64) -> Delayed {
    let mut view = Delayed::new();
    let mut t = 0.0;
    while t <= until + 1e-9 {
        let mut f = foe(60.0);
        if t >= 1.0 {
            f.doing = blade_windup(t - 1.0);
        }
        view.push(&sense(t, me(40.0), f));
        t += 1.0 / 60.0;
    }
    view
}

#[test]
fn a_windup_is_seen_after_reaction() {
    let (early, _) = watch(1.1).seen(1.1, 0.2);
    assert_eq!(early.doing, Doing::Idle);
    let (late, _) = watch(1.25).seen(1.25, 0.2);
    assert!(matches!(late.doing, Doing::Act { .. }));
}

#[test]
fn what_is_seen_late_is_aged_to_now() {
    // seen 0.2 s late, the wind-up that began at 1.0 is shown 0.3 s in at 1.3
    let (f, _) = watch(1.3).seen(1.3, 0.2);
    let Doing::Act { t, .. } = f.doing else {
        unreachable!("{:?}", f.doing)
    };
    assert!((t - 0.3).abs() < 0.02, "t = {t}");
}

#[test]
fn where_the_foe_is_is_seen_now() {
    let mut view = Delayed::new();
    view.push(&sense(0.0, me(40.0), foe(60.0)));
    view.push(&sense(0.1, me(40.0), foe(90.0)));
    let (f, _) = view.seen(0.1, 0.2);
    assert_eq!(f.x, 90.0);
}

#[test]
fn a_young_threat_is_hidden() {
    let mut view = Delayed::new();
    let mut s = sense(1.0, me(40.0), foe(60.0));
    let bolt = |age| Threat {
        age,
        damage: 13.0,
        shape: ThreatShape::Projectile {
            x: 80.0,
            y: 14.0,
            vx: -230.0,
        },
    };
    s.threats = vec![bolt(0.1), bolt(0.3)];
    view.push(&s);
    let (_, threats) = view.seen(1.0, 0.2);
    assert_eq!(threats, vec![bolt(0.3)]);
}

#[test]
fn dt_zero_does_not_age() {
    let mut view = Delayed::new();
    view.push(&sense(1.0, me(40.0), foe(60.0)));
    view.push(&sense(1.0, me(40.0), foe(60.0)));
    assert_eq!(view.len(), 1);
}

#[test]
fn clear_forgets() {
    let mut view = watch(1.3);
    view.clear();
    assert_eq!(view.len(), 0);
}
