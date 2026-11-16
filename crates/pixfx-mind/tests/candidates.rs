//! Candidate actions from the skills held (by kind, never name) and the threats seen; the rank-then-weight pick.

mod common;
use common::*;
use pixfx_mind::*;

struct Case {
    me: Me,
    foe: Foe,
    threats: Vec<Threat>,
    skills: Vec<Skill>,
    ready: Vec<bool>,
    read: Read,
    now: f64,
}

impl Case {
    fn new(me: Me, foe: Foe) -> Case {
        let skills = kit();
        let ready = vec![true; skills.len()];
        let read = Read::new(skills.len());
        Case {
            me,
            foe,
            threats: vec![],
            skills,
            ready,
            read,
            now: 10.0,
        }
    }

    fn candidates(&self) -> Vec<Candidate> {
        let reading = Reading::of(
            &self.me,
            &self.foe,
            &self.threats,
            &arena(),
            &self.skills,
            &self.ready,
            &body(),
            &profile(),
            0.0,
        );
        let ctx = Ctx {
            me: &self.me,
            foe: &self.foe,
            foe_skills: &foe_kit(),
            threats: &self.threats,
            arena: &arena(),
            skills: &self.skills,
            ready: &self.ready,
            body: &body(),
            profile: &profile(),
            reading: &reading,
            read: &self.read,
            calm: 0.0,
            now: self.now,
            predicted: None,
            answer: None,
            level: 0,
            roll: 0.3,
        };
        candidates(&ctx)
    }
}

fn casts(cands: &[Candidate]) -> Vec<String> {
    cands
        .iter()
        .filter_map(|c| match &c.choice {
            Choice::Cast { skill, .. } => Some(skill.clone()),
            _ => None,
        })
        .collect()
}

fn has(cands: &[Candidate], test: impl Fn(&Choice) -> bool) -> bool {
    cands.iter().any(|c| test(&c.choice))
}

#[test]
fn only_ready_skills_are_cast() {
    let mut case = Case::new(me(40.0), foe(60.0));
    case.ready = vec![false; case.skills.len()];
    let cands = case.candidates();
    assert!(casts(&cands).is_empty(), "{:?}", casts(&cands));
    let mut rng = Rng::new(1);
    let Some((i, _)) = pick(&cands, &POSTURES[0].weights, &mut rng) else {
        unreachable!("nothing to do")
    };
    assert!(!matches!(cands[i].choice, Choice::Cast { .. }));
}

#[test]
fn a_blade_in_reach_is_offered() {
    let cands = Case::new(me(40.0), foe(60.0)).candidates();
    assert!(casts(&cands).contains(&"slash".to_string()));
}

#[test]
fn a_bolt_at_chest_height_offers_guard_duck_jump() {
    let mut case = Case::new(me(40.0), foe(150.0));
    // 0.2 s out, flying at chest height toward me, from the side I face
    case.threats = vec![Threat {
        age: 0.3,
        damage: 13.0,
        shape: ThreatShape::Projectile {
            x: 86.0,
            y: 20.0,
            vx: -230.0,
        },
    }];
    let cands = case.candidates();
    assert!(has(&cands, |c| matches!(c, Choice::Guard { .. })));
    assert!(has(&cands, |c| matches!(c, Choice::Duck { .. })));
    assert!(has(&cands, |c| matches!(c, Choice::Jump)));
}

#[test]
fn standing_in_an_area_offers_to_leave() {
    let mut case = Case::new(me(40.0), foe(150.0));
    case.threats = vec![Threat {
        age: 0.5,
        damage: 28.0,
        shape: ThreatShape::Area {
            x: 44.0,
            radius: 16.0,
            reach: 0.0,
            lands_in: 0.4,
            grounded: true,
        },
    }];
    let cands = case.candidates();
    // away from the area's centre, which is to my right
    assert!(
        cands
            .iter()
            .any(|c| c.choice == Choice::Move { dir: -1.0 } && c.category == Category::Evade)
    );
    assert!(has(
        &cands,
        |c| matches!(c, Choice::Cast { skill, .. } if skill == "blink")
    ));
}

#[test]
fn a_foe_blade_winding_up_in_reach_offers_step_back_and_guard() {
    let mut f = foe(60.0);
    f.doing = blade_windup(0.02);
    let cands = Case::new(me(40.0), f).candidates();
    assert!(
        cands
            .iter()
            .any(|c| c.choice == Choice::Move { dir: -1.0 } && c.category == Category::Retreat)
    );
    assert!(has(&cands, |c| matches!(c, Choice::Guard { .. })));
}

#[test]
fn a_barrier_answers_a_big_threat() {
    let mut f = foe(60.0);
    f.doing = Doing::Act {
        kind: SkillKind::Blast,
        phase: ActPhase::Windup,
        t: 0.1,
        windup: 0.32,
        recover: 0.35,
        reach: 38.0,
        damage: 20.0,
        ultimate: false,
    };
    let cands = Case::new(me(40.0), f).candidates();
    assert!(casts(&cands).contains(&"mirror".to_string()));
}

#[test]
fn no_ranged_skills_still_fights() {
    let mut case = Case::new(me(40.0), foe(150.0));
    case.skills = vec![kit()[0].clone(), kit()[2].clone()];
    case.ready = vec![true, true];
    case.read = Read::new(2);
    let cands = case.candidates();
    assert!(cands.iter().any(|c| c.category == Category::Approach));
}

#[test]
fn names_are_never_read() {
    let base = Case::new(me(40.0), foe(60.0));
    let mut renamed = Case::new(me(40.0), foe(60.0));
    for (i, s) in renamed.skills.iter_mut().enumerate() {
        s.name = format!("s{i}");
    }
    let names: Vec<String> = base.skills.iter().map(|s| s.name.clone()).collect();
    let a = base.candidates();
    let b = renamed.candidates();
    assert_eq!(a.len(), b.len());
    for (x, y) in a.iter().zip(&b) {
        assert_eq!(x.base, y.base);
        assert_eq!(x.category, y.category);
        if let (Choice::Cast { skill: sx, .. }, Choice::Cast { skill: sy, .. }) =
            (&x.choice, &y.choice)
        {
            assert_eq!(
                sy,
                &format!("s{}", names.iter().position(|n| n == sx).unwrap_or(99))
            );
        }
    }
}

#[test]
fn a_punished_skill_scores_lower() {
    let mut case = Case::new(me(40.0), foe(60.0));
    let slash = |c: &[Candidate]| {
        c.iter()
            .find(|c| matches!(&c.choice, Choice::Cast { skill, .. } if skill == "slash"))
            .map(|c| c.base)
            .unwrap_or(0.0)
    };
    let before = slash(&case.candidates());
    case.read.punished(0, 9.5);
    let after = slash(&case.candidates());
    assert!(after < before * 0.8, "{before} -> {after}");
    case.now = 30.0;
    assert_eq!(slash(&case.candidates()), before);
}

#[test]
fn what_lands_is_learnt() {
    let mut case = Case::new(me(40.0), foe(60.0));
    let slash = |c: &[Candidate]| {
        c.iter()
            .find(|c| matches!(&c.choice, Choice::Cast { skill, .. } if skill == "slash"))
            .map(|c| c.base)
            .unwrap_or(0.0)
    };
    let before = slash(&case.candidates());
    for _ in 0..6 {
        case.read.tried(0);
        case.read.landed(0);
    }
    assert!(slash(&case.candidates()) > before);
}

#[test]
fn hundred_threats_no_nan() {
    let mut case = Case::new(me(40.0), foe(60.0));
    let mut rng = Rng::new(4);
    for i in 0..100 {
        let x = rng.next() * 220.0;
        let shape = match i % 5 {
            0 => ThreatShape::Projectile {
                x,
                y: rng.next() * 40.0,
                vx: (rng.next() - 0.5) * 400.0,
            },
            1 => ThreatShape::Area {
                x,
                radius: 16.0,
                reach: rng.next() * 60.0,
                lands_in: rng.next(),
                grounded: i % 2 == 0,
            },
            2 => ThreatShape::Seeker {
                x,
                y: rng.next() * 40.0,
                radius: 10.0,
            },
            3 => ThreatShape::Field {
                x,
                reach: rng.next() * 30.0,
            },
            _ => ThreatShape::Barrage {
                lands_in: rng.next(),
            },
        };
        case.threats.push(Threat {
            age: 1.0,
            damage: rng.next() * 30.0,
            shape,
        });
    }
    let cands = case.candidates();
    assert!(cands.iter().all(|c| c.base.is_finite()));
    assert!(pick(&cands, &POSTURES[4].weights, &mut rng).is_some());
}

#[test]
fn no_threats_no_nan() {
    let mut case = Case::new(me(0.0), foe(0.0));
    case.skills.iter_mut().for_each(|s| s.speed = 0.0);
    let cands = case.candidates();
    assert!(cands.iter().all(|c| c.base.is_finite()));
}

#[test]
fn pick_is_seeded() {
    let cands = Case::new(me(40.0), foe(60.0)).candidates();
    let run = |seed| {
        let mut rng = Rng::new(seed);
        (0..100)
            .map(|_| pick(&cands, &POSTURES[1].weights, &mut rng).map(|p| p.0))
            .collect::<Vec<_>>()
    };
    assert_eq!(run(3), run(3));
}

#[test]
fn pick_prefers_what_the_posture_weighs() {
    let cands = vec![
        Candidate {
            choice: Choice::Move { dir: 1.0 },
            category: Category::Approach,
            base: 1.0,
            skill: None,
        },
        Candidate {
            choice: Choice::Meditate,
            category: Category::Meditate,
            base: 1.0,
            skill: None,
        },
    ];
    let mut rng = Rng::new(2);
    let recover = &POSTURES[Posture::Recover as usize].weights;
    let meditates = (0..200)
        .filter(|_| pick(&cands, recover, &mut rng).map(|p| p.0) == Some(1))
        .count();
    assert!(meditates > 180, "{meditates}");
}
