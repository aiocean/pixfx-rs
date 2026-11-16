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
    // seen 0.1 s late, the wind-up that began at 1.0 is shown 0.15 s in at 1.15
    let (f, _) = watch(1.15).seen(1.15, 0.1);
    let Doing::Act { t, phase, .. } = f.doing else {
        unreachable!("{:?}", f.doing)
    };
    assert_eq!(phase, ActPhase::Windup);
    assert!((t - 0.15).abs() < 0.02, "t = {t}");
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

/// A wind-up seen late that has run past its length is a blow already struck: the mind sees its recovery.
#[test]
fn a_windup_seen_late_runs_on_into_recovery() {
    let mut view = Delayed::new();
    let mut f = foe(60.0);
    f.doing = blade_windup(0.15);
    view.push(&sense(1.0, me(40.0), f.clone()));
    // the blow lands at 1.05; at 1.2, seen 0.2 s late, it is 0.15 s into its recovery
    f.doing = Doing::Act {
        kind: SkillKind::Melee,
        phase: ActPhase::Recover,
        t: 0.15,
        windup: 0.2,
        recover: 0.32,
        reach: 24.0,
        damage: 5.0,
        ultimate: false,
    };
    view.push(&sense(1.2, me(40.0), f));
    let (seen, _) = view.seen(1.2, 0.2);
    let Doing::Act { phase, t, .. } = seen.doing else {
        unreachable!("{:?}", seen.doing)
    };
    assert_eq!(phase, ActPhase::Recover);
    assert!((t - 0.15).abs() < 1e-9, "t = {t}");
}

#[test]
fn a_move_seen_late_that_has_ended_is_seen_idle() {
    let mut view = Delayed::new();
    let mut f = foe(60.0);
    f.doing = Doing::Act {
        kind: SkillKind::Melee,
        phase: ActPhase::Recover,
        t: 0.3,
        windup: 0.2,
        recover: 0.32,
        reach: 24.0,
        damage: 5.0,
        ultimate: false,
    };
    view.push(&sense(1.0, me(40.0), f.clone()));
    f.doing = Doing::Idle;
    view.push(&sense(1.2, me(40.0), f));
    assert_eq!(view.seen(1.2, 0.2).0.doing, Doing::Idle);
}

/// What is not a blow coming at me (a cast gathering, a recovery, a walk) is read the moment it shows: it is an
/// opening to take, not a danger to react to.
#[test]
fn a_cast_is_read_the_moment_it_starts() {
    let mut view = Delayed::new();
    view.push(&sense(1.0, me(40.0), foe(150.0)));
    let mut f = foe(150.0);
    f.doing = Doing::Act {
        kind: SkillKind::Area,
        phase: ActPhase::Windup,
        t: 0.02,
        windup: 0.5,
        recover: 0.3,
        reach: 0.0,
        damage: 25.0,
        ultimate: false,
    };
    view.push(&sense(1.02, me(40.0), f.clone()));
    assert_eq!(view.seen(1.02, 0.2).0.doing, f.doing);
}
