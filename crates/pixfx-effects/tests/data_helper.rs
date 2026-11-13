//! The data-golden helper itself: it passes on a matching golden and fails on a mismatch.

mod common;

use common::data::{assert_data_matches, check_data_matches};

#[test]
fn an_effect_without_cues_act_or_state_matches_its_empty_golden() {
    assert_data_matches("fire/candle", &[("seed", 3.0)], "fire__candle__seed=3");
}

#[test]
fn a_golden_the_port_does_not_match_fails() {
    // the golden says there is a state; fire/candle has none
    let r = check_data_matches("fire/candle", &[("seed", 3.0)], "mismatch__state");
    assert!(
        r.as_ref().is_err_and(|e| e.contains("$.state")),
        "expected a $.state difference, got {r:?}"
    );
}

#[test]
fn a_missing_golden_fails() {
    assert!(check_data_matches("fire/candle", &[], "no_such_golden").is_err());
}
