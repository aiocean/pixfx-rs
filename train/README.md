# train

PPO for the landing duel's policy, on the Rust rules (`crates/pixfx-duel`) through `crates/pixfx-duel-py`.
Design: `docs/superpowers/specs/2026-11-15-duel-rules-in-rust-design.md` (Part 2). Scripts, not a CLI.

```
uv sync
uv run maturin develop --release -m ../crates/pixfx-duel-py/Cargo.toml   # after any change to the Rust rules
uv run pytest
uv run python ppo.py --run v1                     # runs/v1/log.jsonl, runs/v1/last.pt
uv run python ppo.py --run v2 --init runs/v1/last.pt --updates 600 --lr 1e-4 --ent 0.003 0.002 --shaping-until 0
                                                  # a fine-tune: a new run from v1's network
uv run python export.py runs/v1/last.pt           # crates/pixfx-duel/weights/policy.bin + its Rust check
cargo run --release -p pixfx-duel --example ladder -- --seeds 150
```

- `net.py`: the network, mirroring `crates/pixfx-mind/src/policy/net.rs`, and its weights file; the HL-Gauss critic.
- `league.py`: who the learner fights (latest self 35%, a past self 25%, brain 15%, first mind 10%, scripted 15%).
- `ppo.py`: the trainer. CPU beats MPS for this network (10.2K vs 8.1K env steps/s on the M4).
- `export.py`: the shipped file and `crates/pixfx-duel/tests/fixtures/policy_check.json`, which
  `cargo test -p pixfx-duel --test policy_export` checks against Rust's own forward pass.

`runs/` stays out of git; the shipped weights go in.

## Training on a remote host

A full run trains on a training machine, not a laptop: the training machine runs about 42K env steps/s (a laptop 6K-12K), so
3000 updates take about 45 minutes, and a crash or sleep on a laptop cannot stop it.

- Reach it as `ssh <train-host>`.
- The code must already be on that machine (clone or sync it); `.venv` is not copied, so `long-run.sh` builds it there.
- Start a run detached, so it outlives the ssh session and the training machine does not sleep:
  ```
  ssh <train-host> 'cd pixfx-rs/train &&
    nohup caffeinate -is ./long-run.sh frames > ~/train-frames.log 2>&1 < /dev/null &'
  ```
  `long-run.sh` builds the rules, runs pytest, trains, and resumes from `runs/<run>/last.pt` (saved every 25
  updates) if the trainer dies. Its log ends in `TRAIN-DONE` or `FAIL`.
- Progress is `runs/<run>/log.jsonl`, which syncs back to a laptop with the rest of `runs/`; export and the ladder run
  on a laptop.
- One run at a time: check `pgrep -fl "ppo.py"` on the training machine before starting, since another session may be training.
