//! The skeleton a pose gives: lengths, which way each joint folds, and how facing turns the figure.

use pixfx_pose::{Body, Limb, Pose, Side, Sword, facing, presets, skeleton};

const EPS: f64 = 1e-6;

fn close(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-6
}

#[test]
fn a_body_stands_as_tall_as_asked() {
    for (h, heads) in [(56.0, 6.0), (40.0, 3.5), (90.0, 7.5)] {
        assert!(
            close(Body::new(h, heads).height(), h),
            "{h} at {heads} heads"
        );
    }
}

#[test]
fn standing_at_rest_the_figure_is_upright_and_placed_on_the_ground() {
    let body = Body::new(56.0, 6.0);
    let sk = skeleton(&body, &Pose::default(), facing::RIGHT).placed(64.0, 62.0, 0.0);
    let [_, top, _, bottom] = sk.body_bounds();
    assert!(close(bottom, 62.0), "feet on the ground: {bottom}");
    assert!(
        close(bottom - top, 56.0),
        "the full height: {}",
        bottom - top
    );
    assert!(close(sk.pelvis.x, 32.0), "pelvis centred across");
    assert!(close(sk.head.x, sk.pelvis.x) && sk.head.y < sk.neck.y);
}

#[test]
fn facing_decides_which_side_is_near_and_where_the_nose_points() {
    let body = Body::default();
    let rest = Pose::default();
    let side = skeleton(&body, &rest, facing::RIGHT);
    assert!(
        side.right_arm.root.depth > side.left_arm.root.depth,
        "side view: her right is near"
    );
    assert!(side.nose.x > side.head.x + EPS && side.nose_visible);

    let front = skeleton(&body, &rest, facing::FRONT);
    assert!(
        front.right_arm.root.x < front.left_arm.root.x,
        "front view: her right is on the viewer's left"
    );
    assert!(front.nose.depth > front.head.depth && front.nose_visible);

    let back = skeleton(&body, &rest, facing::BACK);
    assert!(
        back.right_arm.root.x > back.left_arm.root.x,
        "back view: her right on the viewer's right"
    );
    assert!(!back.nose_visible, "no face from behind");

    let left = skeleton(&body, &rest, facing::LEFT);
    assert!(left.nose.x < left.head.x - EPS);
}

#[test]
fn an_arm_raised_ninety_points_ahead_and_an_elbow_folds_forward() {
    let body = Body::default();
    let reach = Pose {
        right_arm: Limb::new(90.0, 0.0, 0.0),
        ..Pose::default()
    };
    let a = skeleton(&body, &reach, facing::RIGHT).right_arm;
    assert!(close(a.end.y, a.root.y), "level with the shoulder");
    assert!(
        close(a.end.x - a.root.x, body.upper_arm + body.forearm),
        "straight ahead"
    );

    let fold = Pose {
        right_arm: Limb::new(0.0, 0.0, 90.0),
        ..Pose::default()
    };
    let a = skeleton(&body, &fold, facing::RIGHT).right_arm;
    assert!(
        close(a.end.y, a.mid.y) && a.end.x > a.mid.x + EPS,
        "forearm forward from a hanging elbow"
    );
}

#[test]
fn a_knee_folds_back_and_spread_swings_out_to_her_own_side() {
    let body = Body::default();
    let step = Pose {
        right_leg: Limb::new(90.0, 0.0, 90.0),
        ..Pose::default()
    };
    let l = skeleton(&body, &step, facing::RIGHT).right_leg;
    assert!(close(l.mid.x - l.root.x, body.thigh), "thigh ahead");
    assert!(
        close(l.end.x, l.mid.x) && close(l.end.y - l.mid.y, body.shin),
        "shin straight down"
    );

    let back = Pose {
        right_leg: Limb::new(0.0, 0.0, 90.0),
        ..Pose::default()
    };
    let l = skeleton(&body, &back, facing::RIGHT).right_leg;
    assert!(l.end.x < l.mid.x - EPS, "shin back from a hanging thigh");

    let out = Pose {
        right_arm: Limb::new(0.0, 90.0, 0.0),
        left_arm: Limb::new(0.0, 90.0, 0.0),
        ..Pose::default()
    };
    let f = skeleton(&body, &out, facing::FRONT);
    assert!(
        f.right_arm.end.x < f.right_arm.root.x - EPS && f.left_arm.end.x > f.left_arm.root.x + EPS
    );
}

#[test]
fn lying_on_her_back_puts_the_head_behind_her() {
    let sk = skeleton(&Body::default(), &presets::down(), facing::RIGHT);
    assert!(
        sk.head.x < sk.pelvis.x - 20.0,
        "head toward her back, screen left"
    );
    assert!(
        (sk.head.y - sk.pelvis.y).abs() < 2.0,
        "level with the pelvis"
    );
}

#[test]
fn the_sword_starts_in_its_hand() {
    let body = Body::default();
    for hand in [Side::Right, Side::Left] {
        let pose = Pose {
            sword: Some(Sword { hand, angle: 30.0 }),
            ..Pose::default()
        };
        let sk = skeleton(&body, &pose, facing::FRONT_RIGHT);
        let [hilt, tip] = sk.sword.unwrap_or_default();
        let held = if hand == Side::Right {
            sk.right_arm.end
        } else {
            sk.left_arm.end
        };
        assert_eq!(hilt, held);
        let (dx, dy, dz) = (tip.x - hilt.x, tip.y - hilt.y, tip.depth - hilt.depth);
        let len = libm::sqrt(dx * dx + dy * dy + dz * dz);
        assert!(close(len, body.sword));
    }
}

#[test]
fn mirroring_swaps_sides_and_twice_gives_the_pose_back() {
    let p = presets::guard();
    let m = p.mirrored();
    assert_eq!(m.left_arm, p.right_arm);
    assert_eq!(m.sword.map(|s| s.hand), Some(Side::Left));
    assert_eq!(m.mirrored(), p);
}

#[test]
fn a_bad_angle_still_gives_a_finite_skeleton() {
    let pose = Pose {
        lean: f64::NAN,
        right_arm: Limb::new(f64::INFINITY, 0.0, 0.0),
        ..Pose::default()
    };
    let sk = skeleton(&Body::default(), &pose, f64::NAN);
    for (_, a, b) in sk.bones() {
        assert!(
            [a.x, a.y, a.depth, b.x, b.y, b.depth]
                .iter()
                .all(|v| v.is_finite())
        );
    }
}

#[test]
fn every_preset_name_gives_a_pose_and_cycles_repeat() {
    for n in presets::NAMES {
        assert!(presets::by_name(n, 0.3).is_some(), "{n}");
    }
    assert!(presets::by_name("moonwalk", 0.0).is_none());
    assert_eq!(presets::walk(0.25), presets::walk(1.25));
    let (a, b) = (presets::walk(0.0).right_leg, presets::walk(0.5).left_leg);
    assert!(
        close(a.raise, b.raise) && close(a.bend, b.bend),
        "half a cycle on is the other foot"
    );
}
