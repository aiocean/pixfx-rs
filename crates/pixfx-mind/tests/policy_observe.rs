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
            ultimate: false,
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

// The documented layout (`observe.rs`'s header): me 18, foe 38, threats 4 x 12, slots 7 x 35, tag 5.
const MY_WINDUP: usize = 15;
const MY_RECOVER: usize = 16;
const MY_LEFT: usize = 17;
const FOE_LEFT: usize = 18 + 26;
const FOE_SPAN: usize = 18 + 27;
const FOE_ULTIMATE: usize = 18 + 30;
const SLOTS_AT: usize = 18 + 38 + 4 * 12;
const SLOT: usize = 35;
const SLOT_STUN: usize = 22;
const SLOT_CHILL: usize = 28;

#[test]
fn obs_is_the_documented_sum() {
    assert_eq!(OBS, 18 + 38 + 4 * 12 + 7 * 35 + TAG);
}

#[test]
fn the_foes_seconds_left_fall_as_its_windup_runs() {
    let s = sense(1.0, me(40.0), foe(150.0));
    let at = |t: f64| {
        let mut f = foe(150.0);
        f.doing = blade_windup(t);
        observe(&s, &f, &[], &kit(), &[0.0; TAG])
    };
    let (early, late) = (at(0.05), at(0.15));
    assert!(
        (v(&early, FOE_LEFT) - 0.15).abs() < 1e-6,
        "{}",
        v(&early, FOE_LEFT)
    );
    assert!(
        (v(&late, FOE_LEFT) - 0.05).abs() < 1e-6,
        "{}",
        v(&late, FOE_LEFT)
    );
    assert!((v(&early, FOE_SPAN) - 0.2).abs() < 1e-6);
    assert_eq!(v(&early, FOE_ULTIMATE), 0.0);
    let mut f = foe(150.0);
    let mut big = blade_windup(0.1);
    if let Doing::Act { ultimate, .. } = &mut big {
        *ultimate = true;
    }
    f.doing = big;
    assert_eq!(
        v(&observe(&s, &f, &[], &kit(), &[0.0; TAG]), FOE_ULTIMATE),
        1.0
    );
}

#[test]
fn its_own_recover_flag_is_set_while_it_recovers() {
    let mut m = me(40.0);
    let idle = observe(
        &sense(1.0, m.clone(), foe(150.0)),
        &foe(150.0),
        &[],
        &kit(),
        &[0.0; TAG],
    );
    assert_eq!(
        [v(&idle, MY_WINDUP), v(&idle, MY_RECOVER), v(&idle, MY_LEFT)],
        [0.0, 0.0, 0.0]
    );
    m.act = Some(Acting {
        phase: ActPhase::Recover,
        t: 0.1,
        windup: 0.2,
        recover: 0.32,
    });
    let o = observe(
        &sense(1.0, m, foe(150.0)),
        &foe(150.0),
        &[],
        &kit(),
        &[0.0; TAG],
    );
    assert_eq!(v(&o, MY_WINDUP), 0.0);
    assert_eq!(v(&o, MY_RECOVER), 1.0);
    assert!((v(&o, MY_LEFT) - 0.22).abs() < 1e-6, "{}", v(&o, MY_LEFT));
}

#[test]
fn a_slot_of_a_stunning_skill_carries_its_stun() {
    let mut kit = kit();
    kit[4].inflicts.stun = 0.6;
    kit[3].inflicts.chill = 1.0;
    kit[3].inflicts.chill_max = 3.0;
    let mut s = sense(1.0, me(40.0), foe(150.0));
    s.slots = slots();
    s.stacks = vec![1.0, 1.0, 1.0, 2.0];
    let o = observe(&s, &foe(150.0), &[], &kit, &[0.0; TAG]);
    // slot 4 holds skill 3 (the falling area), slot 6 skill 6; skill 4 (the barrier) is in no slot
    assert!((v(&o, SLOTS_AT + 4 * SLOT + SLOT_CHILL) - 2.0 / 3.0).abs() < 1e-6);
    s.slots[1] = Some(4);
    let o = observe(&s, &foe(150.0), &[], &kit, &[0.0; TAG]);
    assert!((v(&o, SLOTS_AT + SLOT + SLOT_STUN) - 0.6).abs() < 1e-6);
    assert_eq!(v(&o, SLOTS_AT + SLOT_STUN), 0.0, "the blade does not stun");
}

/// Value `i` of an observation, NaN past its end.
fn v(o: &[f32], i: usize) -> f32 {
    o.get(i).copied().unwrap_or(f32::NAN)
}
