//! The camera: yaw turns the figure, pitch looks down at her (side, iso, top-down).

#![cfg(feature = "body")]

use pixfx_core::body::{
    Body, Camera, Dir4, Dir8, LimbPoints, Point, Pose, Skeleton, facing, skeleton, skeleton_seen,
};

fn idle() -> Pose {
    Pose::default()
}

fn joints(s: &Skeleton) -> Vec<Point> {
    let limb = |l: LimbPoints| [l.root, l.mid, l.end];
    let mut v = vec![s.head, s.nose, s.neck, s.pelvis];
    for l in [s.right_arm, s.left_arm, s.right_leg, s.left_leg] {
        v.extend(limb(l));
    }
    v
}

#[test]
fn pitch_zero_is_todays_skeleton() {
    let body = Body::default();
    let poses = [
        idle(),
        Pose {
            lean: 20.0,
            twist: 30.0,
            lie: 10.0,
            ..idle()
        },
    ];
    for pose in poses {
        for f in facing::ALL {
            let a = skeleton(&body, &pose, f);
            let b = skeleton_seen(&body, &pose, Camera { yaw: f, pitch: 0.0 });
            assert_eq!(joints(&a), joints(&b), "facing {f}");
        }
    }
}

fn standing_height(cam: Camera) -> f64 {
    let s = skeleton_seen(&Body::default(), &idle(), cam);
    s.ground.y - (s.head.y - s.head_radius)
}

#[test]
fn looking_down_shortens_the_standing_figure() {
    let side = standing_height(Camera::side(Dir4::S));
    let iso = standing_height(Camera::iso(Dir8::S));
    let top = standing_height(Camera::topdown(Dir8::S));
    // Standing side-on, the skeleton is as tall as the body says: crown to the ankles it stands on.
    let painted = Body::default().height();
    assert!((side - painted).abs() < 1.0, "side {side}");
    assert!(top < painted && top > painted * 0.5 - 1.0, "top {top}");
    assert!(iso < side && iso > top, "iso {iso}");
}

#[test]
fn the_ground_point_is_under_the_pelvis_at_her_soles() {
    let s = skeleton_seen(&Body::default(), &idle(), Camera::side(Dir4::E));
    assert!((s.ground.x - s.pelvis.x).abs() < 1e-9);
    let soles = s.right_leg.end.y.max(s.left_leg.end.y);
    assert!(
        (s.ground.y - soles).abs() < 1e-9,
        "{} vs {soles}",
        s.ground.y
    );
}

#[test]
fn top_down_south_sees_the_face_and_north_does_not() {
    let body = Body::default();
    assert!(skeleton_seen(&body, &idle(), Camera::topdown(Dir8::S)).nose_visible);
    assert!(!skeleton_seen(&body, &idle(), Camera::topdown(Dir8::N)).nose_visible);
}

#[test]
fn a_symmetric_pose_from_west_mirrors_east() {
    let body = Body::default();
    let e = skeleton_seen(&body, &idle(), Camera::side(Dir4::E));
    let w = skeleton_seen(&body, &idle(), Camera::side(Dir4::W));
    let flip = |p: Point| Point { x: -p.x, ..p };
    let fl = |l: LimbPoints| LimbPoints {
        root: flip(l.root),
        mid: flip(l.mid),
        end: flip(l.end),
        tip: flip(l.tip),
    };
    let close = |a: LimbPoints, b: LimbPoints| {
        [(a.root, b.root), (a.mid, b.mid), (a.end, b.end)]
            .iter()
            .all(|(p, q)| {
                (p.x - q.x).abs() < 1e-9
                    && (p.y - q.y).abs() < 1e-9
                    && (p.depth - q.depth).abs() < 1e-9
            })
    };
    assert!(close(w.right_arm, fl(e.left_arm)));
    assert!(close(w.left_leg, fl(e.right_leg)));
}

#[test]
fn directions_name_their_yaw() {
    assert_eq!(Dir4::all().map(Dir4::yaw), [0.0, 90.0, 180.0, 270.0]);
    assert_eq!(
        Dir8::all().map(Dir8::yaw),
        [0.0, 45.0, 90.0, 135.0, 180.0, 225.0, 270.0, 315.0]
    );
    assert_eq!(
        Camera::iso(Dir8::SE),
        Camera {
            yaw: 45.0,
            pitch: 30.0
        }
    );
    assert_eq!(
        Camera::topdown(Dir8::N),
        Camera {
            yaw: 270.0,
            pitch: 45.0
        }
    );
}

#[test]
fn a_bad_camera_is_finite() {
    for cam in [
        Camera {
            yaw: f64::NAN,
            pitch: f64::INFINITY,
        },
        Camera {
            yaw: f64::NEG_INFINITY,
            pitch: f64::NAN,
        },
    ] {
        let s = skeleton_seen(&Body::default(), &idle(), cam);
        assert!(
            joints(&s)
                .iter()
                .chain([&s.ground])
                .all(|p| p.x.is_finite() && p.y.is_finite() && p.depth.is_finite())
        );
    }
}
