//! What the policy sees: every value finite and bounded, its tag at the end, at most four threats.

mod common;

use common::*;
use pixfx_mind::policy::{OBS, TAG, observe};
use pixfx_mind::*;

fn slots() -> Vec<Option<usize>> {
    vec![Some(0), None, Some(2), Some(1), Some(3), None, Some(6)]
}

fn bolt_at(x: f64) -> Threat {
    Threat {
        age: 0.5,
        damage: 13.0,
        shape: ThreatShape::Projectile {
            x,
            y: 20.0,
            vx: -230.0,
        },
    }
}

#[test]
fn every_observation_value_is_finite_and_bounded() {
    let kit = kit();
    let tag = [1.0, 0.0, 0.0, 1.0, 0.0];
    let doings = [
        Doing::Idle,
        Doing::Move { dir: -1.0 },
        Doing::Guard,
        blade_windup(0.1),
        Doing::Act {
            kind: SkillKind::Blink,
            phase: ActPhase::Recover,
            t: 9.0,
            windup: 0.0,
            recover: 0.0,
            reach: f64::NAN,
            damage: 1e9,
        },
    ];
    for (mx, fx) in [(0.0, 220.0), (220.0, 0.0), (110.0, 110.0), (-50.0, 400.0)] {
        for doing in &doings {
            let mut s = sense(3.0, me(mx), foe(fx));
            s.slots = slots();
            s.me.vx = 1e6;
            s.me.rage = 99.0;
            let mut f = foe(fx);
            f.doing = doing.clone();
            f.out_for = 50.0;
            let threats = vec![
                bolt_at(mx + 30.0),
                Threat {
                    age: 0.0,
                    damage: f64::NAN,
                    shape: ThreatShape::Area {
                        x: fx,
                        radius: f64::NAN,
                        reach: 0.0,
                        lands_in: -3.0,
                        grounded: true,
                    },
                },
            ];
            let o = observe(&s, &f, &threats, &kit, &tag);
            assert_eq!(o.len(), OBS);
            for (i, v) in o.iter().enumerate() {
                assert!(
                    v.is_finite() && (-4.0..=4.0).contains(v),
                    "obs[{i}] = {v} at me {mx} foe {fx} doing {doing:?}"
                );
            }
        }
    }
}

#[test]
fn the_tag_lands_at_the_end_unchanged() {
    let tag = [0.0, 1.0, 0.0, 0.0, 1.0];
    let s = sense(1.0, me(40.0), foe(150.0));
    let o = observe(&s, &foe(150.0), &[], &kit(), &tag);
    assert_eq!(&o[OBS - TAG..], &tag);
}

#[test]
fn the_view_is_the_same_from_either_side() {
    // a fighter on the left facing a foe on its right sees what its mirror image sees
    let kit = kit();
    let tag = [0.0; TAG];
    let a = observe(
        &sense(1.0, me(40.0), foe(150.0)),
        &foe(150.0),
        &[bolt_at(100.0)],
        &kit,
        &tag,
    );
    let mut m = me(180.0);
    m.face = -1.0;
    let mut b_threat = bolt_at(120.0);
    if let ThreatShape::Projectile { vx, .. } = &mut b_threat.shape {
        *vx = 230.0;
    }
    let b = observe(
        &sense(1.0, m, foe(70.0)),
        &foe(70.0),
        &[b_threat],
        &kit,
        &tag,
    );
    assert_eq!(a, b);
}

#[test]
fn threats_past_the_fourth_nearest_are_dropped_and_missing_ones_are_zero() {
    let kit = kit();
    let tag = [0.0; TAG];
    let s = sense(1.0, me(40.0), foe(150.0));
    let none = observe(&s, &foe(150.0), &[], &kit, &tag);
    let near: Vec<Threat> = [60.0, 70.0, 80.0, 90.0]
        .iter()
        .map(|&x| bolt_at(x))
        .collect();
    let mut more = near.clone();
    more.push(bolt_at(200.0));
    more.insert(0, bolt_at(210.0));
    let four = observe(&s, &foe(150.0), &near, &kit, &tag);
    let six = observe(&s, &foe(150.0), &more, &kit, &tag);
    assert_eq!(four, six, "the two far bolts are not seen");
    assert_ne!(none, four);
    let one = observe(&s, &foe(150.0), &near[..1], &kit, &tag);
    let differing = one.iter().zip(&none).filter(|(a, b)| a != b).count();
    assert!(
        differing > 0 && differing <= 12,
        "one threat fills one threat's group only: {differing}"
    );
}
