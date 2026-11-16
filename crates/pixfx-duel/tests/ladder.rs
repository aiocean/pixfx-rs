//! The ladder: matches fought both ways, counted per school, with the round lengths and how each side played.

mod common;

use common::{all_effects, idle};
use pixfx_duel::ladder::{Match, match_up};
use pixfx_duel::seat::Brain;

#[test]
fn every_seed_is_fought_both_ways_and_counted() {
    let m = Match {
        seeds: 1..3,
        rounds: 1,
        limit: 300.0,
        effects: all_effects(),
    };
    let r = match_up(&Brain::make, &Brain::make, &m);
    assert_eq!(r.won[0] + r.lost[0], 2, "as Thunder: {r:?}");
    assert_eq!(r.won[1] + r.lost[1], 2, "as Frost: {r:?}");
    assert_eq!(r.timeouts, 0);
    assert_eq!(r.lengths.len(), 4);
    assert!(r.lengths.iter().all(|&l| l > 0.0 && l < 300.0));
    assert!(r.decisions > 0 && r.choices.values().sum::<u32>() == r.decisions);
    assert!((0.0..=1.0).contains(&r.win_rate()) && (0.0..=1.0).contains(&r.most_used()));
    assert!(r.mean_distance > 0.0);
}

#[test]
fn a_round_past_the_limit_counts_as_a_timeout() {
    let m = Match {
        seeds: 1..4,
        rounds: 3,
        limit: 5.0,
        effects: all_effects(),
    };
    let r = match_up(&idle, &idle, &m);
    assert_eq!(r.timeouts, 6, "{r:?}");
    assert!(r.lengths.is_empty());
    assert_eq!(r.won, [0, 0]);
    assert_eq!(r.lost, [0, 0]);
}

#[test]
fn telemetry_accounts_for_every_blow_and_every_round() {
    let m = Match {
        seeds: 1..3,
        rounds: 2,
        limit: 300.0,
        effects: all_effects(),
    };
    let r = match_up(&Brain::make, &Brain::make, &m);
    for i in 0..2 {
        let (me, foe) = (&r.schools[i], &r.schools[1 - i]);
        let won = r.won[i] + r.lost[1 - i];
        assert_eq!(me.finals.values().sum::<u32>(), won, "school {i}: {me:?}");
        // what its skills and statuses took is what the foe's life shows it lost
        assert!(
            (me.dealt() - foe.life_lost).abs() < 1e-6,
            "school {i}: dealt {} but the foe lost {}",
            me.dealt(),
            foe.life_lost
        );
        assert!(me.dealt() > 0.0);
        assert!(me.skills.values().map(|s| s.casts).sum::<u32>() > 0);
        assert!(me.skills.values().map(|s| s.landed).sum::<u32>() > 0);
    }
}
