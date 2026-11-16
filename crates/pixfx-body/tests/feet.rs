//! Feet on the ground lie flat in every still, and a body with a zero-length bone still poses.

use pixfx_body::{Body, Camera, Dir4, Layers, Look, catalog, clip_cell, sprite};

#[test]
fn every_still_stands_on_a_flat_foot() {
    let b = Body::default();
    let mut bad = vec![];
    for name in catalog::NAMES.iter().filter(|n| n.starts_with("pose/")) {
        let Some(c) = catalog::by_name(name) else {
            continue;
        };
        if c.keys.first().is_some_and(|k| k.pose.lift > 0.0) {
            continue; // in the air a toe may point
        }
        let cam = Camera::side(Dir4::E);
        let cell = clip_cell(&b, &c, &[cam]);
        let s = sprite(&b, &c, 0, cam, cell, &Look::mannequin(), Layers::ALL).skeleton;
        // a foot on the ground (its heel within a pixel of the lowest heel) may not stab its toe below its heel
        let low = s.right_leg.end.y.max(s.left_leg.end.y);
        for (side, f) in [("right", &s.right_leg), ("left", &s.left_leg)] {
            let drop = f.tip.y - f.end.y;
            if low - f.end.y <= 1.0 && drop > 1.0 {
                bad.push(format!("{name} {side}: toe {drop:.1} px below the heel"));
            }
        }
    }
    assert!(bad.is_empty(), "{bad:#?}");
}

#[test]
fn a_zero_length_shin_still_paints_a_walk() {
    let b = Body {
        shin: 0.0,
        ..Body::default()
    };
    let Some(walk) = catalog::by_name("walk") else {
        unreachable!("walk is in the catalog")
    };
    let cam = Camera::side(Dir4::E);
    let s = sprite(
        &b,
        &walk,
        0,
        cam,
        pixfx_body::Cell::RULER,
        &Look::mannequin(),
        Layers::ALL,
    );
    assert!(s.parts.ids.iter().any(|i| *i != 0));
}
