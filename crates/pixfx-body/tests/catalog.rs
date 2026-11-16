//! The catalog: every clip by name plays from every camera, loops close, and planted feet hold in the world.

use pixfx_body::{
    Body, Camera, Dir8, Limb4, Mode, Point, Skeleton, catalog, skeleton, skeleton_seen,
};

fn joints(s: &Skeleton) -> Vec<Point> {
    let mut v = vec![s.head, s.neck, s.pelvis];
    for l in [s.right_arm, s.left_arm, s.right_leg, s.left_leg] {
        v.extend([l.root, l.mid, l.end, l.tip]);
    }
    v
}

#[test]
fn every_name_gives_a_clip_of_its_family() {
    assert!(catalog::NAMES.len() >= 24);
    for n in catalog::NAMES {
        let c = catalog::by_name(n);
        assert!(
            c.as_ref().is_some_and(|c| c.name == *n && c.frames >= 1),
            "{n}"
        );
        assert!(catalog::family(n).is_some(), "{n}");
    }
    assert!(catalog::by_name("moonwalk").is_none());
    assert_eq!(catalog::family("walk"), Some("ground"));
    assert_eq!(catalog::family("pose/guard"), Some("combat"));
}

#[test]
fn every_clip_is_finite_at_every_frame_from_every_camera() {
    let b = Body::default();
    for n in catalog::NAMES {
        let Some(c) = catalog::by_name(n) else {
            continue;
        };
        for f in 0..c.frames {
            let p = c.pose_at(&b, f);
            for d in Dir8::all() {
                for cam in [
                    Camera::iso(d),
                    Camera::topdown(d),
                    Camera {
                        yaw: d.yaw(),
                        pitch: 0.0,
                    },
                ] {
                    let s = skeleton_seen(&b, &p, cam);
                    assert!(
                        joints(&s)
                            .iter()
                            .all(|q| q.x.is_finite() && q.y.is_finite() && q.depth.is_finite()),
                        "{n} frame {f} {cam:?}"
                    );
                }
            }
        }
    }
}

fn largest_move(a: &Skeleton, b: &Skeleton) -> f64 {
    joints(a)
        .iter()
        .zip(joints(b))
        .map(|(p, q)| libm::hypot(p.x - q.x, p.y - q.y))
        .fold(0.0, f64::max)
}

#[test]
fn every_loop_flows_into_its_first_frame() {
    let b = Body::default();
    for n in catalog::NAMES {
        let Some(c) = catalog::by_name(n) else {
            continue;
        };
        if c.mode != Mode::Loop || c.frames < 2 {
            continue;
        }
        let s = |f: u32| skeleton(&b, &c.pose_at(&b, f), 0.0);
        let inner = (0..c.frames - 1)
            .map(|f| largest_move(&s(f), &s(f + 1)))
            .fold(0.0, f64::max);
        let seam = largest_move(&s(c.frames - 1), &s(0));
        assert!(seam <= inner * 1.25 + 0.5, "{n}: seam {seam} vs {inner}");
    }
}

#[test]
fn planted_feet_hold_in_the_world_and_stand_on_the_ground() {
    let b = Body::default();
    let mut checked = 0;
    for n in catalog::NAMES {
        let Some(c) = catalog::by_name(n) else {
            continue;
        };
        for k in &c.contacts {
            let end = |f: u32| {
                let s = skeleton(&b, &c.pose_at(&b, f), 0.0);
                let l = match k.limb {
                    Limb4::RightLeg => s.right_leg,
                    Limb4::LeftLeg => s.left_leg,
                    Limb4::RightArm => s.right_arm,
                    Limb4::LeftArm => s.left_arm,
                };
                (l.end, s.ground)
            };
            let span: Vec<u32> = if k.from <= k.to {
                (k.from..=k.to).collect()
            } else {
                (k.from..c.frames).chain(0..=k.to).collect()
            };
            let (start, _) = end(k.from);
            for f in span {
                let (p, ground) = end(f);
                let moved = p.x + c.travel(k.from, f).0 - start.x;
                assert!(
                    moved.abs() < 0.5,
                    "{n} {:?} frame {f}: slid {moved}",
                    k.limb
                );
                if matches!(k.limb, Limb4::RightLeg | Limb4::LeftLeg) {
                    assert!(
                        (p.y - ground.y).abs() < 0.5,
                        "{n} {:?} frame {f}: sole {} ground {}",
                        k.limb,
                        p.y,
                        ground.y
                    );
                }
                checked += 1;
            }
        }
    }
    assert!(checked > 0);
    for n in ["walk", "run", "sprint", "crouch_walk"] {
        assert!(
            catalog::by_name(n).is_some_and(|c| !c.contacts.is_empty()),
            "{n} plants its feet"
        );
    }
}
