//! A pose turned into joint positions in 3D, then seen from a facing.
//!
//! The figure stands in a 3D frame: `f` her forward, `u` up her spine, `r` her right. [`skeleton`] places every
//! joint from the [`Body`] lengths and the [`Pose`] angles, then looks at it from the side the viewer stands on:
//! screen x to the right, screen y down, depth toward the viewer.

use serde::{Deserialize, Serialize};

use crate::body::Body;
use crate::camera::Camera;
use crate::pose::{Limb, Pose, Side};

/// Which way the figure faces the viewer, in degrees: 0 faces screen right (a side view, her right side toward
/// the viewer), 90 faces the viewer, 180 faces screen left, 270 shows her back. Any angle in between turns her.
pub mod facing {
    pub const RIGHT: f64 = 0.0;
    pub const FRONT_RIGHT: f64 = 45.0;
    pub const FRONT: f64 = 90.0;
    pub const FRONT_LEFT: f64 = 135.0;
    pub const LEFT: f64 = 180.0;
    pub const BACK_LEFT: f64 = 225.0;
    pub const BACK: f64 = 270.0;
    pub const BACK_RIGHT: f64 = 315.0;
    /// The eight facings, a turnaround's cells in order.
    pub const ALL: [f64; 8] = [
        RIGHT,
        FRONT_RIGHT,
        FRONT,
        FRONT_LEFT,
        LEFT,
        BACK_LEFT,
        BACK,
        BACK_RIGHT,
    ];

    /// A caption for a facing: the nearest of the eight names.
    pub fn name(deg: f64) -> &'static str {
        const NAMES: [&str; 8] = [
            "facing right, side view",
            "facing front-right, three-quarter view",
            "facing the viewer, front view",
            "facing front-left, three-quarter view",
            "facing left, side view",
            "facing back-left, three-quarter back view",
            "back view",
            "facing back-right, three-quarter back view",
        ];
        let step = libm::round(deg.rem_euclid(360.0) / 45.0) as usize % 8;
        NAMES.get(step).copied().unwrap_or(NAMES[0])
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct V3(f64, f64, f64);

impl V3 {
    fn add(self, o: V3) -> V3 {
        V3(self.0 + o.0, self.1 + o.1, self.2 + o.2)
    }
    fn mul(self, k: f64) -> V3 {
        V3(self.0 * k, self.1 * k, self.2 * k)
    }
    fn dot(self, o: V3) -> f64 {
        self.0 * o.0 + self.1 * o.1 + self.2 * o.2
    }
    fn cross(self, o: V3) -> V3 {
        V3(
            self.1 * o.2 - self.2 * o.1,
            self.2 * o.0 - self.0 * o.2,
            self.0 * o.1 - self.1 * o.0,
        )
    }
    /// The unit vector along `self`, or `fallback` when `self` has no length.
    fn unit_or(self, fallback: V3) -> V3 {
        let n = libm::sqrt(self.dot(self));
        if n > 1e-9 {
            self.mul(1.0 / n)
        } else {
            fallback
        }
    }
    /// `self` turned `deg` degrees about the unit axis `k` (right-handed).
    fn turn(self, k: V3, deg: f64) -> V3 {
        let (s, c) = libm::sincos(deg.to_radians());
        self.mul(c)
            .add(k.cross(self).mul(s))
            .add(k.mul(k.dot(self) * (1.0 - c)))
    }
}

/// A body part's own axes: forward, up, her right.
#[derive(Clone, Copy, Debug)]
struct Frame {
    f: V3,
    u: V3,
    r: V3,
}

impl Frame {
    fn turned(self, k: V3, deg: f64) -> Frame {
        Frame {
            f: self.f.turn(k, deg),
            u: self.u.turn(k, deg),
            r: self.r.turn(k, deg),
        }
    }
    /// Bent forward by `deg` (up toward forward) about its own right.
    fn pitched(self, deg: f64) -> Frame {
        self.turned(self.r, -deg)
    }
    /// Tilted toward its right by `deg` (up toward right) about its own forward.
    fn rolled(self, deg: f64) -> Frame {
        self.turned(self.f, deg)
    }
    /// Turned toward its right by `deg` (forward toward right) about its own up.
    fn yawed(self, deg: f64) -> Frame {
        self.turned(self.u, -deg)
    }
    /// `v` given in this frame's components.
    fn of(self, forward: f64, up: f64, right: f64) -> V3 {
        self.f
            .mul(forward)
            .add(self.u.mul(up))
            .add(self.r.mul(right))
    }
}

/// A joint on screen: `x` right and `y` down from the pelvis, in sprite pixels; `depth` toward the viewer.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Point {
    pub x: f64,
    pub y: f64,
    pub depth: f64,
}

impl Point {
    fn of(v: V3) -> Point {
        Point {
            x: v.0,
            y: -v.1,
            depth: v.2,
        }
    }
    fn shifted(self, dx: f64, dy: f64) -> Point {
        Point {
            x: self.x + dx,
            y: self.y + dy,
            ..self
        }
    }
}

/// An arm or a leg on screen.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct LimbPoints {
    /// Shoulder or hip.
    pub root: Point,
    /// Elbow or knee.
    pub mid: Point,
    /// Hand or foot: the wrist, or the sole under the ankle.
    pub end: Point,
    /// Fingertips or toes.
    #[serde(default)]
    pub tip: Point,
}

impl LimbPoints {
    fn shifted(self, dx: f64, dy: f64) -> LimbPoints {
        LimbPoints {
            root: self.root.shifted(dx, dy),
            mid: self.mid.shifted(dx, dy),
            end: self.end.shifted(dx, dy),
            tip: self.tip.shifted(dx, dy),
        }
    }
}

/// Every joint of a posed figure seen from a facing. After [`Skeleton::placed`] the coordinates are pixels in a
/// cell; from [`skeleton`] they are relative to the pelvis.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Skeleton {
    pub head: Point,
    pub head_radius: f64,
    /// The ground point under the pelvis, where her soles stand (less a pose's `lift`): the pivot a clip keeps
    /// still. On a pitched camera it is the projected point.
    #[serde(default)]
    pub ground: Point,
    /// A point on the face, a head radius ahead of its centre: which way she looks.
    pub nose: Point,
    /// Whether the nose is on the viewer's side of the head (false from behind).
    pub nose_visible: bool,
    /// Top of the spine, between the shoulders.
    pub neck: Point,
    pub pelvis: Point,
    pub right_arm: LimbPoints,
    pub left_arm: LimbPoints,
    pub right_leg: LimbPoints,
    pub left_leg: LimbPoints,
    /// Hilt (the hand) and tip of a held sword.
    pub sword: Option<[Point; 2]>,
    pub facing: f64,
}

/// Each line of a skeleton, the unit a sheet colours.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Bone {
    Neck,
    Spine,
    Shoulders,
    Hips,
    RightUpperArm,
    RightForearm,
    LeftUpperArm,
    LeftForearm,
    RightThigh,
    RightShin,
    LeftThigh,
    LeftShin,
    RightHand,
    LeftHand,
    RightFoot,
    LeftFoot,
    Sword,
}

impl Skeleton {
    /// Every line, start and end.
    pub fn bones(&self) -> Vec<(Bone, Point, Point)> {
        let s = self;
        let mid = |a: Point, b: Point| Point {
            x: (a.x + b.x) / 2.0,
            y: (a.y + b.y) / 2.0,
            depth: (a.depth + b.depth) / 2.0,
        };
        let mut out = vec![
            (Bone::Neck, s.neck, s.head),
            (Bone::Spine, s.neck, s.pelvis),
            (Bone::Shoulders, s.right_arm.root, s.left_arm.root),
            (Bone::Hips, s.right_leg.root, s.left_leg.root),
            (Bone::RightUpperArm, s.right_arm.root, s.right_arm.mid),
            (Bone::RightForearm, s.right_arm.mid, s.right_arm.end),
            (Bone::LeftUpperArm, s.left_arm.root, s.left_arm.mid),
            (Bone::LeftForearm, s.left_arm.mid, s.left_arm.end),
            (Bone::RightThigh, s.right_leg.root, s.right_leg.mid),
            (Bone::RightShin, s.right_leg.mid, s.right_leg.end),
            (Bone::LeftThigh, s.left_leg.root, s.left_leg.mid),
            (Bone::LeftShin, s.left_leg.mid, s.left_leg.end),
            (Bone::RightHand, s.right_arm.end, s.right_arm.tip),
            (Bone::LeftHand, s.left_arm.end, s.left_arm.tip),
            (Bone::RightFoot, s.right_leg.end, s.right_leg.tip),
            (Bone::LeftFoot, s.left_leg.end, s.left_leg.tip),
        ];
        // The neck line stops at the bottom of the head circle.
        if let Some(first) = out.first_mut() {
            first.2 = mid(s.neck, s.head);
        }
        if let Some([hilt, tip]) = s.sword {
            out.push((Bone::Sword, hilt, tip));
        }
        out
    }

    /// Every point that can touch the ground or the cell's edge.
    fn extremes(&self) -> Vec<Point> {
        let mut p: Vec<Point> = self.bones().iter().flat_map(|b| [b.1, b.2]).collect();
        let r = self.head_radius;
        p.push(self.head.shifted(0.0, -r));
        p.push(self.head.shifted(0.0, r));
        p.push(self.head.shifted(-r, 0.0));
        p.push(self.head.shifted(r, 0.0));
        p
    }

    /// The bounding box of the body (not the sword): left, top, right, bottom.
    pub fn body_bounds(&self) -> [f64; 4] {
        let pts: Vec<Point> = self
            .extremes()
            .into_iter()
            .filter(|p| self.sword.is_none_or(|[_, tip]| *p != tip))
            .collect();
        bounds(&pts)
    }

    /// The skeleton moved into a cell `w` x `h` pixels: the pelvis centred across, the lowest point of the body
    /// `lift` pixels above `ground` (a y in the cell).
    pub fn placed(&self, w: f64, ground: f64, lift: f64) -> Skeleton {
        let [_, _, _, bottom] = self.body_bounds();
        let (dx, dy) = (w / 2.0 - self.pelvis.x, ground - lift - bottom);
        let limb = |l: LimbPoints| l.shifted(dx, dy);
        Skeleton {
            head: self.head.shifted(dx, dy),
            ground: self.ground.shifted(dx, dy),
            nose: self.nose.shifted(dx, dy),
            neck: self.neck.shifted(dx, dy),
            pelvis: self.pelvis.shifted(dx, dy),
            right_arm: limb(self.right_arm),
            left_arm: limb(self.left_arm),
            right_leg: limb(self.right_leg),
            left_leg: limb(self.left_leg),
            sword: self
                .sword
                .map(|[a, b]| [a.shifted(dx, dy), b.shifted(dx, dy)]),
            ..*self
        }
    }
}

fn bounds(pts: &[Point]) -> [f64; 4] {
    pts.iter().fold(
        [f64::MAX, f64::MAX, f64::MIN, f64::MIN],
        |[l, t, r, b], p| [l.min(p.x), t.min(p.y), r.max(p.x), b.max(p.y)],
    )
}

/// A non-finite angle (NaN, infinity from a bad file) counts as 0, so a pose always gives a drawable skeleton.
fn fin(deg: f64) -> f64 {
    if deg.is_finite() { deg } else { 0.0 }
}

/// The direction of a limb's first bone and of its second, in `frame`. `fold` is +1 for an elbow (folds forward
/// and up), -1 for a knee (folds back).
fn limb_dirs(frame: Frame, side: f64, l: Limb, fold: f64) -> (V3, V3) {
    let (sr, cr) = libm::sincos(fin(l.raise).to_radians());
    let (ss, cs) = libm::sincos(fin(l.spread).to_radians());
    let swing = frame.of(sr, -cr, 0.0);
    let d = swing.mul(cs).add(frame.r.mul(side * ss)).unit_or(swing);
    let e = bend(frame, d, fin(l.bend) * fold);
    (d, e)
}

/// `d` folded by `deg` within the frame's forward-up plane: positive turns down toward forward, forward toward up.
fn bend(frame: Frame, d: V3, deg: f64) -> V3 {
    let (a, b) = (d.dot(frame.f), d.dot(frame.u));
    let p = frame.of(-b, a, 0.0).unit_or(frame.f);
    let (s, c) = libm::sincos(deg.to_radians());
    d.mul(c).add(p.mul(s)).unit_or(d)
}

/// The joints of `pose` on a figure built as `body`, seen at `facing` degrees (see [`facing`]), relative to the
/// pelvis: [`skeleton_seen`] from a side camera. Place it in a cell with [`Skeleton::placed`].
pub fn skeleton(body: &Body, pose: &Pose, facing_deg: f64) -> Skeleton {
    skeleton_seen(
        body,
        pose,
        Camera {
            yaw: facing_deg,
            pitch: 0.0,
        },
    )
}

/// The joints of `pose` on a figure built as `body`, seen from `camera`, relative to the pelvis.
pub fn skeleton_seen(body: &Body, pose: &Pose, camera: Camera) -> Skeleton {
    let facing_deg = camera.yaw;
    let yaw = fin(facing_deg).to_radians();
    let (sy, cy) = libm::sincos(yaw);
    // The viewer looks along -z: x right, y up, z toward the viewer. A pitched camera looks down: the world turns
    // about screen x, so what is higher comes toward the viewer and what is nearer drops down the screen.
    let (sp, cp) = libm::sincos(fin(camera.pitch).to_radians());
    let view = |v: V3| V3(v.0, v.1 * cp - v.2 * sp, v.1 * sp + v.2 * cp);
    let world = Frame {
        f: view(V3(cy, 0.0, sy)),
        u: view(V3(0.0, 1.0, 0.0)),
        r: view(V3(-sy, 0.0, cy)),
    };
    let up = world.u;
    let (pelvis_frame, torso) = body_frames(world, pose);
    let head_frame = torso.pitched(fin(pose.head.nod)).yawed(fin(pose.head.turn));

    let pelvis = V3(0.0, 0.0, 0.0);
    let neck = pelvis.add(torso.u.mul(body.torso));
    let head = neck.add(head_frame.u.mul(body.neck + body.head));
    let nose_dir = head_frame.f;
    let nose = head.add(nose_dir.mul(body.head));

    let arm = |side: Side| {
        let k = side_sign(side);
        let root = neck.add(torso.r.mul(k * body.shoulder));
        let l = match side {
            Side::Right => pose.right_arm,
            Side::Left => pose.left_arm,
        };
        let (d, e) = limb_dirs(torso, k, l, 1.0);
        let mid = root.add(d.mul(body.upper_arm));
        let end = mid.add(e.mul(body.forearm));
        let wrist = match side {
            Side::Right => pose.right_hand,
            Side::Left => pose.left_hand,
        };
        let tip = end.add(bend(torso, e, fin(wrist)).mul(body.hand));
        (root, mid, end, e, tip)
    };
    let leg3 = |side: Side| {
        let k = side_sign(side);
        let root = pelvis.add(pelvis_frame.r.mul(k * body.hip));
        let l = match side {
            Side::Right => pose.right_leg,
            Side::Left => pose.left_leg,
        };
        let (d, e) = limb_dirs(pelvis_frame, k, l, -1.0);
        let mid = root.add(d.mul(body.thigh));
        let end = mid.add(e.mul(body.shin));
        let ankle = match side {
            Side::Right => pose.right_foot,
            Side::Left => pose.left_foot,
        };
        // Square to the shin, toward her front: flat on the ground when she stands; toes up folds it further.
        let tip = end.add(bend(pelvis_frame, e, 90.0 + fin(ankle)).mul(body.foot));
        [root, mid, end, tip]
    };
    let leg = |side: Side| {
        let [root, mid, end, tip] = leg3(side);
        LimbPoints {
            root: Point::of(root),
            mid: Point::of(mid),
            end: Point::of(end),
            tip: Point::of(tip),
        }
    };
    let (ra, la) = (arm(Side::Right), arm(Side::Left));
    let pts = |a: (V3, V3, V3, V3, V3)| LimbPoints {
        root: Point::of(a.0),
        mid: Point::of(a.1),
        end: Point::of(a.2),
        tip: Point::of(a.4),
    };
    let sword = pose.sword.map(|s| {
        let (hand, fore) = match s.hand {
            Side::Right => (ra.2, ra.3),
            Side::Left => (la.2, la.3),
        };
        let blade = bend(torso, fore, fin(s.angle));
        [Point::of(hand), Point::of(hand.add(blade.mul(body.sword)))]
    });
    // The ground: the lowest point of the body (the sword left out) measured along the world's up, less the lift.
    let low = [
        pelvis,
        ra.0,
        ra.1,
        ra.2,
        ra.4,
        la.0,
        la.1,
        la.2,
        la.4,
        neck,
        head.add(up.mul(-body.head)),
    ]
    .iter()
    .map(|v| v.dot(up))
    .chain(
        [Side::Right, Side::Left]
            .into_iter()
            .flat_map(|s| leg3(s).into_iter().map(|v| v.dot(up))),
    )
    .fold(f64::MAX, f64::min);
    let ground = up.mul(low - fin(pose.lift).max(0.0));
    Skeleton {
        head: Point::of(head),
        head_radius: body.head,
        ground: Point::of(ground),
        nose: Point::of(nose),
        nose_visible: nose_dir.2 > -0.25,
        neck: Point::of(neck),
        pelvis: Point::of(pelvis),
        right_arm: pts(ra),
        left_arm: pts(la),
        right_leg: leg(Side::Right),
        left_leg: leg(Side::Left),
        sword,
        facing: fin(facing_deg),
    }
}

fn side_sign(side: Side) -> f64 {
    match side {
        Side::Right => 1.0,
        Side::Left => -1.0,
    }
}

/// The pelvis's frame and the upper body's, from the world's.
fn body_frames(world: Frame, pose: &Pose) -> (Frame, Frame) {
    let pelvis = world.pitched(-fin(pose.lie)).rolled(fin(pose.roll));
    let torso = pelvis
        .pitched(fin(pose.lean))
        .rolled(fin(pose.tilt))
        .yawed(fin(pose.twist));
    (pelvis, torso)
}

/// One of the four limbs a contact can plant.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Limb4 {
    RightArm,
    LeftArm,
    RightLeg,
    LeftLeg,
}

/// The angles that put `limb`'s end (wrist or sole) on `target`, in the side view's body space (x toward her
/// front at facing 0, y up, depth toward the viewer, the pelvis at 0): two-bone analytic IK in the limb's
/// forward-up plane, its `spread` kept (exact when it is 0). Out of reach, the limb points straight at the target.
/// `None` when the target is not a finite point.
pub(crate) fn reach(body: &Body, pose: &Pose, limb: Limb4, target: [f64; 3]) -> Option<Limb> {
    if !target.iter().all(|v| v.is_finite()) {
        return None;
    }
    let world = Frame {
        f: V3(1.0, 0.0, 0.0),
        u: V3(0.0, 1.0, 0.0),
        r: V3(0.0, 0.0, 1.0),
    };
    let (pelvis, torso) = body_frames(world, pose);
    let (side, arm) = match limb {
        Limb4::RightArm => (Side::Right, true),
        Limb4::LeftArm => (Side::Left, true),
        Limb4::RightLeg => (Side::Right, false),
        Limb4::LeftLeg => (Side::Left, false),
    };
    let k = side_sign(side);
    let (root, frame, l1, l2, old) = if arm {
        let neck = torso.u.mul(body.torso);
        let old = match side {
            Side::Right => pose.right_arm,
            Side::Left => pose.left_arm,
        };
        (
            neck.add(torso.r.mul(k * body.shoulder)),
            torso,
            body.upper_arm,
            body.forearm,
            old,
        )
    } else {
        let old = match side {
            Side::Right => pose.right_leg,
            Side::Left => pose.left_leg,
        };
        (
            pelvis.r.mul(k * body.hip),
            pelvis,
            body.thigh,
            body.shin,
            old,
        )
    };
    let rel = V3(target[0], target[1], target[2]).add(root.mul(-1.0));
    let (tf, tu) = (rel.dot(frame.f), rel.dot(frame.u));
    let phi = libm::atan2(tf, -tu).to_degrees();
    let d = libm::sqrt(tf * tf + tu * tu).clamp((l1 - l2).abs() + 1e-6, l1 + l2);
    let acos = |c: f64| libm::acos(c.clamp(-1.0, 1.0)).to_degrees();
    let inner = acos((l1 * l1 + l2 * l2 - d * d) / (2.0 * l1 * l2));
    let alpha = acos((l1 * l1 + d * d - l2 * l2) / (2.0 * l1 * d));
    // An elbow folds forward, so the upper arm sits behind the line to the target; a knee folds back, so the
    // thigh sits ahead of it.
    let raise = if arm { phi - alpha } else { phi + alpha };
    let bend = 180.0 - inner;
    Some(Limb {
        raise: fin(raise),
        spread: old.spread,
        bend: if bend.abs() < 1e-6 { 0.0 } else { fin(bend) },
    })
}
