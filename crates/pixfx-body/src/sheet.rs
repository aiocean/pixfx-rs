//! A clip as a sprite sheet: a row per direction of a view, a column per frame, one image per layer (the
//! composite, the hull alone, the bones alone, the part map in guide colours), and the data a game reads with it.

use std::collections::BTreeMap;

use pixfx_draw::{Frame, Rgba, RgbaImage};
use serde::{Deserialize, Serialize};

use crate::body::Body;
use crate::camera::{Camera, Dir4, Dir8};
use crate::clip::{Clip, Mode};
use crate::paint::{Layers, Look, PartMap};
use crate::part::Part;
use crate::rig::Point;
use crate::sprite::{Cell, clip_cell, sprite};

/// Which camera presets a sheet's rows are.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum View {
    /// A platformer's four facings.
    Side,
    Iso,
    /// A top-down game's eight directions.
    Topdown,
}

impl View {
    /// Each row's direction name and camera.
    pub fn rows(self) -> Vec<(&'static str, Camera)> {
        match self {
            View::Side => Dir4::all().map(|d| (d.name(), Camera::side(d))).to_vec(),
            View::Iso => Dir8::all().map(|d| (d.name(), Camera::iso(d))).to_vec(),
            View::Topdown => Dir8::all().map(|d| (d.name(), Camera::topdown(d))).to_vec(),
        }
    }
    pub fn name(self) -> &'static str {
        match self {
            View::Side => "side",
            View::Iso => "iso",
            View::Topdown => "topdown",
        }
    }
}

/// One frame of the sheet.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SheetFrame {
    pub dir: String,
    pub index: u32,
    /// x, y, w, h on the sheet.
    pub rect: [u32; 4],
    pub duration_ms: u32,
    /// The root's motion on this frame: along the ground toward her front, and height.
    pub root: [f64; 2],
    /// Joints in the cell's pixels.
    pub joints: BTreeMap<String, [f64; 2]>,
}

/// A run of frames, as Aseprite tags them: one per direction.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Tag {
    pub name: String,
    pub from: u32,
    pub to: u32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SheetMeta {
    pub name: String,
    pub view: View,
    pub fps: u32,
    pub looped: bool,
    pub cell: [u32; 2],
    pub pivot: [u32; 2],
    pub directions: Vec<String>,
    pub frames: Vec<SheetFrame>,
    pub tags: Vec<Tag>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ClipSheet {
    pub composite: RgbaImage,
    pub hull: RgbaImage,
    pub bones: RgbaImage,
    pub parts: RgbaImage,
    pub meta: SheetMeta,
}

fn blit(img: &mut RgbaImage, frame: &Frame, x0: u32, y0: u32) {
    for y in 0..frame.h {
        for x in 0..frame.w {
            let c = Rgba::unpack(
                frame
                    .color
                    .get((y * frame.w + x) as usize)
                    .copied()
                    .unwrap_or(0),
            );
            if c.3 != 0 {
                img.set(x0 + x, y0 + y, [c.0, c.1, c.2, c.3]);
            }
        }
    }
}

fn blit_parts(img: &mut RgbaImage, map: &PartMap, x0: u32, y0: u32) {
    for y in 0..map.h {
        for x in 0..map.w {
            if let Some(p) = Part::from_id(map.id(x, y)) {
                let c = p.guide();
                img.set(x0 + x, y0 + y, [c.0, c.1, c.2, 255]);
            }
        }
    }
}

fn joints(s: &crate::rig::Skeleton) -> BTreeMap<String, [f64; 2]> {
    let p = |q: Point| [q.x, q.y];
    [
        ("head", s.head),
        ("neck", s.neck),
        ("pelvis", s.pelvis),
        ("ground", s.ground),
        ("right_shoulder", s.right_arm.root),
        ("right_elbow", s.right_arm.mid),
        ("right_hand", s.right_arm.end),
        ("left_shoulder", s.left_arm.root),
        ("left_elbow", s.left_arm.mid),
        ("left_hand", s.left_arm.end),
        ("right_hip", s.right_leg.root),
        ("right_knee", s.right_leg.mid),
        ("right_foot", s.right_leg.end),
        ("left_hip", s.left_leg.root),
        ("left_knee", s.left_leg.mid),
        ("left_foot", s.left_leg.end),
    ]
    .into_iter()
    .map(|(k, v)| (k.to_string(), p(v)))
    .collect()
}

/// `clip` in every direction of `view`, every frame in one cell that fits them all.
pub fn clip_sheet(body: &Body, clip: &Clip, view: View, look: &Look) -> ClipSheet {
    let rows = view.rows();
    let cams: Vec<Camera> = rows.iter().map(|r| r.1).collect();
    let cell: Cell = clip_cell(body, clip, &cams);
    let frames = clip.frames.max(1);
    let (w, h) = (cell.w * frames, cell.h * rows.len() as u32);
    let mut out = ClipSheet {
        composite: RgbaImage::new(w, h),
        hull: RgbaImage::new(w, h),
        bones: RgbaImage::new(w, h),
        parts: RgbaImage::new(w, h),
        meta: SheetMeta {
            name: clip.name.clone(),
            view,
            fps: clip.fps,
            looped: clip.mode == Mode::Loop,
            cell: [cell.w, cell.h],
            pivot: [cell.pivot.0, cell.pivot.1],
            directions: rows.iter().map(|r| r.0.to_string()).collect(),
            frames: vec![],
            tags: vec![],
        },
    };
    let only = |hull, bones| Layers { hull, bones };
    for (row, (dir, cam)) in rows.iter().enumerate() {
        let y0 = row as u32 * cell.h;
        for f in 0..frames {
            let x0 = f * cell.w;
            let s = sprite(body, clip, f, *cam, cell, look, Layers::ALL);
            blit(&mut out.composite, &s.frame, x0, y0);
            blit_parts(&mut out.parts, &s.parts, x0, y0);
            let hull_only = sprite(body, clip, f, *cam, cell, look, only(true, false));
            blit(&mut out.hull, &hull_only.frame, x0, y0);
            let bones_only = sprite(body, clip, f, *cam, cell, look, only(false, true));
            blit(&mut out.bones, &bones_only.frame, x0, y0);
            let (dx, dy) = clip.root_at(f);
            out.meta.frames.push(SheetFrame {
                dir: dir.to_string(),
                index: f,
                rect: [x0, y0, cell.w, cell.h],
                duration_ms: 1000 / clip.fps.max(1),
                root: [dx, dy],
                joints: joints(&s.skeleton),
            });
        }
        out.meta.tags.push(Tag {
            name: dir.to_string(),
            from: row as u32 * frames,
            to: row as u32 * frames + frames - 1,
        });
    }
    out
}
