//! The shipped weights compute what PyTorch computed when `train/export.py` wrote them
//! (`tests/fixtures/policy_check.json`: observations from real fights, their masks, PyTorch's logits).

use pixfx_duel::seat::shipped;
use pixfx_mind::policy::{ACTIONS, OBS};
use serde_json::Value;

#[test]
fn exported_weights_match_pytorch() {
    let text = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/policy_check.json"
    ));
    assert!(text.is_ok(), "run train/export.py to write the fixture");
    let Ok(text) = text else { return };
    let check: Value = serde_json::from_str(&text).unwrap_or(Value::Null);
    let net = shipped();
    assert!(net.is_ok(), "the shipped weights load");
    let Ok(net) = net else { return };
    let cases = check["cases"].as_array().cloned().unwrap_or_default();
    assert_eq!(cases.len(), 32);
    for (k, case) in cases.iter().enumerate() {
        let floats = |v: &Value| {
            v.as_array()
                .map(|a| {
                    a.iter()
                        .map(|x| x.as_f64().unwrap_or(f64::NAN) as f32)
                        .collect::<Vec<f32>>()
                })
                .unwrap_or_default()
        };
        let obs = floats(&case["obs"]);
        let want = floats(&case["logits"]);
        let legal: Vec<bool> = case["mask"]
            .as_array()
            .map(|a| a.iter().map(|x| x.as_bool().unwrap_or(false)).collect())
            .unwrap_or_default();
        assert_eq!(
            (obs.len(), want.len(), legal.len()),
            (OBS, ACTIONS, ACTIONS),
            "case {k}"
        );
        let mut o = [0.0_f32; OBS];
        o.copy_from_slice(&obs);
        let got = net.logits(&o);
        for i in 0..ACTIONS {
            assert!(
                (got[i] - want[i]).abs() <= 1e-4 + 1e-4 * want[i].abs(),
                "case {k}, logit {i}: {} vs PyTorch {}",
                got[i],
                want[i]
            );
        }
        let best = |l: &[f32]| {
            (0..ACTIONS)
                .filter(|&i| legal[i])
                .max_by(|&a, &b| l[a].total_cmp(&l[b]))
        };
        assert_eq!(best(&got), best(&want), "case {k}: the best legal act");
    }
}
