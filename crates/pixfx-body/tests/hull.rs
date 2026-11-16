//! The hull: one shape per body part, seen from the camera, back to front.

use pixfx_body::{
    Body, HullShape, Limb, Part, Pose, Shape, Side, Skeleton, Sword, facing, hull, skeleton,
};

fn first(shapes: &[HullShape], part: Part) -> Option<usize> {
    shapes.iter().position(|s| s.part == part)
}

fn finite(s: &HullShape) -> bool {
    let pts: Vec<f64> = match s.shape {
        Shape::Capsule { a, b, ra, rb } => vec![a.0, a.1, b.0, b.1, ra, rb],
        Shape::Disc { c, r } => vec![c.0, c.1, r],
        Shape::Quad(q) => q.iter().flat_map(|p| [p.0, p.1]).collect(),
    };
    pts.iter().chain([&s.depth]).all(|v| v.is_finite())
}

#[test]
fn every_part_but_the_sword_is_in_the_hull_of_idle() {
    let b = Body::default();
    let shapes = hull(&b, &skeleton(&b, &Pose::default(), facing::FRONT));
    for p in Part::ALL {
        assert_eq!(first(&shapes, p).is_some(), p != Part::Sword, "{p:?}");
    }
    let held = Pose {
        sword: Some(Sword {
            hand: Side::Right,
            angle: 0.0,
        }),
        ..Pose::default()
    };
    assert!(first(&hull(&b, &skeleton(&b, &held, facing::FRONT)), Part::Sword).is_some());
}

#[test]
fn shapes_run_back_to_front() {
    let b = Body::default();
    let shapes = hull(&b, &skeleton(&b, &Pose::default(), facing::FRONT_RIGHT));
    assert!(shapes.windows(2).all(|w| w[0].depth <= w[1].depth));
}

#[test]
fn the_far_arm_is_drawn_before_the_chest_from_the_side() {
    let b = Body::default();
    let e = hull(&b, &skeleton(&b, &Pose::default(), facing::RIGHT));
    assert!(first(&e, Part::LeftUpperArm) < first(&e, Part::Chest));
    assert!(first(&e, Part::RightUpperArm) > first(&e, Part::Chest));
}

#[test]
fn from_the_other_side_the_order_turns() {
    let b = Body::default();
    let w = hull(&b, &skeleton(&b, &Pose::default(), facing::LEFT));
    assert!(first(&w, Part::RightUpperArm) < first(&w, Part::Chest));
    assert!(first(&w, Part::LeftUpperArm) > first(&w, Part::Chest));
}

#[test]
fn the_chest_has_width_from_the_front_and_thickness_from_the_side() {
    let b = Body::default();
    let spread = |f: f64| {
        let xs: Vec<f64> = hull(&b, &skeleton(&b, &Pose::default(), f))
            .iter()
            .filter(|s| s.part == Part::Chest)
            .flat_map(|s| match s.shape {
                Shape::Capsule { a, b, ra, .. } => vec![a.0 - ra, a.0 + ra, b.0 - ra, b.0 + ra],
                Shape::Disc { c, r } => vec![c.0 - r, c.0 + r],
                Shape::Quad(q) => q.iter().map(|p| p.0).collect(),
            })
            .collect();
        xs.iter().fold(f64::MIN, |m, v| m.max(*v)) - xs.iter().fold(f64::MAX, |m, v| m.min(*v))
    };
    let (front, side) = (spread(facing::FRONT), spread(facing::RIGHT));
    assert!(front > side && side >= 3.0, "front {front} side {side}");
}

#[test]
fn an_arm_pointed_at_the_camera_still_has_a_shape() {
    let b = Body::default();
    let at_you = Pose {
        right_arm: Limb::new(90.0, 0.0, 0.0),
        ..Pose::default()
    };
    let shapes = hull(&b, &skeleton(&b, &at_you, facing::FRONT));
    assert!(first(&shapes, Part::RightForearm).is_some());
    assert!(shapes.iter().all(finite));
}

#[test]
fn a_bad_skeleton_gives_finite_shapes() {
    let b = Body::default();
    let mut s: Skeleton = skeleton(&b, &Pose::default(), facing::FRONT);
    s.head.x = f64::NAN;
    s.right_arm.mid.y = f64::INFINITY;
    let shapes = hull(&b, &s);
    assert!(shapes.iter().all(finite));
    assert!(first(&shapes, Part::Chest).is_some());
}

#[test]
fn every_part_has_an_id_and_a_colour_of_its_own() {
    let ids: Vec<u8> = Part::ALL.iter().map(|p| p.id()).collect();
    assert_eq!(ids, (1..=17).collect::<Vec<u8>>());
    let mut colours: Vec<_> = Part::ALL
        .iter()
        .map(|p| {
            let c = p.guide();
            (c.0, c.1, c.2)
        })
        .collect();
    colours.sort();
    colours.dedup();
    assert_eq!(colours.len(), 17);
    assert_eq!(Part::from_id(5), Some(Part::ALL[4]));
    assert_eq!(Part::from_id(0), None);
}
