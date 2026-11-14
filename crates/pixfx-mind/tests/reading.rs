//! The reading of a moment: what is open, what threatens, how the foe is playing, every input in [0, 1].

mod common;
use common::*;
use pixfx_mind::*;

#[test]
fn opening_counts_recovery() {
    let mut f = foe(60.0);
    f.doing = Doing::Act { kind: SkillKind::Melee, phase: Phase::Recover, t: 0.1, windup: 0.2, recover: 0.32, reach: 24.0, damage: 5.0 };
    let r = reading(&me(40.0), &f, &[]);
    assert!((r.opening - 0.22).abs() < 1e-9, "{}", r.opening);
    assert!(!r.free);
}

#[test]
fn a_striking_windup_is_no_opening() {
    let mut f = foe(60.0);
    f.doing = blade_windup(0.05);
    assert_eq!(reading(&me(40.0), &f, &[]).opening, 0.0);
}

#[test]
fn a_long_cast_is_an_opening() {
    let mut f = foe(150.0);
    f.doing = Doing::Act { kind: SkillKind::Area, phase: Phase::Windup, t: 0.1, windup: 0.5, recover: 0.3, reach: 0.0, damage: 25.0 };
    let r = reading(&me(40.0), &f, &[]);
    assert!((r.opening - 0.7).abs() < 1e-9);
    assert_eq!(r.foe_posture, Posture::Zone);
}

#[test]
fn punisher_reads_foe_blade_in_reach() {
    assert_eq!(punisher(&foe(60.0), &me(40.0), &body(), 0.32, 0.16), 5.0);
    assert_eq!(punisher(&foe(160.0), &me(40.0), &body(), 0.32, 0.16), 0.0);
}

#[test]
fn a_blade_winding_up_in_reach_threatens() {
    let mut f = foe(60.0);
    f.doing = blade_windup(0.05);
    let r = reading(&me(40.0), &f, &[]);
    assert!(r.input(Input::Threat) > 0.2);
    assert_eq!(r.foe_posture, Posture::Pressure);
}

#[test]
fn an_area_on_me_threatens_more_as_it_falls() {
    let area = |lands_in| Threat { age: 0.5, damage: 28.0, shape: Shape::Area { x: 42.0, radius: 16.0, reach: 0.0, lands_in, grounded: true } };
    let far = reading(&me(40.0), &foe(150.0), &[area(0.9)]).input(Input::Threat);
    let near = reading(&me(40.0), &foe(150.0), &[area(0.2)]).input(Input::Threat);
    assert!(near > far && far > 0.0);
}

#[test]
fn a_meditating_foe_reads_as_recovering() {
    let mut f = foe(150.0);
    f.doing = Doing::Meditate;
    let r = reading(&me(40.0), &f, &[]);
    assert_eq!(r.foe_posture, Posture::Recover);
    assert_eq!(r.input(Input::FoeRecovering), 1.0);
}

#[test]
fn cornered_reads_my_back_to_the_wall() {
    let mut m = me(4.0);
    m.face = 1.0;
    assert!(reading(&m, &foe(30.0), &[]).input(Input::Cornered) > 0.8);
    assert!(reading(&me(110.0), &foe(140.0), &[]).input(Input::Cornered) < 0.1);
}

const ALL: [Input; 18] = [
    Input::Qi, Input::Hp, Input::FoeHpLow, Input::Lead, Input::Opening, Input::Threat, Input::InRange, Input::Close,
    Input::CloseReady, Input::RangedReady, Input::AreaReady, Input::Cornered, Input::FoeCornered, Input::FoePressing,
    Input::FoeZoning, Input::FoeRecovering, Input::RageFull, Input::Calm,
];

#[test]
fn inputs_stay_in_unit_range() {
    let mut rng = Rng::new(3);
    let kinds = [SkillKind::Melee, SkillKind::Bolt, SkillKind::Area, SkillKind::Field, SkillKind::Blast, SkillKind::Heal];
    for i in 0..500 {
        let mut m = me(rng.next() * 30.0);
        m.qi = rng.next() * 100.0;
        m.hp = rng.next() * 100.0;
        m.air = rng.next() * 60.0;
        let mut f = foe(rng.next() * 30.0);
        f.hp = rng.next() * 100.0;
        f.out_for = rng.next();
        f.doing = Doing::Act {
            kind: kinds[i % kinds.len()],
            phase: if i % 2 == 0 { Phase::Windup } else { Phase::Recover },
            t: rng.next(),
            windup: rng.next(),
            recover: rng.next(),
            reach: rng.next() * 50.0,
            damage: rng.next() * 30.0,
        };
        let threats: Vec<Threat> = (0..i % 7)
            .map(|k| Threat { age: 1.0, damage: 20.0, shape: Shape::Field { x: k as f64 * 5.0, reach: 18.0 } })
            .collect();
        // an arena narrower than the range it likes
        let narrow = Arena { left: 0.0, right: 30.0, top: 96.0 };
        let kit = kit();
        let r = Reading::of(&m, &f, &threats, &narrow, &kit, &vec![i % 3 != 0; kit.len()], &body(), &profile(), rng.next() * 3.0);
        for input in ALL {
            let v = r.input(input);
            assert!((0.0..=1.0).contains(&v), "{input:?} = {v}");
        }
    }
}
