//! The pose a fighter takes in each phase of an attack ([`pixfx_core::act::Phase`]).

use pixfx_core::act::Phase;

use crate::pose::Pose;

/// How the attack is made, which picks the poses.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Family {
    Blade,
    Spell,
}

/// The pose for `phase` of an attack of `family`.
pub fn pose_for(family: Family, phase: Phase) -> Pose {
    match (family, phase) {
        (Family::Blade, Phase::Windup) => Pose::Draw,
        (Family::Blade, Phase::Strike) => Pose::Slash,
        (Family::Spell, Phase::Windup) => Pose::Channel,
        (Family::Spell, Phase::Strike) => Pose::Cast,
        (_, Phase::Recover) => Pose::Guard,
        (_, Phase::Done) => Pose::Idle,
    }
}
