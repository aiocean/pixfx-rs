//! Postures chosen by score: considerations as data, commitment to the posture held, interrupts that break it.

mod common;
use common::*;
use pixfx_mind::*;

fn fresh(now: f64) -> Held {
    Held {
        posture: Posture::Neutral,
        since: now - 10.0,
    }
}

fn pick(r: &Reading) -> Posture {
    choose_posture(&fresh(10.0), 10.0, r, &profile()).0
}

#[test]
fn low_qi_far_foe_recovers() {
    let r = with(
        quiet(),
        &[
            (Input::Qi, 0.1),
            (Input::Close, 0.0),
            (Input::Threat, 0.0),
            (Input::CloseReady, 0.0),
            (Input::RangedReady, 0.0),
            (Input::AreaReady, 0.0),
        ],
    );
    assert_eq!(pick(&r), Posture::Recover);
}

#[test]
fn foe_recovering_in_reach_punishes() {
    let r = with(
        quiet(),
        &[
            (Input::Opening, 0.9),
            (Input::CloseReady, 1.0),
            (Input::Close, 1.0),
        ],
    );
    assert_eq!(pick(&r), Posture::Punish);
}

#[test]
fn an_incoming_blow_interrupts_any_posture() {
    let r = with(
        quiet(),
        &[
            (Input::Threat, 0.9),
            (Input::CloseReady, 1.0),
            (Input::Close, 1.0),
            (Input::Qi, 1.0),
        ],
    );
    let held = Held {
        posture: Posture::Pressure,
        since: 9.99,
    };
    assert_eq!(
        choose_posture(&held, 10.0, &r, &profile()).0,
        Posture::Defend
    );
}

#[test]
fn ranged_ready_far_from_a_presser_zones() {
    let r = with(
        quiet(),
        &[
            (Input::RangedReady, 1.0),
            (Input::CloseReady, 0.0),
            (Input::Close, 0.1),
            (Input::Qi, 0.9),
            (Input::FoePressing, 1.0),
        ],
    );
    assert_eq!(pick(&r), Posture::Zone);
}

#[test]
fn close_with_a_blade_ready_presses() {
    let r = with(
        quiet(),
        &[
            (Input::Close, 1.0),
            (Input::CloseReady, 1.0),
            (Input::RangedReady, 0.0),
            (Input::Qi, 0.9),
            (Input::Calm, 0.0),
            (Input::Lead, 0.8),
        ],
    );
    assert_eq!(pick(&r), Posture::Pressure);
}

/// Scales every posture so Zone scores 0.5 and Pressure 0.5 + `gap`, the rest nothing.
fn zone_against_pressure(gap: f64) -> (Reading, Profile) {
    let r = with(
        quiet(),
        &[
            (Input::Close, 0.6),
            (Input::CloseReady, 1.0),
            (Input::RangedReady, 1.0),
            (Input::Qi, 0.9),
            (Input::Calm, 0.0),
            (Input::Threat, 0.0),
            (Input::Opening, 0.0),
        ],
    );
    let (_, raw) = choose_posture(&fresh(10.0), 10.0, &r, &profile());
    let (z, p) = (Posture::Zone as usize, Posture::Pressure as usize);
    assert!(raw[z] > 0.0 && raw[p] > 0.0, "{raw:?}");
    let mut prof = profile();
    prof.posture_scale = [0.0; 8];
    prof.posture_scale[z] = 0.5 / raw[z];
    prof.posture_scale[p] = (0.5 + gap) / raw[p];
    (r, prof)
}

#[test]
fn commitment_holds_against_a_better_posture() {
    let (r, prof) = zone_against_pressure(0.25);
    let held = Held {
        posture: Posture::Zone,
        since: 9.95,
    };
    assert_eq!(choose_posture(&held, 10.0, &r, &prof).0, Posture::Zone);
}

#[test]
fn commitment_fades() {
    let (r, prof) = zone_against_pressure(0.25);
    let held = Held {
        posture: Posture::Zone,
        since: 8.0,
    };
    assert_eq!(choose_posture(&held, 10.0, &r, &prof).0, Posture::Pressure);
}

#[test]
fn a_hair_better_posture_does_not_flicker() {
    let (r, prof) = zone_against_pressure(0.05);
    let held = Held {
        posture: Posture::Zone,
        since: 0.0,
    };
    assert_eq!(choose_posture(&held, 10.0, &r, &prof).0, Posture::Zone);
}

#[test]
fn finish_is_vetoed_at_full_foe_life() {
    let r = with(quiet(), &[(Input::FoeHpLow, 0.0), (Input::RageFull, 0.0)]);
    let (_, scores) = choose_posture(&fresh(10.0), 10.0, &r, &profile());
    assert_eq!(scores[Posture::Finish as usize], 0.0);
}

#[test]
fn a_foe_near_death_is_finished() {
    let r = with(
        quiet(),
        &[
            (Input::FoeHpLow, 0.9),
            (Input::Lead, 0.9),
            (Input::Threat, 0.0),
            (Input::Opening, 0.0),
        ],
    );
    assert_eq!(pick(&r), Posture::Finish);
}

#[test]
fn every_posture_is_reachable() {
    let mut seen = std::collections::HashSet::new();
    let mut rng = Rng::new(9);
    for _ in 0..4000 {
        let mut r = quiet();
        for v in &mut r.inputs {
            *v = if rng.next() < 0.5 {
                rng.next()
            } else if rng.next() < 0.5 {
                0.0
            } else {
                1.0
            };
        }
        seen.insert(pick(&r));
    }
    for p in Posture::ALL {
        assert!(seen.contains(&p), "{p:?} never chosen");
    }
}

#[test]
fn every_posture_weighs_every_category() {
    for def in &POSTURES {
        assert!(def.weights.iter().all(|w| *w > 0.0), "{:?}", def.posture);
    }
}
