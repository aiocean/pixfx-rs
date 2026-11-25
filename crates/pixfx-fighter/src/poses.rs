//! Every pose of a fighter look as sheet cells, painted and outlined as the duel draws them, for looking a look over
//! or handing it to an artist (or an image model) as reference (`cargo run -p pixfx-fighter --example poses`).

use crate::look::Look;
use crate::paint::{Appearance, SPRITE, outline, paint_at_density};
use crate::pose::Pose;
use pixfx_draw::PixelDensity;

/// Every pose, in the order a sheet lays them out, with its name in lower case.
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

/// The pose named `name` in [`POSES`].
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

/// The moments of a pose a state row shows, in seconds into the pose.
pub const MOMENTS: [f64; 3] = [0.0, 0.2, 0.6];

/// What a hit or a status adds over the look's own colours.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tint {
    Own,
    Flash,
    Ice,
    Shock,
}

/// One row of a states sheet: how the fighter is drawn besides its pose.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct State {
    /// Knocked out.
    pub out: bool,
    /// 1 facing right, -1 facing left.
    pub face: f64,
    pub tint: Tint,
}

impl State {
    /// The row's label: `out right flash`.
    pub fn label(&self) -> String {
        let out = if self.out { "out" } else { "up" };
        let face = if self.face > 0.0 { "right" } else { "left" };
        format!("{out} {face} {:?}", self.tint).to_lowercase()
    }
}

/// Every state a pose is drawn in: out or not, facing right or left, each tint.
pub fn states() -> Vec<State> {
    let mut all = Vec::new();
    for out in [false, true] {
        for face in [1.0, -1.0] {
            for tint in [Tint::Own, Tint::Flash, Tint::Ice, Tint::Shock] {
                all.push(State { out, face, tint });
            }
        }
    }
    all
}

/// `pose` of `look` in each of `states`, a row of [`MOMENTS`] per state, as [`SPRITE`]-square RGBA cells row by
/// row, painted and outlined as the duel draws them.
pub fn state_cells(look: &Look, pose: Pose, states: &[State]) -> Vec<Vec<u8>> {
    state_cells_at_density(look, pose, states, PixelDensity::One)
}

pub fn state_cells_at_density(
    look: &Look,
    pose: Pose,
    states: &[State],
    density: PixelDensity,
) -> Vec<Vec<u8>> {
    let mut cells = Vec::new();
    for s in states {
        for &t in &MOMENTS {
            let a = Appearance {
                pose_t: t,
                face: s.face,
                out: s.out,
                flash: s.tint == Tint::Flash,
                frozen: s.tint == Tint::Ice || pose == Pose::Frozen,
                shocked: s.tint == Tint::Shock,
                ..appearance(pose)
            };
            let mut sprite = paint_at_density(look, &a, density);
            outline(&mut sprite, look.outline);
            cells.push(sprite.to_image().rgba);
        }
    }
    cells
}

/// Each pose of `look` in [`POSES`] order as a [`SPRITE`]-square RGBA cell (straight alpha), painted and outlined
/// as the duel draws it. The poses in `blank` come out empty (transparent).
pub fn pose_cells(look: &Look, blank: &[Pose]) -> Vec<Vec<u8>> {
    pose_cells_at_density(look, blank, PixelDensity::One)
}

pub fn pose_cells_at_density(look: &Look, blank: &[Pose], density: PixelDensity) -> Vec<Vec<u8>> {
    let side = SPRITE * density.factor() as usize;
    POSES
        .iter()
        .map(|&(_, pose)| {
            if blank.contains(&pose) {
                return vec![0; side * side * 4];
            }
            let mut sprite = paint_at_density(look, &appearance(pose), density);
            outline(&mut sprite, look.outline);
            sprite.to_image().rgba
        })
        .collect()
}
