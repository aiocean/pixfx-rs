//! `pixfx poses`: every pose of a fighter look side by side on one sheet, painted and outlined as the duel draws
//! them, for looking a look over or handing it to an artist (or an image model) as reference.

use pixfx_fighter::look::Look;
use pixfx_fighter::paint::{Appearance, SPRITE, outline, paint};
use pixfx_fighter::pose::Pose;

/// Every pose, in the order a sheet lays them out, with the name the command line knows it by.
pub const POSES: [(&str, Pose); 24] = [
    ("idle", Pose::Idle),
    ("walk", Pose::Walk),
    ("jump", Pose::Jump),
    ("fly", Pose::Fly),
    ("land", Pose::Land),
    ("crouch", Pose::Crouch),
    ("duck", Pose::Duck),
    ("draw", Pose::Draw),
    ("slash", Pose::Slash),
    ("channel", Pose::Channel),
    ("cast", Pose::Cast),
    ("guard", Pose::Guard),
    ("meditate", Pose::Meditate),
    ("hurt", Pose::Hurt),
    ("frozen", Pose::Frozen),
    ("shocked", Pose::Shocked),
    ("launched", Pose::Launched),
    ("tumble", Pose::Tumble),
    ("fall", Pose::Fall),
    ("down", Pose::Down),
    ("roll", Pose::Roll),
    ("sit", Pose::Sit),
    ("kneel", Pose::Kneel),
    ("win", Pose::Win),
];

/// The pose the command line calls `name`.
pub fn pose_named(name: &str) -> Option<Pose> {
    POSES.iter().find(|(n, _)| *n == name).map(|&(_, p)| p)
}

/// A fighter in `pose`, facing right, standing still on the ground a moment into the pose.
fn appearance(pose: Pose) -> Appearance {
    Appearance {
        pose,
        pose_t: 0.2,
        face: 1.0,
        x: 0.0,
        air: 0.0,
        vx: 0.0,
        vy: 0.0,
        out: false,
        flash: false,
        untouchable: false,
        frozen: pose == Pose::Frozen,
        shocked: false,
        hit_t: 0.2,
        hit_power: 6.0,
        dodged: 0.0,
        act: None,
        jolt: [0, 0],
    }
}

/// Each pose of `look` in [`POSES`] order as a [`SPRITE`]-square RGBA cell (straight alpha), painted and outlined
/// as the duel draws it. The poses in `blank` come out empty (transparent).
pub fn pose_cells(look: &Look, blank: &[Pose]) -> Vec<Vec<u8>> {
    POSES
        .iter()
        .map(|&(_, pose)| {
            if blank.contains(&pose) {
                return vec![0; SPRITE * SPRITE * 4];
            }
            let mut sprite = paint(look, &appearance(pose));
            outline(&mut sprite, look.outline);
            sprite.to_image().rgba
        })
        .collect()
}
