//! What a brain is told: each spell as pixfx-mind reads it (`skill_of`), and what a fighter could perceive this tick
//! (`sense_of`), as `duel.js` builds them.

use crate::bar::{ready, slot_of};
use crate::data::{Kind, RAGE, spell};
use crate::fighter::{Fighter, Phase, Slot, State};
use crate::shot::{ShotKind, orb_at};
use pixfx_mind::{ActPhase, Arena, Doing, Foe, Inflicts, Me, Sense, Skill, SkillKind, Threat, ThreatShape};

/// Spell `name` as pixfx-mind reads it: what it does, never what it is called (a heal's `damage` is what it heals).
pub fn skill_of(name: &str) -> Skill {
    let Some(s) = spell(name) else {
        return Skill {
            name: name.to_string(),
            kind: SkillKind::Melee,
            cost: 0.0,
            windup: 0.0,
            recover: 0.0,
            damage: 0.0,
            reach: 0.0,
            radius: 0.0,
            speed: 0.0,
            lands: 0.0,
            strikes: 1,
            ultimate: false,
            grounded: false,
            inflicts: Inflicts { freeze: 0.0, shock: false, silence: 0.0, def_down: 0.0, burn: false, chill: false, pull: 0.0 },
        };
    };
    let kind = if s.dive {
        SkillKind::Dive
    } else {
        match s.kind {
            Kind::Melee => SkillKind::Melee,
            Kind::Dash => SkillKind::Dash,
            Kind::Blast => SkillKind::Blast,
            Kind::Bolt => SkillKind::Bolt,
            Kind::Orb => SkillKind::Seeker,
            Kind::Zone => SkillKind::Area,
            Kind::Void => SkillKind::Field,
            Kind::Heal => SkillKind::Heal,
            Kind::Ward => SkillKind::Ward,
            Kind::Barrier => SkillKind::Barrier,
            Kind::Blink => SkillKind::Blink,
        }
    };
    let reach = match s.kind {
        Kind::Dash | Kind::Blink => s.distance.unwrap_or(f64::NAN),
        Kind::Melee | Kind::Blast => s.range.unwrap_or(f64::NAN),
        _ => s.reach.unwrap_or(0.0),
    };
    let damage = if s.kind == Kind::Heal { s.amount } else { s.damage };
    let lands = if s.kind == Kind::Void { s.life.unwrap_or(f64::NAN) } else { s.sigil.unwrap_or(0.0) + s.impact.unwrap_or(0.0) };
    Skill {
        name: name.to_string(),
        kind,
        cost: s.cost,
        windup: s.windup,
        recover: s.recover,
        damage: damage.unwrap_or(0.0),
        reach,
        radius: s.radius.unwrap_or(0.0),
        speed: s.speed.unwrap_or(0.0),
        lands,
        strikes: s.strikes.map_or(1, |n| n as u32),
        ultimate: s.ultimate,
        grounded: s.grounded,
        inflicts: Inflicts {
            freeze: s.freeze.unwrap_or(0.0),
            shock: s.shock,
            silence: s.silence.unwrap_or(0.0),
            def_down: s.def_down.unwrap_or(0.0),
            burn: s.burn,
            chill: s.chill,
            pull: s.pull.unwrap_or(0.0),
        },
    }
}

/// What fighter `f` is doing, as pixfx-mind reads it.
pub fn doing_of(f: &Fighter) -> Doing {
    if let Some(a) = f.act {
        let skill = skill_of(a.spell);
        let (windup, recover) = spell(a.spell).map_or((0.0, 0.0), |s| (s.windup, s.recover));
        let phase = if a.phase == Phase::Windup { ActPhase::Windup } else { ActPhase::Recover };
        return Doing::Act { kind: skill.kind, phase, t: a.t, windup, recover, reach: skill.reach, damage: skill.damage };
    }
    if f.guard > 0.0 {
        Doing::Guard
    } else if f.medit {
        Doing::Meditate
    } else if f.fly {
        Doing::Fly
    } else if f.intent != 0.0 {
        Doing::Move { dir: f.intent }
    } else {
        Doing::Idle
    }
}

/// The foe's shots that could reach fighter `me`, each with how long it has been there to see.
pub fn threats_of(state: &State, me: usize) -> Vec<Threat> {
    let mut out = Vec::new();
    for s in &state.shots {
        if s.owner == me {
            continue;
        }
        let sp = s.spell;
        let damage = sp.damage.unwrap_or(0.0);
        let threat = match s.kind {
            ShotKind::Bolt { x, y, vx } => Some(Threat { age: s.t, damage, shape: ThreatShape::Projectile { x, y, vx } }),
            ShotKind::Orb { .. } => {
                let (x, y) = orb_at(s);
                Some(Threat { age: s.t, damage, shape: ThreatShape::Seeker { x, y: -y, radius: sp.radius.unwrap_or(f64::NAN) } })
            }
            ShotKind::Zone { fell: false, follow, x, lead, .. } => {
                if follow && s.t < 0.0 {
                    // a barrage still falling: its next strike aims where I stand
                    let age = s.t + sp.every.unwrap_or(f64::NAN) + sp.aim_ahead.unwrap_or(f64::NAN);
                    Some(Threat { age, damage, shape: ThreatShape::Barrage { lands_in: -s.t } })
                } else {
                    // a sealed spell is seen from the moment its seal shows, a falling one once it trembles
                    let shape = ThreatShape::Area {
                        x,
                        radius: sp.radius.unwrap_or(f64::NAN),
                        reach: sp.reach.unwrap_or(0.0),
                        lands_in: sp.impact.unwrap_or(f64::NAN) - s.t,
                        grounded: sp.grounded,
                    };
                    Some(Threat { age: s.t + lead - sp.notice.unwrap_or(f64::NAN), damage, shape })
                }
            }
            ShotKind::Void { x, .. } => {
                Some(Threat { age: s.t, damage, shape: ThreatShape::Field { x, reach: sp.reach.unwrap_or(f64::NAN) } })
            }
            ShotKind::Zone { .. } | ShotKind::Spray => None,
        };
        out.extend(threat);
    }
    out
}

/// What fighter `me` could perceive this tick, as pixfx-mind reads it: the true state; it sees late on its own.
pub fn sense_of(state: &State, me: usize) -> Sense {
    let f = &state.fighters[me];
    let foe = &state.fighters[1 - me];
    let b = &f.body;
    let free = !(f.act.is_some() || b.reeling() || b.squat > 0.0 || b.duck > 0.0 || b.frozen > 0.0 || b.shocked > 0.0 || f.hp <= 0.0);
    Sense {
        time: state.time,
        me: Me {
            x: b.x,
            air: b.air,
            face: b.face,
            hp: f.hp,
            qi: f.qi,
            rage: f.rage,
            rage_full: RAGE,
            flying: f.fly,
            hover: f.hover,
            vx: b.vx,
            can_act: free,
            silenced: f.silence > 0.0,
            sealed: f.sealed > 0.0,
            barrier: f.barrier > 0.0,
            shield: f.charges > 0.0,
            seeker_out: state.shots.iter().any(|s| matches!(s.kind, ShotKind::Orb { .. }) && s.owner == me),
        },
        foe: Foe {
            x: foe.body.x,
            air: foe.body.air,
            flying: foe.fly,
            hp: foe.hp,
            qi: foe.qi,
            doing: doing_of(foe),
            out_for: foe.body.out_for(),
            open_in: foe.body.open_in(),
            frozen: foe.body.frozen,
            shocked: foe.body.shocked,
            barrier: foe.barrier > 0.0,
            ready: foe.spells.iter().map(|n| ready(foe, n)).collect(),
        },
        arena: Arena { left: state.left, right: state.right, top: state.top },
        threats: threats_of(state, me),
        ready: f.spells.iter().map(|n| ready(f, n)).collect(),
        // a bar skill strikes at its power on the bar
        power: f
            .spells
            .iter()
            .map(|n| match slot_of(f, n) {
                Some(Slot::Held { power, .. }) => *power,
                _ => 1.0,
            })
            .collect(),
    }
}
