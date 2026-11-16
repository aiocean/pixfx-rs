//! Every command and event of 120 s of brain-against-brain play, hashed per seed: a change to any rule, roll or
//! number shows here. A change meant to alter fights rewrites the file: delete it and run the test once.

mod common;

use common::{fight, run};
use std::fmt::Write as _;

const FILE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/goldens/event_hashes.json"
);

/// FNV-1a over `bytes`.
fn fnv1a(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325, |h, &b| {
        (h ^ u64::from(b)).wrapping_mul(0x0100_0000_01b3)
    })
}

/// Each seed's hash, as the file holds them.
fn hashes() -> String {
    let mut out = String::from("{\n");
    for seed in 1..=20_u32 {
        let log: String = run(&mut fight(seed), 120.0)
            .iter()
            .map(|o| format!("{o:?}\n"))
            .collect();
        let comma = if seed < 20 { "," } else { "" };
        let _ = writeln!(
            out,
            "  \"{seed}\": \"{:016x}\"{comma}",
            fnv1a(log.as_bytes())
        );
    }
    out.push_str("}\n");
    out
}

#[test]
fn event_log_hashes_are_pinned() {
    let got = hashes();
    match std::fs::read_to_string(FILE) {
        Ok(want) => assert_eq!(
            got, want,
            "the fights changed: if that was meant, delete {FILE} and run again"
        ),
        Err(_) => {
            assert!(std::fs::write(FILE, &got).is_ok(), "could not write {FILE}");
            eprintln!("wrote {FILE}: the first run pins the hashes");
        }
    }
}
