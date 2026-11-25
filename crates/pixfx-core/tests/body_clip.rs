//! Clips: keys eased into frames, loops that close, root motion, and planted limbs that do not slide.

#![cfg(feature = "body")]

use pixfx_core::body::{Body, Clip, Contact, Ease, Key, Limb, Limb4, Mode, Pose, skeleton};

fn key(at: u32, pose: Pose) -> Key {
    Key {
        at,
        pose,
        ease: Ease::Linear,
    }
}

fn lean(l: f64) -> Pose {
    Pose {
        lean: l,
        ..Pose::default()
    }
}

fn clip(mode: Mode, keys: Vec<Key>) -> Clip {
    Clip {
        name: "t".into(),
        frames: 8,
        fps: 12,
        mode,
        keys,
        root: vec![],
        contacts: vec![],
    }
}

#[test]
fn a_frame_on_a_key_is_that_keys_pose() {
    let c = clip(Mode::Once, vec![key(0, lean(0.0)), key(4, lean(40.0))]);
    assert_eq!(c.pose_at(&Body::default(), 4).lean, 40.0);
}

#[test]
fn halfway_with_linear_is_the_mean() {
    let c = clip(Mode::Once, vec![key(0, lean(0.0)), key(4, lean(40.0))]);
    assert!((c.pose_at(&Body::default(), 2).lean - 20.0).abs() < 1e-9);
}

#[test]
fn ease_in_out_is_slow_at_both_ends() {
    let mut c = clip(Mode::Once, vec![key(0, lean(0.0)), key(4, lean(40.0))]);
    c.keys[0].ease = Ease::InOut;
    let b = Body::default();
    assert!(c.pose_at(&b, 1).lean < 10.0);
    assert!((c.pose_at(&b, 2).lean - 20.0).abs() < 1e-9);
    c.keys[0].ease = Ease::Hold;
    assert_eq!(c.pose_at(&b, 3).lean, 0.0);
}

#[test]
fn a_loop_flows_from_its_last_frame_into_its_first() {
    let c = clip(Mode::Loop, vec![key(0, lean(0.0)), key(4, lean(40.0))]);
    let b = Body::default();
    assert_eq!(c.pose_at(&b, 8), c.pose_at(&b, 0));
    assert_eq!(c.pose_at(&b, 10), c.pose_at(&b, 2));
    let steps: Vec<f64> = (0..8)
        .map(|f| (c.pose_at(&b, f + 1).lean - c.pose_at(&b, f).lean).abs())
        .collect();
    let inner = steps.iter().take(7).fold(0.0_f64, |m, s| m.max(*s));
    assert!(
        steps[7] <= inner + 1e-9,
        "the wrap step {} is no larger than {inner}",
        steps[7]
    );
}

#[test]
fn once_holds_the_last_key() {
    let c = clip(Mode::Once, vec![key(0, lean(0.0)), key(4, lean(40.0))]);
    let b = Body::default();
    assert_eq!(c.pose_at(&b, 6).lean, 40.0);
    assert_eq!(c.pose_at(&b, 99).lean, 40.0);
}

#[test]
fn one_key_is_a_still_clip_and_none_is_the_rest_pose() {
    let b = Body::default();
    let c = clip(Mode::Loop, vec![key(3, lean(15.0))]);
    assert!((0..8).all(|f| c.pose_at(&b, f).lean == 15.0));
    assert_eq!(clip(Mode::Loop, vec![]).pose_at(&b, 2), Pose::default());
}

#[test]
fn root_motion_is_eased_like_the_keys() {
    let mut c = clip(Mode::Loop, vec![key(0, lean(0.0))]);
    c.root = vec![(0, 0.0, 0.0), (4, 8.0, 2.0)];
    assert_eq!(c.root_at(2), (4.0, 1.0));
    assert_eq!(c.root_at(0), (0.0, 0.0));
}

fn stride() -> Clip {
    let step = |r: f64| Pose {
        right_leg: Limb::new(r, 0.0, 10.0),
        left_leg: Limb::new(-r, 0.0, 10.0),
        ..Pose::default()
    };
    let mut c = clip(Mode::Loop, vec![key(0, step(25.0)), key(4, step(-25.0))]);
    c.contacts = vec![Contact {
        limb: Limb4::RightLeg,
        from: 0,
        to: 4,
    }];
    c
}

#[test]
fn a_planted_foot_does_not_slide() {
    let b = Body::default();
    let c = stride();
    let at = |f: u32| skeleton(&b, &c.pose_at(&b, f), 0.0).right_leg.end;
    let first = at(0);
    for f in 1..=4 {
        let p = at(f);
        assert!(
            (p.x - first.x).abs() < 0.5 && (p.y - first.y).abs() < 0.5,
            "frame {f}: {p:?} vs {first:?}"
        );
    }
    // without the contact the foot swings
    let mut free = stride();
    free.contacts.clear();
    let moved = skeleton(&b, &free.pose_at(&b, 2), 0.0).right_leg.end;
    assert!((moved.x - first.x).abs() > 2.0);
}

#[test]
fn a_planted_foot_stays_put_in_the_world_while_the_root_moves() {
    let b = Body::default();
    let mut c = stride();
    c.root = vec![(0, 0.0, 0.0), (4, 3.0, 0.0)];
    let world = |f: u32| skeleton(&b, &c.pose_at(&b, f), 0.0).right_leg.end.x + c.root_at(f).0;
    for f in 1..=4 {
        assert!((world(f) - world(0)).abs() < 0.5, "frame {f}");
    }
}

#[test]
fn an_unreachable_target_straightens_the_leg_and_stays_finite() {
    let b = Body::default();
    let mut c = stride();
    c.root = vec![(0, 0.0, 0.0), (4, 60.0, 0.0)];
    let p = c.pose_at(&b, 4);
    assert!(
        p.right_leg.bend.abs() < 1e-6,
        "straight: {}",
        p.right_leg.bend
    );
    assert!(p.right_leg.raise.is_finite());
}

#[test]
fn a_bad_number_in_a_clip_stays_finite() {
    let b = Body::default();
    let mut c = stride();
    c.keys[1].pose.right_leg.raise = f64::NAN;
    c.root = vec![(0, f64::NAN, f64::INFINITY)];
    for f in 0..8 {
        let s = skeleton(&b, &c.pose_at(&b, f), 0.0);
        assert!(
            s.bones()
                .iter()
                .all(|(_, p, q)| [p.x, p.y, q.x, q.y].iter().all(|v| v.is_finite())),
            "frame {f}"
        );
        let (dx, dy) = c.root_at(f);
        assert!(dx.is_finite() && dy.is_finite());
    }
}

#[test]
fn a_clip_reads_from_json() {
    let c: Result<Clip, _> = serde_json::from_str(
        r#"{"name":"nod","frames":4,"fps":8,"mode":"once","keys":[{"at":0,"pose":{"lean":5},"ease":"in_out"}]}"#,
    );
    assert!(c.is_ok(), "{c:?}");
}

#[test]
fn a_planted_foot_across_the_loops_seam_stays_put_in_the_world() {
    let b = Body::default();
    let step = |r: f64| Pose {
        right_leg: Limb::new(r, 0.0, 10.0),
        left_leg: Limb::new(-r, 0.0, 10.0),
        ..Pose::default()
    };
    // the right foot lands ahead on frame 6 and is planted round the seam until it is behind on frame 2; the
    // cycle travels 22 pixels, keyed at its end
    let mut c = clip(Mode::Loop, vec![key(2, step(-25.0)), key(6, step(25.0))]);
    c.contacts = vec![Contact {
        limb: Limb4::RightLeg,
        from: 6,
        to: 1,
    }];
    c.root = vec![(0, 0.0, 0.0), (8, 22.0, 0.0)];
    let end = |f: u32| skeleton(&b, &c.pose_at(&b, f), 0.0).right_leg.end.x;
    // where the foot is in the world: before the seam the root is root_at(f), after it a whole cycle on
    let world = |f: u32| end(f) + c.root_at(f).0 + if f < 6 { 22.0 } else { 0.0 };
    for f in [7, 0, 1] {
        assert!(
            (world(f) - world(6)).abs() < 0.5,
            "frame {f}: {} vs {}",
            world(f),
            world(6)
        );
    }
}

#[test]
fn a_planted_foot_stays_flat_on_the_ground() {
    let b = Body::default();
    let mut c = stride();
    c.root = vec![(0, 0.0, 0.0), (4, 6.0, 0.0)];
    for f in 0..=4 {
        let s = skeleton(&b, &c.pose_at(&b, f), 0.0);
        let (end, tip) = (s.right_leg.end, s.right_leg.tip);
        let start = skeleton(&b, &c.pose_at(&b, 0), 0.0).right_leg;
        let slope =
            |e: pixfx_core::body::Point, t: pixfx_core::body::Point| (t.y - e.y) / (t.x - e.x);
        assert!(
            (slope(end, tip) - slope(start.end, start.tip)).abs() < 1e-6,
            "frame {f}"
        );
    }
}
