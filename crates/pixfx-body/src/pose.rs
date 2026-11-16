//! A pose as joint angles, in degrees, each relative to the body part it hangs from.
//!
//! Every angle is 0 in the rest pose: standing upright, arms and legs hanging straight down, looking ahead. "Her
//! right" is the figure's own right, whatever way she faces the viewer.

use serde::{Deserialize, Serialize};

/// The figure's own left or right.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Side {
    Right,
    Left,
}

/// An arm (from the shoulder) or a leg (from the hip).
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Limb {
    /// Swing forward from hanging: 90 points straight ahead, 180 straight up, negative swings back.
    pub raise: f64,
    /// Swing out to the figure's side: 90 points straight out sideways.
    pub spread: f64,
    /// Elbow or knee: 0 straight. An elbow folds the forearm forward and up, a knee folds the shin back.
    pub bend: f64,
}

impl Limb {
    pub const fn new(raise: f64, spread: f64, bend: f64) -> Limb {
        Limb {
            raise,
            spread,
            bend,
        }
    }
}

/// The head, from the top of the spine.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Head {
    /// Chin down (positive) or up (negative).
    pub nod: f64,
    /// Look toward her right (positive) or left (negative).
    pub turn: f64,
}

/// A sword held in one hand.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Sword {
    pub hand: Side,
    /// The blade's turn from the line of the forearm, folding the way the elbow folds: 0 continues the forearm,
    /// 90 points the blade square to it, forward and up.
    pub angle: f64,
}

impl Default for Sword {
    fn default() -> Self {
        Sword {
            hand: Side::Right,
            angle: 0.0,
        }
    }
}

/// A whole pose. Fields left out of JSON are 0 (the rest pose), and no sword.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Pose {
    /// Upper body bends forward (positive) or back (negative) at the waist.
    pub lean: f64,
    /// Upper body bends toward her right (positive) or left.
    pub tilt: f64,
    /// Shoulders turn toward her right (positive) or left, about the spine.
    pub twist: f64,
    pub head: Head,
    pub right_arm: Limb,
    pub left_arm: Limb,
    pub right_leg: Limb,
    pub left_leg: Limb,
    /// The whole body pitched back: 90 lies on her back, -90 face down.
    pub lie: f64,
    /// The lowest point above the ground, in pixels (a jump).
    pub lift: f64,
    /// The whole body turned about her forward axis toward her right (a cartwheel): 180 stands her on her head.
    pub roll: f64,
    /// Ankle: toes up (positive) or pointed down (negative) from flat.
    pub right_foot: f64,
    pub left_foot: f64,
    /// Wrist: the hand folds the way the elbow folds (positive) or back.
    pub right_hand: f64,
    pub left_hand: f64,
    pub sword: Option<Sword>,
}

impl Pose {
    /// The same pose with left and right swapped: a pose for the other hand, or the other foot forward.
    pub fn mirrored(&self) -> Pose {
        // Spread is measured outward on each side, so swapping the limbs is the whole of it.
        Pose {
            tilt: -self.tilt,
            twist: -self.twist,
            head: Head {
                nod: self.head.nod,
                turn: -self.head.turn,
            },
            right_arm: self.left_arm,
            left_arm: self.right_arm,
            right_leg: self.left_leg,
            left_leg: self.right_leg,
            roll: -self.roll,
            right_foot: self.left_foot,
            left_foot: self.right_foot,
            right_hand: self.left_hand,
            left_hand: self.right_hand,
            sword: self.sword.map(|s| Sword {
                hand: match s.hand {
                    Side::Right => Side::Left,
                    Side::Left => Side::Right,
                },
                ..s
            }),
            ..*self
        }
    }
}
