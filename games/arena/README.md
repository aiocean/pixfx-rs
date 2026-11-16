# Arena

Two cultivators fight in a pixel arena, Thunder against Frost, and a trained AI decides for each corner. The duel on
the pixfx landing page is this game. Players train AIs of their own, submit them as weights files, and the arena runs
a tournament between them to find the strongest.

The game runs on the pixfx engine (`crates/`): the engine draws, moves the bodies and plays the effects; the arena
holds the rules, the training environment and the trainer.

## What is here

| Path | What it holds |
|---|---|
| `crates/pixfx-duel` | The rules: the two schools, their stats and skills, the rotating bar, statuses, frame data, rounds, the seats that decide for each corner, the training env, the ladder. Deterministic from a seed. |
| `crates/pixfx-duel/weights/policy.bin` | The shipped AI, the one the landing page plays. |
| `crates/pixfx-duel-py` | The rules as a Python module (`import pixfx_duel`): many duels stepped in parallel for a trainer. |
| `train/` | The PPO trainer that made the shipped AI: the network, the opponents it trains against, export. |
| `docs/` | The docs below, and the design history (`docs/specs`, `docs/plans`). |
| `site/` | The arena's site, arena.aiocean.io: the duel full-screen, a page to watch two bots from their weights files and a seed, and the docs below as pages. `site/build.sh` builds it into `site/dist`. |

The duel on both sites is drawn by the pixfx site's page code (`site/assets/duel.js`, `site/assets/duel-hero.js` at
the repo root), through the engine's wasm (`crates/pixfx-wasm`, feature `duel`).

## Docs

| Doc | For |
|---|---|
| [docs/rules.md](docs/rules.md) | How a fight works: schools, skills, the bar, qi and rage, statuses, frame data, rounds. |
| [docs/bot.md](docs/bot.md) | What a bot sees (354 numbers), what it can do (30 acts), when it decides, the weights file. |
| [docs/training.md](docs/training.md) | How to train a bot: setup, the environment, reward, opponents, export, measuring it. |
| [docs/tournament.md](docs/tournament.md) | Seasons, what a player submits, how it is checked, how matches are played and ranked. |
| [docs/roadmap.md](docs/roadmap.md) | What the player kit still needs, in order. |

## Quick start (train a bot)

```sh
cd games/arena/train
uv sync
uv run maturin develop --release -m ../crates/pixfx-duel-py/Cargo.toml
uv run pytest
# fine-tune from the shipped AI: 600 updates, about 8 minutes at 40K steps/s
uv run python ppo.py --run mine --init ../crates/pixfx-duel/weights/policy.bin --updates 600 --lr 1e-4 \
  --ent 0.003 0.002 --shaping-until 0 --thunder 0.9
uv run python export.py runs/mine/last.pt   # writes ../crates/pixfx-duel/weights/policy.bin and its check
cd ../../.. && cargo run --release -p pixfx-duel --example ladder -- --seeds 150 --telemetry
```

`docs/training.md` explains each step and what to change to make the bot your own.
