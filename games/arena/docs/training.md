# Training a bot

Everything here runs from a clone of pixfx-rs. The trainer is `games/arena/train/` (Python, PyTorch, `uv`); the rules
it trains on are the same Rust rules the page plays, through `games/arena/crates/pixfx-duel-py`. What a bot sees and
does is `docs/bot.md`.

## What you need

- Rust (the toolchain in `rust-toolchain.toml`), `uv`, Python 3.12.
- A fast machine for long runs. The env is the cost, and the trainer runs on CPU (CPU beats MPS for this network).
  Measured: about 42K env steps a second on the Mac mini the shipped AI trained on (30K each with two runs side by
  side), 6K-12K on a laptop. A fine-tune of 600 updates is about 20M steps, roughly 8 minutes at 42K; a run from
  scratch (3000 updates, about 98M steps) about 40-45 minutes.

## Setup

```sh
cd games/arena/train
uv sync
uv run maturin develop --release -m ../crates/pixfx-duel-py/Cargo.toml   # again after any change to the Rust rules
uv run pytest                                                            # 12 tests
```

## The environment

```python
import pixfx_duel as pd

env = pd.VecEnv(n=512, threads=8)          # 512 duels, stepped on 8 worker threads
env.set_weights(0, open("policy.bin", "rb").read())   # a weights file an opponent can fight with, as id 0
env.reset(i, seed, learner, style, foe_style, opponent, weights, limit)
obs, mask = env.observe()                  # float32[n, 354], bool[n, 30]
reward, done, won, length = env.step(actions, shaping)   # actions int64[n]
```

- `reset` starts env `i` on a fresh round. `learner` is the corner and so the school (0 Thunder, 1 Frost); `style` and
  `foe_style` are 0 aggressive, 1 balanced, 2 defensive; `opponent` is `"brain"`, `"first"`, `"aggressive"`,
  `"turtle"` or `"policy"` (then `weights` is a `set_weights` id, otherwise -1); `limit` is the round's time limit in
  seconds (the trainer uses 90).
- One step runs the duel to the learner's next decision. `done` marks the round's end; `won` is 1, -1, or 0 for a cut
  round; `length` is the round's seconds.
- A done env ignores its action until it is reset.
- The trainer's fight seeds start at 1,000,000 + 10,000,000 x `--seed` (11,000,000 by default). The ladder plays
  seeds 1-150; never train on them.

## The reward

Today the reward is computed in Rust (`crates/pixfx-duel/src/train.rs`), per decision:

| Term | Value |
|---|---|
| Round won / lost (a cut round is lost) | +1 / -1 |
| Damage | `shaping` x (dealt - taken) / 100, dealt and taken in % of a life (so 1.0 is a whole life), with `shaping` handed to `step` (the trainer anneals it to 0) |
| A fight gone quiet (no damage either way for 6 s) | -0.002 a decision |
| The same act (not wait) 6 times in a row | -0.002 |
| An act already 40% of the last 40 decisions | -0.004 |

To pay your bot differently today, edit those constants or `Env::step` and rebuild the module. A reward written in
Python, from facts `step` hands back, is the first item of `docs/roadmap.md`.

## The trainer

`ppo.py` is PPO in one file (CleanRL-style): 512 envs, 64-step rollouts, masked actions, an HL-Gauss critic.

```sh
uv run python ppo.py --run v1                                    # from scratch: 3000 updates
uv run python ppo.py --run v2 --init runs/v1/last.pt --updates 600 --lr 1e-4 --ent 0.003 0.002 --shaping-until 0
uv run python ppo.py --run v2 --resume runs/v2/last.pt           # continue a run that stopped
```

| Flag | Default | What |
|---|---|---|
| `--updates` | 3000 | PPO updates (each 512 x 64 steps) |
| `--envs`, `--steps`, `--threads` | 512, 64, 8 | rollout size and worker threads |
| `--lr` | 3e-4 | learning rate, falling linearly to 0 |
| `--ent` | 0.01 0.002 | entropy weight, from the first to the second over the run |
| `--shaping-until` | 0.6 | share of the run over which the damage term falls to 0 (0: none at all) |
| `--thunder` | 0.5 | share of fights in which the learner plays Thunder |
| `--snapshot-every` | 20 | how often the latest self joins the league |
| `--gamma`, `--lam` | 0.995, 0.95 | discount and GAE lambda |
| `--clip`, `--epochs`, `--minibatches` | 0.2, 4, 8 | PPO clip, passes over each rollout, minibatches per pass |
| `--device`, `--seed` | cpu, 1 | torch device; the seed for torch, numpy and the fight seeds |
| `--init` | | start from a `.pt` checkpoint or a `.bin` weights file, with a fresh schedule |
| `--resume` | | continue a run, optimiser and schedule included; the league restarts with only the current policy (past selves are not saved) |

It writes `runs/<run>/log.jsonl` every 10 updates (speed, losses, entropy, round length, and the win rate against
each opponent, overall and per school: `win/brain`, `win/latest/thunder`, ...) and `runs/<run>/last.pt` every 25.

**Opponents** (`league.py`): its latest self 35%, a past self 25%, the brain 15%, the first mind 10%, the scripted
pair 15%; schools and styles drawn per fight.

**A lesson from the shipped AI.** Trained with each school half the time, it played Thunder worse: Thunder won 40% of
its own mirror while the rules were even (the brain's mirror: 50.9%). Fine-tuned with `--thunder 0.9`, Thunder won
49.1%. Watch the per-school win rates in the log; a school you lose with needs more of the training.

## Making it your own

| You want | Change |
|---|---|
| A different style of play | the reward (`train.rs` today) |
| Different sparring partners | `MIX` in `league.py`, or other weights files through `set_weights` |
| Another algorithm | anything that drives `VecEnv` and ends in a weights file (`net.py`'s `to_bin`) |
| A head start | `--init ../crates/pixfx-duel/weights/policy.bin` |
| More of one school | `--thunder` |

The network's shape and its inputs are fixed for a season (`docs/bot.md`).

## Export and check

```sh
uv run python export.py runs/<run>/last.pt
```

It writes the weights file to `games/arena/crates/pixfx-duel/weights/policy.bin` (in your clone, the AI the page and
the ladder use) and `tests/fixtures/policy_check.json`: 32 observations from real fights and PyTorch's logits for
them. Then check that Rust computes the same:

```sh
cargo test -p pixfx-duel --test policy_export
```

`--out FILE` writes the weights elsewhere and `--check FILE` the check beside it. The Rust test checks the file built
into the crate (`weights/policy.bin`), so pass both when you keep the shipped file: an `--out` alone leaves a check
that no longer matches it, and the test fails.

## Measure it

```sh
cargo run --release -p pixfx-duel --example ladder -- --seeds 150 --telemetry [--weights FILE]
```

The ladder plays seeds 1-150, each both ways (as Thunder, then as Frost) for 3 rounds, rounds cut at 120 s. The
policy plays the balanced style against the brain, the first mind and the scripts; only the mirror draws styles. A cut
ends that fight and shows in the `cut` column; win rates count finished rounds. It prints:

| Match | The shipped AI | Bar |
|---|---|---|
| vs the brain | 95.9% | at least 75%, 65% each school |
| vs the first mind | 95.4% | at least 80% |
| vs the aggressive and turtle scripts | 91.1%, 100% | |
| its own mirror, Thunder's share | 49.1% | 45-55% |

`--telemetry` adds what each school's skills did in each mirror: casts, landed, damage, guarded, broken, punished,
final blows.

## Watch it

Copy your file into `games/arena/crates/pixfx-duel/weights/` under a new name, name it in `POLICY_FILES`
(`site/assets/landing.js`: `[left, right]`), build the site (`site/build.sh`, which copies every `weights/*.bin`)
and serve `site/dist`; the landing duel seats it.
