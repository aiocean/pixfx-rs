//! A clip's frame as a sprite in a cell: the figure placed with its ground point centred on the cell's feet row,
//! four rows above the bottom (the ruler's row 60 of 64), its hull rasterized and painted.

use pixfx_draw::Frame;

use crate::body::Body;
use crate::camera::Camera;
use crate::clip::Clip;
use crate::hull::{Shape, hull};
use crate::paint::{Layers, Look, PartMap, paint, rasterize};
use crate::rig::{Skeleton, skeleton_seen};

/// Rows under the feet in a cell.
const BELOW_FEET: u32 = 4;
/// The smallest cell: the ruler's.
const MIN_CELL: u32 = 64;

/// One painted frame: its pixels, which part covers each, the placed skeleton and the pivot (the ground point).
#[derive(Clone, Debug, PartialEq)]
pub struct Sprite {
    pub frame: Frame,
    pub parts: PartMap,
    pub skeleton: Skeleton,
    pub pivot: (i32, i32),
}

/// A sprite's cell and where its pivot (the ground point under the pelvis) sits in it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Cell {
    pub w: u32,
    pub h: u32,
    pub pivot: (u32, u32),
}

impl Cell {
    /// The ruler's cell: 64 x 64, the feet on row 60.
    pub const RULER: Cell = Cell {
        w: MIN_CELL,
        h: MIN_CELL,
        pivot: (MIN_CELL / 2, MIN_CELL - BELOW_FEET),
    };
}

/// `clip`'s frame `frame` seen from `camera`, its ground point on the cell's pivot.
pub fn sprite(
    body: &Body,
    clip: &Clip,
    frame: u32,
    camera: Camera,
    cell: Cell,
    look: &Look,
    layers: Layers,
) -> Sprite {
    let s = skeleton_seen(body, &clip.pose_at(body, frame), camera);
    let (px, py) = (f64::from(cell.pivot.0), f64::from(cell.pivot.1));
    let placed = s.shifted(px - s.ground.x, py - s.ground.y);
    let parts = rasterize(&hull(body, &placed), cell.w, cell.h);
    Sprite {
        frame: paint(&parts, &placed, look, layers),
        parts,
        skeleton: placed,
        pivot: (px as i32, py as i32),
    }
}

/// The cell every frame of `clip` from every one of `cameras` fits in: a multiple of 8 each way, never under the
/// ruler's 64, the pivot centred across with at least four rows under it, and more when a part seen from above
/// comes down past the ground point (the near arm of a body lying on the floor).
pub fn clip_cell(body: &Body, clip: &Clip, cameras: &[Camera]) -> Cell {
    let (mut half, mut above, mut below) = (0.0_f64, 0.0_f64, 0.0_f64);
    for f in 0..clip.frames.max(1) {
        let pose = clip.pose_at(body, f);
        for cam in cameras {
            let s = skeleton_seen(body, &pose, *cam);
            let s = s.shifted(-s.ground.x, -s.ground.y);
            for h in hull(body, &s) {
                let pts: Vec<(f64, f64, f64)> = match h.shape {
                    Shape::Capsule { a, b, ra, rb } => vec![(a.0, a.1, ra), (b.0, b.1, rb)],
                    Shape::Disc { c, r } => vec![(c.0, c.1, r)],
                    Shape::Quad(q) => q.iter().map(|p| (p.0, p.1, 0.0)).collect(),
                };
                for (x, y, r) in pts {
                    half = half.max(x.abs() + r);
                    above = above.max(-(y - r));
                    below = below.max(y + r);
                }
            }
        }
    }
    let up8 = |v: f64| {
        let v = libm::ceil(v).clamp(0.0, 4096.0) as u32;
        v.div_ceil(8) * 8
    };
    let w = up8(2.0 * half + 2.0).max(MIN_CELL);
    let under = BELOW_FEET.max(libm::ceil(below).clamp(0.0, 4096.0) as u32 + 1);
    let h = up8(above + 1.0 + f64::from(under)).max(MIN_CELL);
    Cell {
        w,
        h,
        pivot: (w / 2, h - under),
    }
}
