//! The body's volume as flat shapes on screen: a disc for the head, a capsule for every bone, and for the chest
//! and pelvis a quad between the shoulders or hips (their width) over a capsule down the spine (their thickness),
//! sorted back to front. Geometry only: [`crate::rasterize`] turns it into game pixels, and a guide renderer can
//! draw the same list smooth.

use serde::{Deserialize, Serialize};

use crate::body::Body;
use crate::part::Part;
use crate::rig::{Point, Skeleton};

/// A shape in screen pixels (x right, y down).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Shape {
    /// A line from `a` to `b` swept by a circle whose radius runs from `ra` to `rb`.
    Capsule {
        a: (f64, f64),
        b: (f64, f64),
        ra: f64,
        rb: f64,
    },
    Disc {
        c: (f64, f64),
        r: f64,
    },
    /// A convex quad, corners in order round it (either way).
    Quad([(f64, f64); 4]),
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct HullShape {
    pub part: Part,
    pub shape: Shape,
    /// Toward the viewer: a shape with more depth is drawn over one with less.
    pub depth: f64,
}

fn lerp(a: Point, b: Point, t: f64) -> Point {
    Point {
        x: a.x + (b.x - a.x) * t,
        y: a.y + (b.y - a.y) * t,
        depth: a.depth + (b.depth - a.depth) * t,
    }
}

/// `from` moved by `(to - origin) * k`.
fn offset(from: Point, origin: Point, to: Point, k: f64) -> Point {
    Point {
        x: from.x + (to.x - origin.x) * k,
        y: from.y + (to.y - origin.y) * k,
        depth: from.depth + (to.depth - origin.depth) * k,
    }
}

fn xy(p: Point) -> (f64, f64) {
    (p.x, p.y)
}

fn capsule(part: Part, a: Point, b: Point, ra: f64, rb: f64) -> HullShape {
    HullShape {
        part,
        shape: Shape::Capsule {
            a: xy(a),
            b: xy(b),
            ra,
            rb,
        },
        depth: (a.depth + b.depth) / 2.0,
    }
}

fn quad(part: Part, q: [Point; 4]) -> HullShape {
    HullShape {
        part,
        shape: Shape::Quad(q.map(xy)),
        depth: q.iter().map(|p| p.depth).sum::<f64>() / 4.0,
    }
}

fn finite(s: &HullShape) -> bool {
    let n: Vec<f64> = match s.shape {
        Shape::Capsule { a, b, ra, rb } => vec![a.0, a.1, b.0, b.1, ra, rb],
        Shape::Disc { c, r } => vec![c.0, c.1, r],
        Shape::Quad(q) => q.iter().flat_map(|p| [p.0, p.1]).collect(),
    };
    s.depth.is_finite() && n.iter().all(|v| v.is_finite())
}

/// The hull of a seen skeleton, back to front. A shape with a bad number is left out.
pub fn hull(body: &Body, s: &Skeleton) -> Vec<HullShape> {
    let limb = body.limb_r;
    let half_thick = body.torso_d / 2.0;
    let waist = lerp(s.pelvis, s.neck, 0.25);
    // the chest's corners: inside the shoulder joints, as wide as the torso
    let chest_k = if body.shoulder > 0.0 {
        body.torso_w / 2.0 / body.shoulder
    } else {
        1.0
    };
    let pelvis_k = if body.hip > 0.0 {
        body.pelvis_w / 2.0 / body.hip
    } else {
        1.0
    };
    let (rs, ls) = (s.right_arm.root, s.left_arm.root);
    let (rh, lh) = (s.right_leg.root, s.left_leg.root);
    let mut out = vec![
        HullShape {
            part: Part::Head,
            shape: Shape::Disc {
                c: xy(s.head),
                r: s.head_radius,
            },
            depth: s.head.depth,
        },
        capsule(
            Part::Neck,
            s.neck,
            lerp(s.neck, s.head, 0.5),
            limb * 0.8,
            limb * 0.8,
        ),
        quad(
            Part::Chest,
            [
                offset(s.neck, s.neck, rs, chest_k),
                offset(s.neck, s.neck, ls, chest_k),
                offset(waist, s.pelvis, lh, pelvis_k),
                offset(waist, s.pelvis, rh, pelvis_k),
            ],
        ),
        capsule(Part::Chest, s.neck, waist, half_thick, half_thick),
        quad(
            Part::Pelvis,
            [
                offset(waist, s.pelvis, rh, pelvis_k),
                offset(waist, s.pelvis, lh, pelvis_k),
                offset(s.pelvis, s.pelvis, lh, pelvis_k),
                offset(s.pelvis, s.pelvis, rh, pelvis_k),
            ],
        ),
        capsule(Part::Pelvis, waist, s.pelvis, half_thick, half_thick),
    ];
    let arms = [
        (
            s.right_arm,
            Part::RightUpperArm,
            Part::RightForearm,
            Part::RightHand,
        ),
        (
            s.left_arm,
            Part::LeftUpperArm,
            Part::LeftForearm,
            Part::LeftHand,
        ),
    ];
    for (l, upper, fore, hand) in arms {
        out.push(capsule(upper, l.root, l.mid, limb, limb));
        out.push(capsule(fore, l.mid, l.end, limb, limb * 0.9));
        out.push(capsule(hand, l.end, l.tip, limb * 0.8, limb * 0.6));
    }
    let legs = [
        (
            s.right_leg,
            Part::RightThigh,
            Part::RightShin,
            Part::RightFoot,
        ),
        (s.left_leg, Part::LeftThigh, Part::LeftShin, Part::LeftFoot),
    ];
    for (l, thigh, shin, foot) in legs {
        out.push(capsule(thigh, l.root, l.mid, limb * 1.15, limb));
        out.push(capsule(shin, l.mid, l.end, limb, limb * 0.85));
        out.push(capsule(foot, l.end, l.tip, limb * 0.75, limb * 0.6));
    }
    if let Some([hilt, tip]) = s.sword {
        out.push(capsule(Part::Sword, hilt, tip, 0.5, 0.5));
    }
    out.retain(finite);
    out.sort_by(|a, b| a.depth.total_cmp(&b.depth));
    out
}
