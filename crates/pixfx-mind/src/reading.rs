//! The reading of a moment: the raw facts the candidates reason with (distance, openings, the foe's answer) and
//! every consideration's input in `[0, 1]`. Openings and the punisher follow the duel's first mind (Sirlin's
//! Playing to Win: an opening is time the foe cannot answer; a miss costs the hardest blow it can land back).

use crate::choice::Posture;
use crate::curve::{Input, Inputs};
use crate::sense::{Arena, Doing, Foe, Me, Phase, Shape, Threat};
use crate::skill::{BodySize, Profile, Skill, SkillKind};

/// Urgency of 20 reads as a full threat.
const THREAT_FULL: f64 = 20.0;
/// A wall this near reads as a full corner.
const CORNER: f64 = 40.0;

#[derive(Clone, Debug, PartialEq)]
pub struct Reading {
    /// Distance to the foe, and its sign (foe minus me).
    pub d: f64,
    pub dx: f64,
    /// Close enough in height for a blade.
    pub level: bool,
    /// How long the foe cannot answer: gathering, recovering or staggered.
    pub busy: f64,
    /// How long the foe stays open to punish (a blow it winds up is no opening).
    pub opening: f64,
    /// Until a blow can touch the foe at all.
    pub open_in: f64,
    /// The foe is free to answer anything.
    pub free: bool,
    /// Ahead on life: -1 far behind, 1 far ahead.
    pub lead: f64,
    /// The worst urgency of what is coming at me.
    pub urgency: f64,
    /// How the foe seems to be playing.
    pub foe_posture: Posture,
    /// The range I want to fight at now.
    pub want: f64,
    pub inputs: [f64; 18],
}

fn unit(x: f64) -> f64 {
    if x.is_nan() { 0.0 } else { x.clamp(0.0, 1.0) }
}

const fn slot(input: Input) -> usize {
    match input {
        Input::Qi => 0,
        Input::Hp => 1,
        Input::FoeHpLow => 2,
        Input::Lead => 3,
        Input::Opening => 4,
        Input::Threat => 5,
        Input::InRange => 6,
        Input::Close => 7,
        Input::CloseReady => 8,
        Input::RangedReady => 9,
        Input::AreaReady => 10,
        Input::Cornered => 11,
        Input::FoeCornered => 12,
        Input::FoePressing => 13,
        Input::FoeZoning => 14,
        Input::FoeRecovering => 15,
        Input::RageFull => 16,
        Input::Calm => 17,
    }
}

impl Inputs for Reading {
    fn input(&self, input: Input) -> f64 {
        self.inputs[slot(input)]
    }
}

pub fn is_close(kind: SkillKind) -> bool {
    matches!(kind, SkillKind::Melee | SkillKind::Dive | SkillKind::Dash | SkillKind::Blast)
}

pub fn is_ranged(kind: SkillKind) -> bool {
    matches!(kind, SkillKind::Bolt | SkillKind::Seeker)
}

pub fn is_area(kind: SkillKind) -> bool {
    matches!(kind, SkillKind::Area | SkillKind::Field)
}

/// A skill the foe winds up that strikes where it stands: no opening, only a trade.
fn striking(kind: SkillKind) -> bool {
    matches!(kind, SkillKind::Melee | SkillKind::Dive | SkillKind::Blast | SkillKind::Dash)
}

/// The hardest blow `foe` could land on `me` within `time` seconds (my recovery after a miss), from what it has
/// ready and where it stands.
pub fn punisher(foe: &Foe, me: &Me, body: &BodySize, time: f64, reaction: f64) -> f64 {
    let d = (foe.x - me.x).abs();
    let level = (foe.air - me.air).abs();
    let mut most: f64 = 0.0;
    for s in &foe.ready {
        if s.damage <= 0.0 || s.ultimate {
            continue;
        }
        let lands = match s.kind {
            SkillKind::Melee if d < s.reach + body.half_width && level < body.chest => s.windup,
            SkillKind::Blast if d < s.reach + body.half_width && level < body.body => s.windup,
            SkillKind::Dash if d < s.reach && level < body.chest => s.windup,
            SkillKind::Bolt if level < body.body && s.speed > 0.0 => s.windup + d / s.speed,
            _ => f64::INFINITY,
        };
        if lands <= time + reaction {
            most = most.max(s.damage);
        }
    }
    most
}

/// How urgent each thing coming at me is (the duel's first mind's `dangers` and `blows`), the worst of them.
fn urgency(me: &Me, foe: &Foe, threats: &[Threat], body: &BodySize) -> f64 {
    let mut worst: f64 = 0.0;
    for th in threats {
        let u = match th.shape {
            Shape::Barrage { .. } => th.damage * 1.4,
            Shape::Projectile { x, y, vx } => {
                let toward = vx != 0.0 && (me.x - x).signum() == vx.signum();
                let tti = if vx == 0.0 { f64::INFINITY } else { (me.x - x).abs() / vx.abs() };
                let hits = y >= me.air - 3.0 && y <= me.air + body.body + 3.0;
                if toward && hits && tti <= 0.6 { th.damage * (1.2 - tti) } else { 0.0 }
            }
            Shape::Seeker { x, y, .. } => {
                let d = libm::hypot(x - me.x, y - (me.air + body.chest));
                if d <= 48.0 { th.damage * (1.4 - d / 48.0) } else { 0.0 }
            }
            Shape::Area { x, radius, reach, lands_in, .. } => {
                if (me.x - x).abs() <= reach + radius + body.half_width + 4.0 { th.damage * (1.5 - lands_in) } else { 0.0 }
            }
            Shape::Field { x, reach } => {
                let dx = (me.x - x).abs();
                if reach > 0.0 && dx <= reach { th.damage * (1.2 - dx / reach) } else { 0.0 }
            }
        };
        worst = worst.max(u);
    }
    if let Doing::Act { kind: SkillKind::Melee | SkillKind::Blast | SkillKind::Dive, phase: Phase::Windup, reach, damage, .. } = foe.doing
        && (foe.x - me.x).abs() <= reach + body.half_width + 6.0
    {
        worst = worst.max(damage * 1.2);
    }
    worst
}

/// How the foe seems to be playing, from what it does now.
fn foe_posture(me: &Me, foe: &Foe) -> Posture {
    let d = (foe.x - me.x).abs();
    if foe.out_for > 0.0 || foe.frozen > 0.0 || foe.shocked > 0.0 {
        return Posture::Defend;
    }
    match &foe.doing {
        Doing::Act { kind, reach, .. } if is_close(*kind) && d < reach + 30.0 => Posture::Pressure,
        Doing::Act { kind, .. } if is_ranged(*kind) || is_area(*kind) => Posture::Zone,
        Doing::Act { kind: SkillKind::Barrier | SkillKind::Ward | SkillKind::Heal, .. } => Posture::Defend,
        Doing::Act { kind: SkillKind::Blink, .. } => Posture::Evade,
        Doing::Meditate => Posture::Recover,
        Doing::Guard => Posture::Defend,
        Doing::Move { dir } if *dir != 0.0 && dir.signum() == (me.x - foe.x).signum() && d < 90.0 => Posture::Pressure,
        Doing::Fly if d > 60.0 => Posture::Zone,
        _ if foe.qi < 25.0 => Posture::Recover,
        _ => Posture::Neutral,
    }
}

impl Reading {
    #[allow(clippy::too_many_arguments)]
    pub fn of(
        me: &Me,
        foe: &Foe,
        threats: &[Threat],
        arena: &Arena,
        skills: &[Skill],
        ready: &[bool],
        body: &BodySize,
        profile: &Profile,
        calm: f64,
    ) -> Reading {
        let dx = foe.x - me.x;
        let d = dx.abs();
        let level = (foe.air - me.air).abs() < body.chest;
        let act = match foe.doing {
            Doing::Act { kind, phase, t, windup, recover, .. } => Some((kind, phase, t, windup, recover)),
            _ => None,
        };
        let busy = foe.out_for
            + act.map_or(0.0, |(_, phase, t, windup, recover)| match phase {
                Phase::Windup => (windup - t).max(0.0) + recover,
                Phase::Recover => (recover - t).max(0.0),
            });
        let opening = foe.out_for
            + foe.frozen
            + foe.shocked
            + act.map_or(0.0, |(kind, phase, t, windup, recover)| match phase {
                Phase::Recover => (recover - t).max(0.0),
                Phase::Windup if striking(kind) => 0.0,
                Phase::Windup => (windup - t).max(0.0) + recover,
            });
        let free = foe.out_for == 0.0 && act.is_none() && foe.frozen == 0.0 && foe.shocked == 0.0;
        let lead = ((me.hp - foe.hp) / 40.0).clamp(-1.0, 1.0);
        let urgency = urgency(me, foe, threats, body);

        let held = |test: fn(SkillKind) -> bool| skills.iter().any(|s| test(s.kind));
        let has_ready = |test: &dyn Fn(&Skill) -> bool| skills.iter().zip(ready).any(|(s, r)| *r && test(s));
        // the distance it keeps comes from what it can use: its range with spells that fly or fall, arm's length
        // with a blast or dash, a blade's length with nothing else; with too little qi, its `dry` distance
        let blade = skills.iter().filter(|s| s.kind == SkillKind::Melee).map(|s| s.reach).fold(24.0, f64::min);
        let want = if me.qi < 25.0 {
            profile.dry
        } else if held(is_ranged) || held(is_area) {
            profile.range
        } else if skills.iter().any(|s| matches!(s.kind, SkillKind::Blast | SkillKind::Dash)) {
            34.0
        } else {
            blade - 4.0
        };
        let quickest = skills
            .iter()
            .zip(ready)
            .filter(|(s, r)| **r && s.damage > 0.0)
            .map(|(s, _)| s.windup)
            .fold(f64::INFINITY, f64::min);
        let opening_n = if quickest.is_finite() { (opening - quickest * 0.5) / quickest.max(0.05) } else { 0.0 };
        let wall = |x: f64| (x - arena.left).min(arena.right - x);
        let posture = foe_posture(me, foe);
        let rage_full = me.rage >= me.rage_full && has_ready(&|s: &Skill| s.ultimate);
        let foe_recovering = if matches!(foe.doing, Doing::Meditate) { 1.0 } else { (30.0 - foe.qi) / 30.0 };

        let mut inputs = [0.0; 18];
        let mut set = |input: Input, v: f64| inputs[slot(input)] = unit(v);
        set(Input::Qi, me.qi / 100.0);
        set(Input::Hp, me.hp / 100.0);
        set(Input::FoeHpLow, 1.0 - foe.hp / 100.0);
        set(Input::Lead, (lead + 1.0) / 2.0);
        set(Input::Opening, opening_n);
        set(Input::Threat, urgency / THREAT_FULL);
        set(Input::InRange, 1.0 - (d - want).abs() / 60.0);
        set(Input::Close, 1.0 - (d - 20.0) / 60.0);
        set(Input::CloseReady, f64::from(u8::from(has_ready(&|s: &Skill| is_close(s.kind) && !s.ultimate))));
        set(Input::RangedReady, f64::from(u8::from(has_ready(&|s: &Skill| is_ranged(s.kind)))));
        set(Input::AreaReady, f64::from(u8::from(has_ready(&|s: &Skill| is_area(s.kind) && !s.ultimate))));
        set(Input::Cornered, 1.0 - wall(me.x) / CORNER);
        set(Input::FoeCornered, 1.0 - wall(foe.x) / CORNER);
        set(Input::FoePressing, f64::from(u8::from(posture == Posture::Pressure)));
        set(Input::FoeZoning, f64::from(u8::from(posture == Posture::Zone)));
        set(Input::FoeRecovering, foe_recovering);
        set(Input::RageFull, f64::from(u8::from(rage_full)));
        set(Input::Calm, if profile.tempo > 0.0 { calm / profile.tempo } else { 0.0 });

        Reading { d, dx, level, busy, opening, open_in: foe.open_in, free, lead, urgency, foe_posture: posture, want, inputs }
    }

    /// Set an input, clamped to `[0, 1]`: the mind blends what it remembers of the foe into the moment's read.
    pub fn set(&mut self, input: Input, value: f64) {
        self.inputs[slot(input)] = unit(value);
    }
}
