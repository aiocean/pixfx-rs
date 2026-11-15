# pixfx-duel-py

Layer: **Shell** (for the trainer). Read the root `AGENTS.md` ("The boundary" and "Layers and where code goes")
before changing anything here; this file only says what this crate holds. The design is
`docs/superpowers/specs/2026-11-15-duel-rules-in-rust-design.md` (Part 2, "Training").

## Job

`import pixfx_duel` for the PPO trainer in `train/`: `VecEnv(n, threads)` runs `n` training episodes
(`pixfx_duel::train::Env`) on worker threads and hands numpy arrays back: `set_weights(id, bytes)` for `policy`
opponents, `reset(i, seed, learner, style, foe_style, opponent, weights_id, limit)`, `observe()` →
`(obs float32[n, OBS], mask bool[n, ACTIONS])`, `step(actions int64[n], shaping)` →
`(reward, done, won, length)`; `OBS` and `ACTIONS`. Each env is built and stepped on one thread (it holds `Rc`s
and seats); only plain arrays cross threads.

## Belongs here

- Translating `pixfx_duel::train` to Python and spreading envs over threads.

## Does not belong here

| If the code... | It goes to |
|---|---|
| decides a reward, an episode's end, an opponent's play | `pixfx-duel` (`train`, `seat`, `scripted`) |
| encodes what the policy sees or may do | `pixfx-mind` (`policy`) |
| is the trainer | `train/` |

## Dependencies

`pyo3`, `numpy`, `pixfx-duel`, `pixfx-mind`, `pixfx-effects` (the registry list only). It is its own workspace
(excluded from the root, like `pixfx-gpui`), so pyo3 never builds with the rest.

## Traps

- `unsafe_code` is `deny`, not `forbid`: pyo3's macros emit `unsafe` marked `allow`. This crate's code has none.
- The `#[pymodule]` function must not be named `pixfx_duel` (it would shadow the crate); its Python name is set
  with `#[pyo3(name = "pixfx_duel")]`.

## Before you finish

```
cd train && uv run maturin develop --release -m ../crates/pixfx-duel-py/Cargo.toml && uv run pytest
PYO3_PYTHON=$PWD/train/.venv/bin/python cargo clippy --manifest-path crates/pixfx-duel-py/Cargo.toml -- -D warnings
```
