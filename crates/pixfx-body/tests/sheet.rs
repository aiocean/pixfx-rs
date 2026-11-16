//! A clip laid out as a sheet: a row per direction, a column per frame, one image per layer, and the frames'
//! rectangles, timing, root motion and joints as data.

use pixfx_body::{Body, Look, View, catalog, clip_sheet};

#[test]
fn a_side_sheet_has_four_rows_and_a_frame_a_column() {
    let Some(run) = catalog::by_name("run") else {
        unreachable!("run is in the catalog")
    };
    let s = clip_sheet(&Body::default(), &run, View::Side, &Look::mannequin());
    let (cw, ch) = (s.meta.cell[0], s.meta.cell[1]);
    assert_eq!(s.meta.directions, ["E", "S", "W", "N"]);
    assert_eq!((s.composite.w, s.composite.h), (cw * run.frames, ch * 4));
    for img in [&s.hull, &s.bones, &s.parts] {
        assert_eq!((img.w, img.h), (s.composite.w, s.composite.h));
    }
    assert_eq!(s.meta.frames.len(), (run.frames * 4) as usize);
    let f = &s.meta.frames[run.frames as usize + 2];
    assert_eq!((f.dir.as_str(), f.index), ("S", 2));
    assert_eq!(f.rect, [2 * cw, ch, cw, ch]);
    assert_eq!(f.duration_ms, 1000 / run.fps);
    assert_eq!(s.meta.tags.len(), 4);
    assert_eq!(
        (s.meta.tags[1].from, s.meta.tags[1].to),
        (run.frames, 2 * run.frames - 1)
    );
}

#[test]
fn top_down_and_iso_sheets_have_eight_rows() {
    let Some(walk) = catalog::by_name("walk") else {
        unreachable!("walk is in the catalog")
    };
    for v in [View::Topdown, View::Iso] {
        let s = clip_sheet(&Body::default(), &walk, v, &Look::mannequin());
        assert_eq!(s.meta.directions.len(), 8);
        assert_eq!(s.composite.h, s.meta.cell[1] * 8);
    }
}

#[test]
fn the_composite_is_the_hull_with_the_bones_over_it() {
    let Some(idle) = catalog::by_name("idle") else {
        unreachable!("idle is in the catalog")
    };
    let s = clip_sheet(&Body::default(), &idle, View::Side, &Look::mannequin());
    let n = (s.composite.w * s.composite.h) as usize;
    let px = |img: &pixfx_draw::RgbaImage, i: usize| {
        [
            img.rgba[4 * i],
            img.rgba[4 * i + 1],
            img.rgba[4 * i + 2],
            img.rgba[4 * i + 3],
        ]
    };
    let mut bones = 0;
    for i in 0..n {
        let (c, h, b, p) = (
            px(&s.composite, i),
            px(&s.hull, i),
            px(&s.bones, i),
            px(&s.parts, i),
        );
        if b[3] != 0 {
            assert_eq!(c, b);
            bones += 1;
        } else {
            assert_eq!(c, h);
        }
        assert_eq!(
            h[3] != 0,
            p[3] != 0,
            "hull and part map cover the same pixels"
        );
    }
    assert!(bones > 0);
}

#[test]
fn every_frame_names_its_joints_inside_its_cell() {
    let Some(walk) = catalog::by_name("walk") else {
        unreachable!("walk is in the catalog")
    };
    let s = clip_sheet(&Body::default(), &walk, View::Side, &Look::mannequin());
    for f in &s.meta.frames {
        for name in ["head", "pelvis", "ground", "right_hand", "left_foot"] {
            let Some(p) = f.joints.get(name) else {
                unreachable!("{name} missing")
            };
            assert!(
                p[0] >= 0.0
                    && p[1] >= 0.0
                    && p[0] <= f64::from(s.meta.cell[0])
                    && p[1] <= f64::from(s.meta.cell[1]),
                "{name} {p:?}"
            );
        }
    }
    let json = serde_json::to_value(&s.meta).unwrap_or_default();
    assert!(json["frames"][0]["root"].is_array());
}
