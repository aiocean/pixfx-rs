//! Every animation by name, in families. Each clip is our own key poses, authored on the ruler's figure
//! ([`Body::default`]); a planted foot is placed by where it stands, so a stride's length and its root motion agree.

mod ground;

use super::Body;
use super::clip::{Clip, Key, Mode};
use super::pose::{Limb, Pose};
use super::presets;
use super::rig::{Limb4, reach};

/// Every clip name [`by_name`] knows, family by family.
pub const NAMES: &[&str] = &[
    // ground
    "idle",
    "walk",
    "run",
    "run_start",
    "run_stop",
    "sprint",
    "skid",
    "turn",
    "crouch",
    "crouch_walk",
    // combat: the sword fighter's still poses
    "pose/rest",
    "pose/idle",
    "pose/walk",
    "pose/run",
    "pose/guard",
    "pose/windup",
    "pose/slash",
    "pose/thrust",
    "pose/hurt",
    "pose/kneel",
    "pose/crouch",
    "pose/jump",
    "pose/down",
    "pose/cast",
];

/// A clip by name; `None` for a name not in [`NAMES`].
pub fn by_name(name: &str) -> Option<Clip> {
    if let Some(pose) = name.strip_prefix("pose/") {
        return still(name, presets::by_name(pose, 0.0)?);
    }
    let mut clip = ground::by_name(name)?;
    clip.name = name.to_string();
    Some(clip)
}

/// The family a clip belongs to: `"ground"` or `"combat"`.
pub fn family(name: &str) -> Option<&'static str> {
    if !NAMES.contains(&name) {
        return None;
    }
    Some(if name.starts_with("pose/") {
        "combat"
    } else {
        "ground"
    })
}

/// A preset as a one-frame clip.
fn still(name: &str, pose: Pose) -> Option<Clip> {
    Some(Clip {
        name: name.to_string(),
        frames: 1,
        mode: Mode::Once,
        keys: vec![Key {
            at: 0,
            pose,
            ease: Default::default(),
        }],
        ..Clip::default()
    })
}

/// Thigh and shin of the figure the catalog is authored on.
fn leg_length() -> f64 {
    let b = Body::default();
    b.thigh + b.shin
}

/// `pose` with `limb`'s sole at `x` ahead of the pelvis and `h` below it, its foot flat.
fn foot_at(pose: Pose, limb: Limb4, x: f64, h: f64) -> Pose {
    let Some(l) = reach(&Body::default(), &pose, limb, [x, -h, 0.0]) else {
        return pose;
    };
    let mut p = pose;
    match limb {
        Limb4::RightLeg => p.right_leg = l,
        Limb4::LeftLeg => p.left_leg = l,
        Limb4::RightArm => p.right_arm = l,
        Limb4::LeftArm => p.left_arm = l,
    }
    flat(p)
}

/// Both feet level with the ground: each ankle takes up its shin's slope (a knee folds the shin back).
fn flat(p: Pose) -> Pose {
    let level = |l: Limb| l.bend - l.raise;
    Pose {
        right_foot: level(p.right_leg),
        left_foot: level(p.left_leg),
        ..p
    }
}
