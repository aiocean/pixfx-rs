//! The mannequin painted: part colours, bones that hide behind the body, every direction turned in 3D.

#![cfg(feature = "body")]

use pixfx_core::body::{
    Body, Camera, Cell, Clip, Dir4, Dir8, Key, Layers, Look, Mode, Part, Pose, Shape, Sprite,
    catalog, clip_cell, hull, sprite,
};
use pixfx_draw::Rgba;

fn clip(name: &str) -> Clip {
    catalog::by_name(name).unwrap_or_default()
}

fn idle_at(cam: Camera, layers: Layers) -> Sprite {
    sprite(
        &Body::default(),
        &clip("idle"),
        0,
        cam,
        Cell::RULER,
        &Look::mannequin(),
        layers,
    )
}

fn colour(s: &Sprite, x: u32, y: u32) -> Rgba {
    Rgba::unpack(s.frame.color[(y * s.frame.w + x) as usize])
}

#[test]
fn every_painted_pixel_is_a_tone_of_its_parts_ramp() {
    let look = Look::mannequin();
    let s = idle_at(
        Camera::side(Dir4::S),
        Layers {
            hull: true,
            bones: false,
        },
    );
    let mut seen = 0;
    for y in 0..Cell::RULER.h {
        for x in 0..Cell::RULER.w {
            let Some(p) = Part::from_id(s.parts.id(x, y)) else {
                assert_eq!(colour(&s, x, y), Rgba::NONE, "empty at {x},{y}");
                continue;
            };
            let ramp = look.ramps[usize::from(p.id() - 1)];
            assert!(ramp.contains(&colour(&s, x, y)), "{p:?} at {x},{y}");
            seen += 1;
        }
    }
    assert!(seen > 150);
    assert!(s.frame.light.iter().all(|l| *l == 0));
}

#[test]
fn a_bone_shows_only_on_its_own_part() {
    let look = Look::mannequin();
    let Some(white) = look.bone else {
        unreachable!("white is in the catalog and the look")
    };
    // side on, the far (left) arm held straight ahead runs behind the chest: its bone line crosses chest pixels
    let reach = Clip {
        name: "reach".into(),
        frames: 1,
        mode: Mode::Once,
        keys: vec![Key {
            at: 0,
            pose: Pose {
                left_arm: pixfx_core::body::Limb::new(90.0, 0.0, 0.0),
                ..Pose::default()
            },
            ease: Default::default(),
        }],
        ..Clip::default()
    };
    let s = sprite(
        &Body::default(),
        &reach,
        0,
        Camera::side(Dir4::E),
        Cell::RULER,
        &look,
        Layers::ALL,
    );
    let (a, b) = (s.skeleton.left_arm.root, s.skeleton.left_arm.mid);
    let spine_x = libm::floor(s.skeleton.pelvis.x) as u32;
    let mut hidden = 0;
    for i in 0..=32 {
        let t = f64::from(i) / 32.0;
        let (x, y) = (
            libm::floor(a.x + (b.x - a.x) * t) as u32,
            libm::floor(a.y + (b.y - a.y) * t) as u32,
        );
        if s.parts.id(x, y) == Part::Chest.id() && x != spine_x {
            hidden += 1;
            assert!(
                colour(&s, x, y) != white,
                "the far arm's bone shows through the chest at {x},{y}"
            );
        }
    }
    assert!(hidden > 0, "the far arm runs behind the chest");
    // from the front the spine shows on the chest
    let front = idle_at(Camera::side(Dir4::S), Layers::ALL);
    let on_chest = (0..Cell::RULER.w * Cell::RULER.h)
        .filter(|i| {
            front.parts.ids[*i as usize] == Part::Chest.id()
                && colour(&front, i % Cell::RULER.w, i / Cell::RULER.w) == white
        })
        .count();
    assert!(on_chest > 0);
}

#[test]
fn bones_off_leaves_only_hull_colours_and_bones_on_adds_white() {
    let white = Look::mannequin().bone.unwrap_or(Rgba::NONE);
    let off = idle_at(
        Camera::side(Dir4::S),
        Layers {
            hull: true,
            bones: false,
        },
    );
    let on = idle_at(Camera::side(Dir4::S), Layers::ALL);
    let count = |s: &Sprite| s.frame.color.iter().filter(|c| **c == white.pack()).count();
    assert_eq!(count(&off), 0);
    assert!(count(&on) > 20);
    let bones_only = idle_at(
        Camera::side(Dir4::S),
        Layers {
            hull: false,
            bones: true,
        },
    );
    assert!(
        bones_only
            .frame
            .color
            .iter()
            .all(|c| *c == 0 || *c == white.pack())
    );
}

#[test]
fn west_is_rendered_not_flipped() {
    let b = Body::default();
    let walk = clip("walk");
    let at = |d: Dir4| {
        sprite(
            &b,
            &walk,
            2,
            Camera::side(d),
            Cell::RULER,
            &Look::mannequin(),
            Layers::ALL,
        )
    };
    let (e, w) = (at(Dir4::E), at(Dir4::W));
    let mirrored = (0..Cell::RULER.h).all(|y| {
        (0..Cell::RULER.w).all(|x| e.parts.id(x, y) == w.parts.id(Cell::RULER.w - 1 - x, y))
    });
    assert!(
        !mirrored,
        "the near leg differs, so west is not east flipped"
    );
}

fn inside(s: &Sprite, b: &Body, cell: Cell) -> bool {
    hull(b, &s.skeleton).iter().all(|h| {
        let pts: Vec<(f64, f64, f64)> = match h.shape {
            Shape::Capsule { a, b, ra, rb } => vec![(a.0, a.1, ra), (b.0, b.1, rb)],
            Shape::Disc { c, r } => vec![(c.0, c.1, r)],
            Shape::Quad(q) => q.iter().map(|p| (p.0, p.1, 0.0)).collect(),
        };
        pts.iter().all(|(x, y, r)| {
            x - r >= 0.0 && y - r >= 0.0 && x + r <= f64::from(cell.w) && y + r <= f64::from(cell.h)
        })
    })
}

#[test]
fn a_tumble_from_above_stays_in_its_cell() {
    let b = Body::default();
    let roll = Clip {
        name: "roll".into(),
        frames: 1,
        mode: Mode::Once,
        keys: vec![Key {
            at: 0,
            pose: Pose {
                roll: 180.0,
                ..Pose::default()
            },
            ease: Default::default(),
        }],
        ..Clip::default()
    };
    for c in [clip("pose/down"), roll, clip("pose/jump")] {
        let cams: Vec<Camera> = Dir8::all().into_iter().map(Camera::topdown).collect();
        let cell = clip_cell(&b, &c, &cams);
        assert!(
            cell.w.is_multiple_of(8)
                && cell.h.is_multiple_of(8)
                && cell.w >= Cell::RULER.w
                && cell.h >= Cell::RULER.h,
            "{cell:?}"
        );
        for cam in cams {
            let s = sprite(&b, &c, 0, cam, cell, &Look::mannequin(), Layers::ALL);
            assert!(inside(&s, &b, cell), "{} {cam:?} {cell:?}", c.name);
        }
    }
}

#[test]
fn a_bad_pose_still_paints() {
    let b = Body::default();
    let bad = Clip {
        name: "bad".into(),
        frames: 1,
        mode: Mode::Once,
        keys: vec![Key {
            at: 0,
            pose: Pose {
                lean: f64::NAN,
                lift: f64::INFINITY,
                ..Pose::default()
            },
            ease: Default::default(),
        }],
        ..Clip::default()
    };
    let s = sprite(
        &b,
        &bad,
        0,
        Camera::iso(Dir8::S),
        Cell::RULER,
        &Look::mannequin(),
        Layers::ALL,
    );
    assert!(s.parts.ids.iter().any(|i| *i != 0));
}
