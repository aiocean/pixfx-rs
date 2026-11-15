//! The mind over a round: when it thinks, what it remembers, what it does in the moments that matter.

mod common;
use common::*;
use pixfx_mind::*;

const DT: f64 = 1.0 / 60.0;

fn mind(seed: u32) -> Mind {
    let mut m = Mind::new(profile(), body(), kit(), seed);
    m.meet(foe_kit());
    m
}

/// Ticks the mind through `seconds` of the same moment (time moving), returning its thoughts.
fn run(m: &mut Mind, from: f64, seconds: f64, at: impl Fn(f64) -> Sense) -> Vec<Thought> {
    let mut out = vec![];
    let mut t = from;
    while t < from + seconds {
        if let Some(th) = m.think(&at(t), DT) {
            out.push(th);
        }
        t += DT;
    }
    out
}

fn calm_moment(t: f64) -> Sense {
    sense(t, me(40.0), foe(150.0))
}

#[test]
fn same_seed_same_thoughts() {
    let (mut a, mut b) = (mind(4), mind(4));
    let ta = run(&mut a, 0.0, 10.0, calm_moment);
    let tb = run(&mut b, 0.0, 10.0, calm_moment);
    assert!(ta.len() > 20);
    assert_eq!(ta, tb);
}

#[test]
fn it_thinks_about_eight_times_a_second() {
    let thoughts = run(&mut mind(1), 0.0, 10.0, calm_moment);
    assert!((60..=100).contains(&thoughts.len()), "{}", thoughts.len());
}

#[test]
fn no_thought_while_it_cannot_act() {
    let thoughts = run(&mut mind(1), 0.0, 5.0, |t| {
        let mut s = calm_moment(t);
        s.me.can_act = false;
        s
    });
    assert!(thoughts.is_empty());
}

#[test]
fn hit_stop_holds_the_clock() {
    let mut m = mind(1);
    let first = run(&mut m, 0.0, 2.0, calm_moment).len();
    assert!(first > 0);
    // a long hit-stop: time stands still, dt is 0
    for _ in 0..1000 {
        assert!(m.think(&calm_moment(2.0), 0.0).is_none());
    }
    // and the mind picks up where it was: a thought within its usual gap
    let after = run(&mut m, 2.0, 0.2, calm_moment);
    assert!(!after.is_empty());
}

#[test]
fn hold_keeps_it_quiet() {
    let mut m = mind(1);
    m.hold(3.0);
    assert!(run(&mut m, 0.0, 2.9, calm_moment).is_empty());
    assert!(!run(&mut m, 2.9, 1.0, calm_moment).is_empty());
}

#[test]
fn the_first_thought_of_a_round_waits() {
    let mut m = mind(2);
    let mut t = 0.0;
    while m.think(&calm_moment(t), DT).is_none() {
        t += DT;
    }
    assert!((0.38..=0.72).contains(&t), "{t}");
}

#[test]
fn reset_forgets_the_round() {
    let mut m = mind(3);
    // a foe that keeps coming in with its blade
    run(&mut m, 0.0, 6.0, |t| {
        let mut f = foe(70.0 - (t % 1.0) * 30.0);
        f.doing = if t % 1.0 < 0.5 {
            Doing::Move { dir: -1.0 }
        } else {
            blade_windup(t % 0.5)
        };
        sense(t, me(40.0), f)
    });
    m.reset();
    assert_eq!(m.posture(), Posture::Neutral);
    assert!(
        run(&mut m, 6.0, 0.35, calm_moment).is_empty(),
        "the first thought of the new round waits again"
    );
}

#[test]
fn scenario_punishes_a_whiff() {
    let mut m = mind(5);
    let thoughts = run(&mut m, 0.0, 1.5, |t| {
        let mut f = foe(62.0);
        f.doing = Doing::Act {
            kind: SkillKind::Melee,
            phase: ActPhase::Recover,
            t,
            windup: 0.2,
            recover: 3.0,
            reach: 24.0,
            damage: 5.0,
        };
        sense(t, me(40.0), f)
    });
    let first = &thoughts[..2.min(thoughts.len())];
    assert!(
        first.iter().any(|th| th.posture == Posture::Punish),
        "{first:?}"
    );
    assert!(
        first.iter().any(|th| matches!(&th.choice, Choice::Cast { skill, .. } if ["slash", "lotus", "prison"].contains(&skill.as_str()))),
        "{first:?}"
    );
}

#[test]
fn scenario_meditates_when_dry() {
    let mut m = mind(6);
    let kit = kit();
    let thoughts = run(&mut m, 0.0, 2.0, |t| {
        let mut s = sense(t, me(30.0), foe(180.0));
        s.me.qi = 10.0;
        s.ready = kit.iter().map(|k| k.cost <= 10.0).collect();
        s
    });
    let last = thoughts.last().cloned();
    assert_eq!(
        last.as_ref().map(|t| t.posture),
        Some(Posture::Recover),
        "{thoughts:?}"
    );
    assert!(
        thoughts
            .iter()
            .rev()
            .take(4)
            .any(|t| t.choice == Choice::Meditate),
        "{thoughts:?}"
    );
}

#[test]
fn scenario_leaves_a_falling_area() {
    let mut m = mind(7);
    let mut first = None;
    let mut t = 0.0;
    while first.is_none() && t < 2.0 {
        let mut s = sense(t, me(40.0), foe(150.0));
        s.threats = vec![Threat {
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
        first = m.think(&s, DT);
        t += DT;
    }
    let Some(th) = first else {
        unreachable!("no thought")
    };
    assert!(
        matches!(th.posture, Posture::Defend | Posture::Evade),
        "{th:?}"
    );
    let leaves = match &th.choice {
        Choice::Move { dir } => *dir < 0.0,
        Choice::Fly { .. } => true,
        Choice::Cast { skill, .. } => skill == "blink" || skill == "mirror",
        _ => false,
    };
    assert!(leaves, "{th:?}");
}

#[test]
fn an_attack_is_followed_by_a_breath() {
    // the foe free and in reach the whole time: no opening, so after an attack the mind holds back for its breath
    // (Frost's tempo 2 s, at least 1.2 s with its spread)
    let mut m = mind(8);
    let mut attacks = vec![];
    let mut t = 0.0;
    while t < 8.0 {
        if let Some(th) = m.think(&sense(t, me(40.0), foe(60.0)), DT)
            && let Choice::Cast { skill, .. } = &th.choice
            && ["slash", "bolt", "icicle", "lotus", "prison"].contains(&skill.as_str())
        {
            attacks.push((t, skill.clone()));
        }
        t += DT;
    }
    assert!(!attacks.is_empty());
    for pair in attacks.windows(2) {
        assert!(pair[1].0 - pair[0].0 >= 1.2, "{attacks:?}");
    }
}

#[test]
fn tells_are_learnt() {
    let mut m = mind(9);
    m.tell(&Outcome::Landed {
        skill: "slash".into(),
    });
    m.tell(&Outcome::Punished {
        skill: "bolt".into(),
    });
    m.tell(&Outcome::Landed {
        skill: "no such skill".into(),
    });
}

#[test]
fn explain_fills_why() {
    let mut m = mind(10);
    assert!(
        run(&mut m, 0.0, 1.0, calm_moment)
            .iter()
            .all(|t| t.why.is_none())
    );
    m.explain(true);
    let thoughts = run(&mut m, 1.0, 1.0, calm_moment);
    let Some(why) = thoughts.last().and_then(|t| t.why.clone()) else {
        unreachable!("no why")
    };
    assert_eq!(why.postures.len(), 8);
    assert!(!why.top.is_empty());
}
