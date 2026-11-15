//! A sheet as an image model sees it: every limb in its own colour, a caption over each cell, the grid `cut` reads.

use pixfx_pixel::sheet::{cell_rect, size};
use pixfx_pose::{Bone, Cell, PoseSheet, colour, facing, presets, spec, turnaround};

fn colours_in(img: &pixfx_draw::RgbaImage, [x, y, w, h]: [u32; 4]) -> Vec<[u8; 4]> {
    let mut out = Vec::new();
    for yy in y..y + h {
        for xx in x..x + w {
            if let Some(p) = img.get(xx, yy)
                && !out.contains(&p)
            {
                out.push(p);
            }
        }
    }
    out
}

fn cell(caption: &str, pose: pixfx_pose::Pose, facing: f64) -> Cell {
    Cell {
        caption: caption.into(),
        pose,
        facing,
    }
}

#[test]
fn every_bone_shows_in_its_own_colour_from_the_front() {
    let sheet = PoseSheet {
        cells: vec![cell("GUARD", presets::guard(), facing::FRONT)],
        ..PoseSheet::default()
    };
    let drawn = sheet.render().unwrap_or_else(|e| unreachable!("{e}"));
    let (w, h) = size(&drawn.layout);
    assert_eq!((drawn.image.w, drawn.image.h), (w, h));
    let seen = colours_in(&drawn.image, cell_rect(&drawn.layout, 0, 0));
    for bone in [
        Bone::Spine,
        Bone::RightUpperArm,
        Bone::RightForearm,
        Bone::LeftUpperArm,
        Bone::LeftForearm,
        Bone::RightThigh,
        Bone::RightShin,
        Bone::LeftThigh,
        Bone::LeftShin,
        Bone::Sword,
    ] {
        let c = colour(bone);
        assert!(seen.contains(&[c.0, c.1, c.2, c.3]), "{bone:?} missing");
    }
}

#[test]
fn the_caption_is_written_in_the_gap_above_its_cell() {
    let sheet = PoseSheet {
        cells: vec![cell("IDLE: at ease", presets::idle(), facing::RIGHT)],
        ..PoseSheet::default()
    };
    let drawn = sheet.render().unwrap_or_else(|e| unreachable!("{e}"));
    let [x, y, w, _] = cell_rect(&drawn.layout, 0, 0);
    let band = drawn.layout.gap * drawn.layout.scale;
    let ink = colours_in(&drawn.image, [x, y - band, w, band - 1]);
    assert!(
        ink.contains(&[20, 20, 20, 255]),
        "caption ink above the cell"
    );

    let quiet = PoseSheet {
        cells: vec![cell("", presets::idle(), facing::RIGHT)],
        ..PoseSheet::default()
    };
    let drawn = quiet.render().unwrap_or_else(|e| unreachable!("{e}"));
    assert!(!colours_in(&drawn.image, [x, y - band, w, band - 1]).contains(&[20, 20, 20, 255]));
}

#[test]
fn a_caption_too_long_for_its_gap_names_the_cell() {
    let long =
        "a caption far too long to fit two short lines of the narrow gap above a small cell "
            .repeat(4);
    let sheet = PoseSheet {
        cells: vec![
            cell("ok", presets::idle(), 0.0),
            cell(&long, presets::idle(), 0.0),
        ],
        gap: 4,
        scale: 4,
        ..PoseSheet::default()
    };
    let err = sheet.render().err().unwrap_or_default();
    assert!(err.starts_with("cell 2 (1,0)"), "{err}");
}

#[test]
fn rows_follow_the_cell_count_and_a_turnaround_has_eight_facings() {
    let cells = turnaround("IDLE", &presets::idle());
    assert_eq!(cells.len(), 8);
    assert_eq!(cells[2].facing, facing::FRONT);
    assert!(cells[6].caption.contains("back view"));
    let sheet = PoseSheet {
        cells,
        cols: 3,
        ..PoseSheet::default()
    };
    assert_eq!((sheet.layout().cols, sheet.layout().rows), (3, 3));
}

#[test]
fn a_spec_lays_a_pose_over_its_preset() {
    let s = spec::parse(
        r#"{"cols": 2, "cells": [
            {"caption": "GUARD", "preset": "guard", "pose": {"head": {"turn": -40}}, "facing": 45},
            {"caption": "REST WITH SWORD", "pose": {"sword": {"angle": 20}}},
            {"caption": "IDLE", "preset": "idle", "turnaround": true}
        ]}"#,
    )
    .unwrap_or_else(|e| unreachable!("{e}"));
    assert_eq!(s.cells.len(), 10);
    assert_eq!(
        s.cells[0].pose.right_arm,
        presets::guard().right_arm,
        "the preset stays"
    );
    assert_eq!(s.cells[0].pose.head.turn, -40.0, "the override lands");
    assert_eq!(s.cells[0].facing, 45.0);
    assert_eq!(s.cells[1].pose.sword.map(|w| w.angle), Some(20.0));
    assert_eq!(s.cols, 2);
}

#[test]
fn a_spec_refuses_what_it_does_not_know() {
    let e = spec::parse(r#"{"cells": [{"preset": "moonwalk"}]}"#)
        .err()
        .unwrap_or_default();
    assert!(e.contains("cell 1") && e.contains("guard"), "{e}");
    let e = spec::parse(r#"{"cells": [{"pose": {"right_arm": {"raize": 3}}}]}"#)
        .err()
        .unwrap_or_default();
    assert!(e.contains("raize"), "{e}");
}
