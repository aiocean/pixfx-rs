# Roadmap

What the player kit and the tournament still need, most important first. Each item says what exists today.

## The player kit

1. **A reward written in Python.** Today the reward is fixed in Rust (`crates/pixfx-duel/src/train.rs`) and a player
   changes it only by editing Rust. `step` should also hand back the facts of each decision (damage dealt and taken,
   blows landed, guarded and punished, statuses, the round's end), so a trainer computes any reward in Python; the
   Rust reward stays the default.
2. **Install without Rust.** Today a player builds the module with `maturin develop`. Publish `pixfx-duel` as
   prebuilt wheels (macOS arm64 and x86_64, Linux x86_64), so `pip install pixfx-duel` is enough.
3. **Check a bot before submitting.** Today the ladder is a Rust example that measures one file. Add, callable from
   Python: the ladder for any file against any opponent (`pixfx_duel.ladder(file, opponent, seeds)`), the entry bar
   as a pass/fail, and the submission checks of `docs/tournament.md`. `export.py` gets `--out`, so a player's file
   does not have to replace the shipped one.
4. **Watch two files fight.** Today a player edits `POLICY_FILES` and builds the site. A page where two `.bin` files
   are dropped in and fight, with the seed shown and replayable.
5. **A standard environment.** A Gymnasium / PettingZoo wrapper over `VecEnv`, so Stable-Baselines3, CleanRL and RLlib
   work without reading our trainer.
6. **Training without a strong machine.** A notebook that fine-tunes from the shipped AI on a hosted GPU or CPU.

## The tournament

7. **Seasons.** The first season tag and a check that refuses a weights file built for another season's shapes.
8. **The runner.** Plays every match of a season from the submitted files, both ways, on a pool of machines, and
   writes results, ratings and replays. Deterministic, so a match can be split across machines and checked.
9. **Submission and results pages.** Upload, the checks' verdict, the table, each match's replays.
