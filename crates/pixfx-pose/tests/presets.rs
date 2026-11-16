//! The sword fighter's presets on the rig: which way they put her, mirroring, the cycles.

use pixfx_pose::{Body, Side, facing, presets, skeleton};

fn close(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-6
}

#[test]
fn lying_on_her_back_puts_the_head_behind_her() {
    let body = Body::default();
    let sk = skeleton(&body, &presets::down(), facing::RIGHT);
    // most of the way along her spine and neck from the pelvis
    assert!(
        sk.head.x < sk.pelvis.x - 0.8 * (body.torso + body.neck),
        "head toward her back, screen left"
    );
    assert!(
        (sk.head.y - sk.pelvis.y).abs() < 2.0,
        "level with the pelvis"
    );
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
