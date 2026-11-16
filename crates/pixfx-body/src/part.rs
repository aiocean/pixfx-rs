//! The body's parts: one id each for a part map, one guide colour each, in the reference's scheme (head cyan, chest
//! orange, pelvis brown, her right arm red to pink, her left arm violet, her right leg blues, her left leg greens).

use pixfx_draw::Rgba;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Part {
    Head,
    Neck,
    Chest,
    Pelvis,
    RightUpperArm,
    RightForearm,
    RightHand,
    LeftUpperArm,
    LeftForearm,
    LeftHand,
    RightThigh,
    RightShin,
    RightFoot,
    LeftThigh,
    LeftShin,
    LeftFoot,
    Sword,
}

impl Part {
    pub const ALL: [Part; 17] = [
        Part::Head,
        Part::Neck,
        Part::Chest,
        Part::Pelvis,
        Part::RightUpperArm,
        Part::RightForearm,
        Part::RightHand,
        Part::LeftUpperArm,
        Part::LeftForearm,
        Part::LeftHand,
        Part::RightThigh,
        Part::RightShin,
        Part::RightFoot,
        Part::LeftThigh,
        Part::LeftShin,
        Part::LeftFoot,
        Part::Sword,
    ];

    /// 1 to 17 in [`Part::ALL`]'s order; 0 is an empty pixel in a part map.
    pub fn id(self) -> u8 {
        Part::ALL
            .iter()
            .position(|p| *p == self)
            .map_or(0, |i| i as u8 + 1)
    }

    pub fn from_id(id: u8) -> Option<Part> {
        Part::ALL.get(usize::from(id).checked_sub(1)?).copied()
    }

    pub fn name(self) -> &'static str {
        match self {
            Part::Head => "head",
            Part::Neck => "neck",
            Part::Chest => "chest",
            Part::Pelvis => "pelvis",
            Part::RightUpperArm => "right_upper_arm",
            Part::RightForearm => "right_forearm",
            Part::RightHand => "right_hand",
            Part::LeftUpperArm => "left_upper_arm",
            Part::LeftForearm => "left_forearm",
            Part::LeftHand => "left_hand",
            Part::RightThigh => "right_thigh",
            Part::RightShin => "right_shin",
            Part::RightFoot => "right_foot",
            Part::LeftThigh => "left_thigh",
            Part::LeftShin => "left_shin",
            Part::LeftFoot => "left_foot",
            Part::Sword => "sword",
        }
    }

    /// The part's own colour: flat, bright, one hue a part.
    pub fn guide(self) -> Rgba {
        match self {
            Part::Head => Rgba(110, 232, 212, 255),
            Part::Neck => Rgba(70, 190, 170, 255),
            Part::Chest => Rgba(245, 145, 60, 255),
            Part::Pelvis => Rgba(150, 110, 80, 255),
            Part::RightUpperArm => Rgba(228, 70, 70, 255),
            Part::RightForearm => Rgba(240, 120, 160, 255),
            Part::RightHand => Rgba(250, 185, 205, 255),
            Part::LeftUpperArm => Rgba(140, 90, 215, 255),
            Part::LeftForearm => Rgba(185, 130, 240, 255),
            Part::LeftHand => Rgba(215, 185, 250, 255),
            Part::RightThigh => Rgba(60, 100, 215, 255),
            Part::RightShin => Rgba(85, 155, 240, 255),
            Part::RightFoot => Rgba(150, 210, 250, 255),
            Part::LeftThigh => Rgba(35, 165, 85, 255),
            Part::LeftShin => Rgba(125, 205, 65, 255),
            Part::LeftFoot => Rgba(210, 230, 90, 255),
            Part::Sword => Rgba(175, 178, 190, 255),
        }
    }
}
