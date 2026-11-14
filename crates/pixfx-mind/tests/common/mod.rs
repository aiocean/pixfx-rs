//! A duel-like fighter for the tests: the skills of the site's Frost and Thunder, as data.
#![allow(dead_code)]

use pixfx_mind::*;

pub fn skill(name: &str, kind: SkillKind, cost: f64, windup: f64, recover: f64, damage: f64, reach: f64) -> Skill {
    Skill {
        name: name.into(),
        kind,
        cost,
        windup,
        recover,
        damage,
        reach,
        radius: 0.0,
        speed: 0.0,
        lands: 0.0,
        strikes: 1,
        ultimate: false,
        grounded: false,
        inflicts: Inflicts::default(),
    }
}

/// Blade, bolt, blink, a falling area, a barrier, a palm blast and an ultimate.
pub fn kit() -> Vec<Skill> {
    let mut bolt = skill("bolt", SkillKind::Bolt, 18.0, 0.28, 0.22, 13.0, 0.0);
    bolt.speed = 230.0;
    let mut icicle = skill("icicle", SkillKind::Area, 33.0, 0.42, 0.28, 28.0, 0.0);
    icicle.radius = 16.0;
    icicle.lands = 0.944;
    icicle.grounded = true;
    let mut prison = skill("prison", SkillKind::Area, 0.0, 0.7, 0.5, 31.0, 110.0);
    prison.radius = 26.0;
    prison.lands = 1.02;
    prison.ultimate = true;
    vec![
        skill("slash", SkillKind::Melee, 0.0, 0.2, 0.32, 5.0, 24.0),
        bolt,
        skill("blink", SkillKind::Blink, 20.0, 0.06, 0.12, 0.0, 56.0),
        icicle,
        skill("mirror", SkillKind::Barrier, 18.0, 0.06, 0.1, 0.0, 0.0),
        skill("lotus", SkillKind::Blast, 24.0, 0.3, 0.3, 18.0, 40.0),
        prison,
    ]
}

pub fn body() -> BodySize {
    BodySize { half_width: 6.0, chest: 14.0, body: 24.0, duck: 14.0, walk: 60.0 }
}

pub fn profile() -> Profile {
    Profile {
        aggression: 1.1,
        tempo: 2.0,
        spread: 0.8,
        range: 100.0,
        dry: 80.0,
        sky: 48.0,
        risk: 0.6,
        floor: 0.3,
        lead: 0.8,
        prior: 6.0,
        reaction: 0.16,
        think: 0.12,
        posture_scale: [1.0; 8],
    }
}

pub fn me(x: f64) -> Me {
    Me {
        x,
        air: 0.0,
        face: 1.0,
        hp: 100.0,
        qi: 100.0,
        rage: 0.0,
        rage_full: 4.0,
        flying: false,
        hover: 0.0,
        vx: 0.0,
        can_act: true,
        silenced: false,
        sealed: false,
        barrier: false,
        shield: false,
        seeker_out: false,
    }
}

pub fn foe(x: f64) -> Foe {
    Foe {
        x,
        air: 0.0,
        flying: false,
        hp: 100.0,
        qi: 100.0,
        doing: Doing::Idle,
        out_for: 0.0,
        open_in: 0.0,
        frozen: 0.0,
        shocked: 0.0,
        barrier: false,
        ready: vec![skill("slash", SkillKind::Melee, 0.0, 0.2, 0.32, 5.0, 24.0)],
    }
}

pub fn arena() -> Arena {
    Arena { left: 0.0, right: 220.0, top: 96.0 }
}

pub fn sense(time: f64, me: Me, foe: Foe) -> Sense {
    let n = kit().len();
    Sense { time, me, foe, arena: arena(), threats: vec![], ready: vec![true; n] }
}

pub fn blade_windup(t: f64) -> Doing {
    Doing::Act { kind: SkillKind::Melee, phase: Phase::Windup, t, windup: 0.2, recover: 0.32, reach: 24.0, damage: 5.0 }
}

pub fn reading(me: &Me, foe: &Foe, threats: &[Threat]) -> Reading {
    let kit = kit();
    let ready = vec![true; kit.len()];
    Reading::of(me, foe, threats, &arena(), &kit, &ready, &body(), &profile(), 0.0)
}
