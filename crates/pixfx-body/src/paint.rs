//! The hull in game pixels, then painted: each part's flat colour, a light edge toward the light (up and left) and a
//! dark edge away from it, and a one-pixel bone inside each part that hides where another part covers it.

use pixfx_draw::{Frame, Rgba};

use crate::hull::{HullShape, Shape};
use crate::part::Part;
use crate::rig::{Bone, Skeleton};

/// Which part covers each pixel (`0` none, else [`Part::id`]) and its tone (`-1` dark, `0` base, `1` light).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PartMap {
    pub w: u32,
    pub h: u32,
    pub ids: Vec<u8>,
    pub shade: Vec<i8>,
}

impl PartMap {
    /// The part id at `(x, y)`, 0 outside the map.
    pub fn id(&self, x: u32, y: u32) -> u8 {
        if x >= self.w || y >= self.h {
            return 0;
        }
        self.ids
            .get((y * self.w + x) as usize)
            .copied()
            .unwrap_or(0)
    }
    fn id_at(&self, x: i64, y: i64) -> u8 {
        if x < 0 || y < 0 {
            return 0;
        }
        self.id(x as u32, y as u32)
    }
}

/// Distance squared from `p` to the segment `a`-`b`, and how far along it (0 to 1) the nearest point is.
fn to_segment(p: (f64, f64), a: (f64, f64), b: (f64, f64)) -> (f64, f64) {
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let len2 = dx * dx + dy * dy;
    let t = if len2 > 1e-12 {
        (((p.0 - a.0) * dx + (p.1 - a.1) * dy) / len2).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let (qx, qy) = (a.0 + dx * t - p.0, a.1 + dy * t - p.1);
    (qx * qx + qy * qy, t)
}

fn covers(shape: &Shape, p: (f64, f64)) -> bool {
    match *shape {
        Shape::Disc { c, r } => {
            let (dx, dy) = (p.0 - c.0, p.1 - c.1);
            dx * dx + dy * dy <= r * r
        }
        Shape::Capsule { a, b, ra, rb } => {
            let (d2, t) = to_segment(p, a, b);
            let r = ra + (rb - ra) * t;
            d2 <= r * r
        }
        Shape::Quad(q) => {
            let mut sign = 0.0;
            for i in 0..4 {
                let (a, b) = (q[i], q[(i + 1) % 4]);
                let cross = (b.0 - a.0) * (p.1 - a.1) - (b.1 - a.1) * (p.0 - a.0);
                if cross.abs() < 1e-12 {
                    continue;
                }
                if sign == 0.0 {
                    sign = cross.signum();
                } else if cross.signum() != sign {
                    return false;
                }
            }
            sign != 0.0
        }
    }
}

/// Left, top, right, bottom of a shape.
fn extent(shape: &Shape) -> [f64; 4] {
    match *shape {
        Shape::Disc { c, r } => [c.0 - r, c.1 - r, c.0 + r, c.1 + r],
        Shape::Capsule { a, b, ra, rb } => {
            let r = ra.max(rb);
            [
                a.0.min(b.0) - r,
                a.1.min(b.1) - r,
                a.0.max(b.0) + r,
                a.1.max(b.1) + r,
            ]
        }
        Shape::Quad(q) => q.iter().fold(
            [f64::MAX, f64::MAX, f64::MIN, f64::MIN],
            |[l, t, r, b], p| [l.min(p.0), t.min(p.1), r.max(p.0), b.max(p.1)],
        ),
    }
}

fn centre(shape: &Shape) -> (f64, f64) {
    match *shape {
        Shape::Disc { c, .. } => c,
        Shape::Capsule { a, b, .. } => ((a.0 + b.0) / 2.0, (a.1 + b.1) / 2.0),
        Shape::Quad(q) => (
            q.iter().map(|p| p.0).sum::<f64>() / 4.0,
            q.iter().map(|p| p.1).sum::<f64>() / 4.0,
        ),
    }
}

/// The shapes (back to front) in a `w` x `h` map at one game pixel per pixel: a pixel is a shape's when its centre
/// is inside it, a nearer shape covers a farther one, and a shape too thin to hold a pixel centre still takes the
/// pixel under its own centre.
pub fn rasterize(shapes: &[HullShape], w: u32, h: u32) -> PartMap {
    let mut map = PartMap {
        w,
        h,
        ids: vec![0; (w * h) as usize],
        shade: vec![0; (w * h) as usize],
    };
    let clamp = |v: f64, hi: u32| libm::floor(v).clamp(0.0, f64::from(hi)) as u32;
    for s in shapes {
        let id = s.part.id();
        let [l, t, r, b] = extent(&s.shape);
        let (x0, x1) = (clamp(l, w), clamp(r + 1.0, w));
        let (y0, y1) = (clamp(t, h), clamp(b + 1.0, h));
        let mut any = false;
        for y in y0..y1 {
            for x in x0..x1 {
                if covers(&s.shape, (f64::from(x) + 0.5, f64::from(y) + 0.5))
                    && let Some(px) = map.ids.get_mut((y * w + x) as usize)
                {
                    *px = id;
                    any = true;
                }
            }
        }
        if !any {
            let (cx, cy) = centre(&s.shape);
            if cx >= 0.0
                && cy >= 0.0
                && cx < f64::from(w)
                && cy < f64::from(h)
                && let Some(px) = map.ids.get_mut((clamp(cy, h) * w + clamp(cx, w)) as usize)
            {
                *px = id;
            }
        }
    }
    for y in 0..h {
        for x in 0..w {
            let id = map.id(x, y);
            if id == 0 {
                continue;
            }
            let (xi, yi) = (i64::from(x), i64::from(y));
            let lit = map.id_at(xi - 1, yi) != id || map.id_at(xi, yi - 1) != id;
            let dark = map.id_at(xi + 1, yi) != id || map.id_at(xi, yi + 1) != id;
            if let Some(s) = map.shade.get_mut((y * w + x) as usize) {
                *s = match (lit, dark) {
                    (true, false) => 1,
                    (false, true) => -1,
                    _ => 0,
                };
            }
        }
    }
    map
}

/// The colours a mannequin is painted in.
#[derive(Clone, Debug, PartialEq)]
pub struct Look {
    /// Dark, base and light tone per part, in [`Part::ALL`]'s order.
    pub ramps: [[Rgba; 3]; 17],
    /// The bone line; `None` paints none.
    pub bone: Option<Rgba>,
}

impl Look {
    /// The reference's look: each part its [`Part::guide`] colour, a darker and a lighter tone, white bones.
    pub fn mannequin() -> Look {
        let tone = |c: Rgba, k: f64, toward: f64| {
            let f = |v: u8| {
                let v = f64::from(v);
                (v + (toward - v) * k).clamp(0.0, 255.0) as u8
            };
            Rgba(f(c.0), f(c.1), f(c.2), 255)
        };
        Look {
            ramps: Part::ALL.map(|p| {
                let c = p.guide();
                [tone(c, 0.22, 0.0), c, tone(c, 0.18, 255.0)]
            }),
            bone: Some(Rgba(255, 255, 255, 255)),
        }
    }
}

/// Which layers a sprite paints.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Layers {
    pub hull: bool,
    pub bones: bool,
}

impl Layers {
    pub const ALL: Layers = Layers {
        hull: true,
        bones: true,
    };
}

/// The parts a bone's line may show on: it never crosses onto another part.
fn bone_parts(b: Bone) -> &'static [Part] {
    match b {
        Bone::Neck => &[Part::Neck, Part::Head],
        Bone::Spine => &[Part::Chest, Part::Pelvis],
        Bone::Shoulders => &[Part::Chest],
        Bone::Hips => &[Part::Pelvis],
        Bone::RightUpperArm => &[Part::RightUpperArm],
        Bone::RightForearm => &[Part::RightForearm],
        // hands and feet are too small to hold a bone line
        Bone::RightHand | Bone::LeftHand | Bone::RightFoot | Bone::LeftFoot => &[],
        Bone::LeftUpperArm => &[Part::LeftUpperArm],
        Bone::LeftForearm => &[Part::LeftForearm],
        Bone::RightThigh => &[Part::RightThigh],
        Bone::RightShin => &[Part::RightShin],
        Bone::LeftThigh => &[Part::LeftThigh],
        Bone::LeftShin => &[Part::LeftShin],
        Bone::Sword => &[],
    }
}

/// The map painted into a frame of its size: the hull's tones, then the bones over it.
pub fn paint(map: &PartMap, skel: &Skeleton, look: &Look, layers: Layers) -> Frame {
    let mut frame = Frame::new(map.w, map.h, 0, 0);
    if layers.hull {
        for (i, id) in map.ids.iter().enumerate() {
            let Some(p) = Part::from_id(*id) else {
                continue;
            };
            let Some(ramp) = look.ramps.get(usize::from(p.id() - 1)) else {
                continue;
            };
            let tone = match map.shade.get(i).copied().unwrap_or(0) {
                -1 => ramp[0],
                1 => ramp[2],
                _ => ramp[1],
            };
            if let Some(px) = frame.color.get_mut(i) {
                *px = tone.pack();
            }
        }
    }
    let Some(white) = look.bone.filter(|_| layers.bones) else {
        return frame;
    };
    for (bone, a, b) in skel.bones() {
        let allowed = bone_parts(bone);
        let (dx, dy) = (b.x - a.x, b.y - a.y);
        let steps = libm::ceil(dx.abs().max(dy.abs())).clamp(1.0, 4096.0) as u32;
        if !(dx.is_finite() && dy.is_finite() && a.x.is_finite() && a.y.is_finite()) {
            continue;
        }
        for i in 0..=steps {
            let t = f64::from(i) / f64::from(steps);
            let (x, y) = (libm::floor(a.x + dx * t), libm::floor(a.y + dy * t));
            if x < 0.0 || y < 0.0 || x >= f64::from(map.w) || y >= f64::from(map.h) {
                continue;
            }
            let (x, y) = (x as u32, y as u32);
            if Part::from_id(map.id(x, y)).is_some_and(|p| allowed.contains(&p))
                && let Some(px) = frame.color.get_mut((y * map.w + x) as usize)
            {
                *px = white.pack();
            }
        }
    }
    frame
}
