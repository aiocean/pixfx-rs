//! What the policy can do: nine plain acts and a cast from each slot at one of three aims, `ACTIONS` in all, each
//! with a fixed index; which of them can happen now (`legal`); and the `Choice` each one is.

use super::ahead;
use crate::choice::Choice;
use crate::sense::{Foe, Sense};
use crate::skill::{Skill, SkillKind};

/// The castable slots a game lays out (the duel: three basics, three bar slots, the ultimate).
pub const SLOTS: usize = 7;
const PLAIN: usize = 9;
const AIMS: usize = 3;
pub const ACTIONS: usize = PLAIN + SLOTS * AIMS;

/// How far ahead of or behind the foe an aimed cast lands, game pixels.
const AIM: f64 = 30.0;
/// How far from the foe a blink lands beside it.
const BESIDE: f64 = 20.0;
/// How high it flies, as a share of the ceiling.
const FLY: f64 = 0.6;
/// How long a duck or guard is held, seconds.
const HOLD: f64 = 0.4;

/// Where a cast aims, seen from the fighter: at the foe, past it, or short of it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Aim {
    Foe,
    Ahead,
    Behind,
}

/// One thing the policy can do. Directions are relative to the foe: `Toward` walks at it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Act {
    Wait,
    Toward,
    Away,
    Jump,
    Duck,
    Guard,
    FlyHigh,
    Land,
    Meditate,
    Cast { slot: usize, aim: Aim },
}

const PLAINS: [Act; PLAIN] = [
    Act::Wait,
    Act::Toward,
    Act::Away,
    Act::Jump,
    Act::Duck,
    Act::Guard,
    Act::FlyHigh,
    Act::Land,
    Act::Meditate,
];
const AIMED: [Aim; AIMS] = [Aim::Foe, Aim::Ahead, Aim::Behind];

impl Act {
    pub fn index(self) -> usize {
        match self {
            Act::Cast { slot, aim } => {
                PLAIN + slot * AIMS + AIMED.iter().position(|&a| a == aim).unwrap_or(0)
            }
            plain => PLAINS.iter().position(|&a| a == plain).unwrap_or(0),
        }
    }

    pub fn from_index(i: usize) -> Option<Act> {
        if i < PLAIN {
            return PLAINS.get(i).copied();
        }
        let c = i - PLAIN;
        let slot = c / AIMS;
        if slot >= SLOTS {
            return None;
        }
        AIMED.get(c % AIMS).map(|&aim| Act::Cast { slot, aim })
    }
}

/// Whether a skill of `kind` lands where it is aimed (the others take `Aim::Foe` alone).
fn aims(kind: SkillKind) -> bool {
    matches!(kind, SkillKind::Area | SkillKind::Blink)
}

/// The skill in `slot`, with its index in `skills`.
fn slotted<'a>(sense: &Sense, skills: &'a [Skill], slot: usize) -> Option<(usize, &'a Skill)> {
    let k = sense.slots.get(slot).copied().flatten()?;
    skills.get(k).map(|s| (k, s))
}

/// Which acts can happen now: waiting always; the rest only while it can act; a cast only from a filled slot whose
/// skill is ready, and aimed off the foe only for areas and blinks.
pub fn legal(sense: &Sense, skills: &[Skill]) -> [bool; ACTIONS] {
    let me = &sense.me;
    let mut out = [false; ACTIONS];
    for (i, o) in out.iter_mut().enumerate() {
        let Some(act) = Act::from_index(i) else {
            continue;
        };
        *o = match act {
            Act::Wait => true,
            _ if !me.can_act => false,
            Act::Toward | Act::Away | Act::Duck | Act::Guard | Act::Meditate => true,
            Act::Jump | Act::FlyHigh => !me.flying,
            Act::Land => me.flying,
            Act::Cast { slot, aim } => slotted(sense, skills, slot).is_some_and(|(k, s)| {
                sense.ready.get(k).copied().unwrap_or(false) && (aim == Aim::Foe || aims(s.kind))
            }),
        };
    }
    out
}

/// The `Choice` act `act` is, for a fighter that senses `sense` and sees `foe`.
pub fn choice(act: Act, sense: &Sense, foe: &Foe, skills: &[Skill]) -> Choice {
    let me = &sense.me;
    let a = &sense.arena;
    let dir = ahead(me, foe);
    match act {
        Act::Wait => Choice::Wait,
        Act::Toward => Choice::Move { dir },
        Act::Away => Choice::Move { dir: -dir },
        Act::Jump => Choice::Jump,
        Act::Duck => Choice::Duck { hold: HOLD },
        Act::Guard => Choice::Guard { hold: HOLD },
        Act::FlyHigh => Choice::Fly {
            height: a.top * FLY,
        },
        Act::Land => Choice::Fly { height: 0.0 },
        Act::Meditate => Choice::Meditate,
        Act::Cast { slot, aim } => {
            let Some((_, s)) = slotted(sense, skills, slot) else {
                return Choice::Wait;
            };
            let target = match (s.kind, aim) {
                (SkillKind::Area, Aim::Foe) => Some(foe.x),
                (SkillKind::Area, Aim::Ahead) => Some(foe.x + dir * AIM),
                (SkillKind::Area, Aim::Behind) => Some(foe.x - dir * AIM),
                (SkillKind::Blink, Aim::Foe) => Some(foe.x - dir * BESIDE),
                (SkillKind::Blink, Aim::Ahead) => Some(foe.x + dir * BESIDE),
                (SkillKind::Blink, Aim::Behind) => Some(me.x - dir * s.reach),
                _ => None,
            };
            let target = target.map(|x| {
                if x.is_finite() {
                    x.clamp(a.left, a.right)
                } else {
                    me.x
                }
            });
            Choice::Cast {
                skill: s.name.clone(),
                target,
            }
        }
    }
}
