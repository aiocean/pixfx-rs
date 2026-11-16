//! A clip: poses keyed on frames and eased between, played once or looped, with the root's motion along the ground
//! as data (the figure plays in place) and limbs planted on a point for a span of frames.

use serde::{Deserialize, Serialize};

use crate::body::Body;
use crate::pose::{Head, Limb, Pose, Sword};
use crate::rig::{Limb4, reach, skeleton};

/// How a key eases into the next one.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Ease {
    #[default]
    Linear,
    InOut,
    In,
    Out,
    /// Keep this key's pose until the next key.
    Hold,
}

impl Ease {
    fn apply(self, t: f64) -> f64 {
        let t = if t.is_finite() {
            t.clamp(0.0, 1.0)
        } else {
            0.0
        };
        match self {
            Ease::Linear => t,
            Ease::InOut => t * t * (3.0 - 2.0 * t),
            Ease::In => t * t,
            Ease::Out => 1.0 - (1.0 - t) * (1.0 - t),
            Ease::Hold => 0.0,
        }
    }
}

/// A pose on a frame.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Key {
    pub at: u32,
    pub pose: Pose,
    pub ease: Ease,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Mode {
    #[default]
    Loop,
    Once,
}

/// `limb`'s end stays where it was on frame `from` through frame `to` (a span that may wrap round a loop), as the
/// root moves.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Contact {
    pub limb: Limb4,
    pub from: u32,
    pub to: u32,
}

/// An animation. Frames run 0 to `frames - 1`; `root` keys `(frame, dx along the ground toward her front, dy
/// height)` in pixels, eased linearly.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Clip {
    pub name: String,
    pub frames: u32,
    pub fps: u32,
    pub mode: Mode,
    pub keys: Vec<Key>,
    pub root: Vec<(u32, f64, f64)>,
    pub contacts: Vec<Contact>,
}

impl Default for Clip {
    fn default() -> Self {
        Clip {
            name: String::new(),
            frames: 1,
            fps: 12,
            mode: Mode::Loop,
            keys: vec![],
            root: vec![],
            contacts: vec![],
        }
    }
}

impl Clip {
    /// The frame within the clip: wrapped in a loop, held at the last in a one-shot.
    fn norm(&self, frame: u32) -> u32 {
        let n = self.frames.max(1);
        match self.mode {
            Mode::Loop => frame % n,
            Mode::Once => frame.min(n - 1),
        }
    }

    /// The keys around `f` and how far between them, for keys at `ats` (sorted).
    fn between(&self, ats: &[u32], f: u32) -> Option<(usize, usize, f64)> {
        let last = ats.len().checked_sub(1)?;
        let n = f64::from(self.frames.max(1));
        let (ff, a0, al) = (f64::from(f), f64::from(ats[0]), f64::from(ats[last]));
        if let Some(i) = ats.windows(2).position(|w| w[0] <= f && f < w[1]) {
            let (a, b) = (f64::from(ats[i]), f64::from(ats[i + 1]));
            return Some((i, i + 1, (ff - a) / (b - a)));
        }
        match self.mode {
            Mode::Once => Some(if f < ats[0] {
                (0, 0, 0.0)
            } else {
                (last, last, 0.0)
            }),
            // Past the last key the loop eases back into the first, which comes again at frames + its own frame.
            Mode::Loop => {
                let span = n - al + a0;
                let into = if ff >= al { ff - al } else { ff + n - al };
                Some((last, 0, if span > 0.0 { into / span } else { 0.0 }))
            }
        }
    }

    /// The keyed pose on `frame`, before any contact.
    fn keyed(&self, frame: u32) -> Pose {
        let mut keys = self.keys.clone();
        keys.sort_by_key(|k| k.at);
        let ats: Vec<u32> = keys.iter().map(|k| k.at).collect();
        let Some((i, j, t)) = self.between(&ats, self.norm(frame)) else {
            return Pose::default();
        };
        let (Some(a), Some(b)) = (keys.get(i), keys.get(j)) else {
            return Pose::default();
        };
        if i == j {
            return a.pose;
        }
        lerp_pose(&a.pose, &b.pose, a.ease.apply(t))
    }

    /// The pose on `frame`: keys eased, then every planted limb moved back onto its point.
    pub fn pose_at(&self, body: &Body, frame: u32) -> Pose {
        let f = self.norm(frame);
        let mut pose = self.keyed(f);
        for c in &self.contacts {
            let on = if c.from <= c.to {
                (c.from..=c.to).contains(&f)
            } else {
                f >= c.from || f <= c.to
            };
            if !on || f == c.from {
                continue;
            }
            let s = skeleton(body, &self.keyed(c.from), 0.0);
            let end = match c.limb {
                Limb4::RightArm => s.right_arm.end,
                Limb4::LeftArm => s.left_arm.end,
                Limb4::RightLeg => s.right_leg.end,
                Limb4::LeftLeg => s.left_leg.end,
            };
            let ((x0, y0), (x1, y1)) = (self.root_at(c.from), self.root_at(f));
            let target = [end.x + x0 - x1, -end.y + y0 - y1, end.depth];
            if let Some(l) = reach(body, &pose, c.limb, target) {
                match c.limb {
                    Limb4::RightArm => pose.right_arm = l,
                    Limb4::LeftArm => pose.left_arm = l,
                    Limb4::RightLeg => pose.right_leg = l,
                    Limb4::LeftLeg => pose.left_leg = l,
                }
            }
        }
        pose
    }

    /// The root's motion on `frame`: `(dx, dy)` in pixels, 0 where nothing is keyed or a number is bad.
    pub fn root_at(&self, frame: u32) -> (f64, f64) {
        let mut root = self.root.clone();
        root.sort_by_key(|r| r.0);
        let ats: Vec<u32> = root.iter().map(|r| r.0).collect();
        let Some((i, j, t)) = self.between(&ats, self.norm(frame)) else {
            return (0.0, 0.0);
        };
        let (Some(a), Some(b)) = (root.get(i), root.get(j)) else {
            return (0.0, 0.0);
        };
        let fin = |v: f64| if v.is_finite() { v } else { 0.0 };
        let t = if i == j { 0.0 } else { t };
        (fin(lerp(a.1, b.1, t)), fin(lerp(a.2, b.2, t)))
    }
}

fn lerp(a: f64, b: f64, t: f64) -> f64 {
    a + (b - a) * t
}

fn lerp_limb(a: Limb, b: Limb, t: f64) -> Limb {
    Limb {
        raise: lerp(a.raise, b.raise, t),
        spread: lerp(a.spread, b.spread, t),
        bend: lerp(a.bend, b.bend, t),
    }
}

/// Every angle and distance of two poses mixed; a sword changing hands switches at the second key.
fn lerp_pose(a: &Pose, b: &Pose, t: f64) -> Pose {
    let sword = match (a.sword, b.sword) {
        (Some(x), Some(y)) if x.hand == y.hand => Some(Sword {
            hand: x.hand,
            angle: lerp(x.angle, y.angle, t),
        }),
        (x, y) => {
            if t < 1.0 {
                x
            } else {
                y
            }
        }
    };
    Pose {
        lean: lerp(a.lean, b.lean, t),
        tilt: lerp(a.tilt, b.tilt, t),
        twist: lerp(a.twist, b.twist, t),
        head: Head {
            nod: lerp(a.head.nod, b.head.nod, t),
            turn: lerp(a.head.turn, b.head.turn, t),
        },
        right_arm: lerp_limb(a.right_arm, b.right_arm, t),
        left_arm: lerp_limb(a.left_arm, b.left_arm, t),
        right_leg: lerp_limb(a.right_leg, b.right_leg, t),
        left_leg: lerp_limb(a.left_leg, b.left_leg, t),
        lie: lerp(a.lie, b.lie, t),
        lift: lerp(a.lift, b.lift, t),
        roll: lerp(a.roll, b.roll, t),
        right_foot: lerp(a.right_foot, b.right_foot, t),
        left_foot: lerp(a.left_foot, b.left_foot, t),
        right_hand: lerp(a.right_hand, b.right_hand, t),
        left_hand: lerp(a.left_hand, b.left_hand, t),
        sword,
    }
}
