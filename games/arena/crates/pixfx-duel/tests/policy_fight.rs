//! The shipped policy's fights, pinned (`goldens/policy_fight.json`): seeds 1-3, both corners the policy, 60 s at
//! 60 Hz, then the round, wins and each fighter's hp, qi, rage, x and air. `site/tests/policy.test.js` plays the same
//! fights in wasm and checks them against this file, so native and wasm play the same policy bit for bit.

mod common;

use common::{all_effects, config, pinned};
use pixfx_duel::duel::{Duel, TICK};
use pixfx_duel::seat::{PolicySeat, shipped};
use serde_json::{Value, json};

#[test]
fn policy_fights_are_pinned() {
    let net = shipped();
    assert!(net.is_ok(), "the shipped weights load");
    let Ok(net) = net else { return };
    let mut all = serde_json::Map::new();
    for seed in 1..=3_u32 {
        let mut duel = Duel::new(
            config(seed, all_effects()),
            [PolicySeat::make(net.clone()), PolicySeat::make(net.clone())],
        );
        for _ in 0..60 * 60 {
            duel.step(TICK);
        }
        let s = &duel.state;
        let fighters: Vec<Value> = s
            .fighters
            .iter()
            .map(|f| json!([f.hp, f.qi, f.rage, f.body.x, f.body.air]))
            .collect();
        all.insert(
            seed.to_string(),
            json!({ "time": s.time, "round": s.round, "wins": s.wins, "fighters": fighters }),
        );
    }
    pinned("policy_fight", &Value::Object(all));
}
