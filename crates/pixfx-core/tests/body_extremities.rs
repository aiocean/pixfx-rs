//! Hands, feet and roll: new joints that leave every old one where it was.

#![cfg(feature = "body")]

use pixfx_core::body::{Body, Bone, Pose, facing, skeleton};

#[test]
fn a_body_has_the_rulers_hand_and_foot() {
    let b = Body::default();
    let r = pixfx_draw::RULER;
    assert!((b.hand - f64::from(r.hand)).abs() < 1e-9);
    assert!((b.foot - 1.5 * f64::from(r.limb)).abs() < 1e-9);
}

#[test]
fn a_foot_points_forward_along_the_ground() {
    let s = skeleton(&Body::default(), &Pose::default(), facing::RIGHT);
    let (end, tip) = (s.right_leg.end, s.right_leg.tip);
    assert!(
        tip.x > end.x + 1.0,
        "toes ahead of the heel: {} vs {}",
        tip.x,
        end.x
    );
    assert!((tip.y - end.y).abs() < 0.5, "flat on the ground");
}

#[test]
fn toes_up_lifts_the_tip() {
    let flat = skeleton(&Body::default(), &Pose::default(), facing::RIGHT);
    let up = Pose {
        right_foot: 30.0,
        ..Pose::default()
    };
    let s = skeleton(&Body::default(), &up, facing::RIGHT);
    assert!(s.right_leg.tip.y < flat.right_leg.tip.y - 0.5);
    assert_eq!(s.left_leg.tip, flat.left_leg.tip);
}

#[test]
fn a_hand_carries_on_past_the_wrist() {
    let s = skeleton(&Body::default(), &Pose::default(), facing::RIGHT);
    assert!(
        s.right_arm.tip.y > s.right_arm.end.y + 1.0,
        "hanging hand below the wrist"
    );
}

#[test]
fn a_roll_of_180_stands_her_on_her_head() {
    let s = skeleton(
        &Body::default(),
        &Pose {
            roll: 180.0,
            ..Pose::default()
        },
        facing::FRONT,
    );
    assert!(s.head.y > s.pelvis.y + 5.0);
}

#[test]
fn hands_and_feet_are_bones_before_the_sword() {
    let s = skeleton(&Body::default(), &Pose::default(), facing::RIGHT);
    let names: Vec<Bone> = s.bones().iter().map(|b| b.0).collect();
    for b in [
        Bone::RightHand,
        Bone::LeftHand,
        Bone::RightFoot,
        Bone::LeftFoot,
    ] {
        assert!(names.contains(&b), "{b:?}");
    }
}

#[test]
fn mirroring_swaps_hands_and_feet_and_turns_the_roll() {
    let p = Pose {
        right_foot: 20.0,
        left_hand: 10.0,
        roll: 15.0,
        ..Pose::default()
    };
    let m = p.mirrored();
    assert_eq!((m.left_foot, m.right_hand, m.roll), (20.0, 10.0, -15.0));
}

#[test]
fn an_old_json_body_and_pose_still_load() {
    let body: Result<Body, _> = serde_json::from_str(
        r#"{"head":3.5,"neck":1.4,"torso":10,"shoulder":5,"hip":2.8,"upper_arm":6,"forearm":5.5,"thigh":7,"shin":7,"sword":16}"#,
    );
    assert!(body.is_ok(), "{body:?}");
    let pose: Result<Pose, _> = serde_json::from_str(r#"{"lean":10}"#);
    assert!(pose.is_ok());
}

#[test]
fn a_spin_of_180_turns_her_whole_body_round() {
    let b = Body::default();
    let s = skeleton(
        &b,
        &Pose {
            spin: 180.0,
            ..Pose::default()
        },
        facing::RIGHT,
    );
    let e = skeleton(&b, &Pose::default(), facing::LEFT);
    assert!(
        (s.nose.x - e.nose.x).abs() < 1e-9 && (s.right_leg.tip.x - e.right_leg.tip.x).abs() < 1e-9
    );
    assert_eq!(
        Pose {
            spin: 30.0,
            ..Pose::default()
        }
        .mirrored()
        .spin,
        -30.0
    );
}
