# train

PPO for the landing duel's policy, on the Rust rules (`crates/pixfx-duel`) through `crates/pixfx-duel-py`.
Design: `docs/superpowers/specs/2026-11-15-duel-rules-in-rust-design.md` (Part 2). Scripts, not a CLI.

```
uv sync
uv run maturin develop --release -m ../crates/pixfx-duel-py/Cargo.toml   # after any change to the Rust rules
uv run pytest
uv run python ppo.py --run v1                     # runs/v1/log.jsonl, runs/v1/last.pt
uv run python export.py runs/v1/last.pt           # crates/pixfx-duel/weights/policy.bin + its Rust check
cargo run --release -p pixfx-duel --example ladder -- --seeds 150
```

- `net.py`: the network, mirroring `crates/pixfx-mind/src/policy/net.rs`, and its weights file; the HL-Gauss critic.
- `league.py`: who the learner fights (latest self 35%, a past self 25%, brain 15%, first mind 10%, scripted 15%).
- `ppo.py`: the trainer. CPU beats MPS for this network (10.2K vs 8.1K env steps/s on the M4).
- `export.py`: the shipped file and `crates/pixfx-duel/tests/fixtures/policy_check.json`, which
  `cargo test -p pixfx-duel --test policy_export` checks against Rust's own forward pass.

`runs/` stays out of git; the shipped weights go in.
