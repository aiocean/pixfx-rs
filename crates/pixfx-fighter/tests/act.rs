//! An act's phases choose the fighter's pose.

use pixfx_core::act::Phase;
use pixfx_fighter::act::{Family, pose_for};
use pixfx_fighter::pose::Pose;

#[test]
fn blade_phases_map_to_draw_slash_guard() {
    assert_eq!(pose_for(Family::Blade, Phase::Windup), Pose::Draw);
    assert_eq!(pose_for(Family::Blade, Phase::Strike), Pose::Slash);
    assert_eq!(pose_for(Family::Blade, Phase::Recover), Pose::Guard);
    assert_eq!(pose_for(Family::Blade, Phase::Done), Pose::Idle);
}

#[test]
fn spell_phases_map_to_channel_cast_guard() {
    assert_eq!(pose_for(Family::Spell, Phase::Windup), Pose::Channel);
    assert_eq!(pose_for(Family::Spell, Phase::Strike), Pose::Cast);
    assert_eq!(pose_for(Family::Spell, Phase::Recover), Pose::Guard);
    assert_eq!(pose_for(Family::Spell, Phase::Done), Pose::Idle);
}
