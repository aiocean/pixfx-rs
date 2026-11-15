# The duel's rules in Rust, and a brain trained by PPO

## Intent

The landing duel is two AI fighters, Thunder and Frost, that people watch and bet points and coins on. Its brain
(`pixfx-mind`, a hand-written utility AI) is not smart: over 900 simulated rounds it wins 55.0% ±3.3 against the
duel's first mind, the simple JS mind it replaced (52.0% ±4.6 as Thunder, 58.0% ±4.6 as Frost; 150 seeds, each
played both ways, 2026-11-15). `site/tests/mind-match.test.js` has been a `todo` since the brain landed, because
the brain cannot look past its hand-written moves.

This work makes the brain smarter by training it. A small neural network learns to fight by reinforcement learning
(PPO) against a league of opponents. The duel's rules move from `site/assets/duel.js` into a Rust crate first, so
training runs on the same rules the page plays, fast and natively, and so there is one copy of the rules.

Why the fighters must be smart, for people who watch and bet:

- **Fights that look skilled.** It sidesteps a bolt at the last moment, punishes a whiffed cast, and saves the
  ultimate for the finish. It never spams one move.
- **A winner that is hard to call.** Thunder against Frost stays near even as both get smarter, so a bet is a
  read, never a sure thing.
- **Odds worth studying.** Fighters play in recognisable styles, so a bettor who studies them guesses better than
  chance.

## Goals

1. The duel's rules live in one Rust crate, `crates/pixfx-duel`. The page, the HUD, the tooltips and the bout
   pages run on it through wasm. The only behaviour change is fixed ticks (below).
2. A PPO-trained policy in `pixfx-mind` drives both fighters on the page. It is the same on every target: the same
   seed and the same inputs give the same fight natively and in wasm.
3. The new brain is measurably smarter on a ladder of opponents, and the fights stay balanced and watchable (see
   "Done when").

## Non-goals

- **Betting:** accounts, points, coins, odds and payouts. That is the next project, and it needs one thing from this
  work: a fight is fully determined by its seed and the code. The betting project must therefore close bets before
  the seed is revealed. Otherwise anyone can run the fight ahead in their browser and know the winner.
- **A player-controlled fighter.**
- **Search while playing** (MCTS, minimax). This work makes it possible later.
- **Pretrained or language models.** Researched 2026-11-15, none fit: they are too slow for 8 decisions a second
  on a phone, and too big for a web page.
- **A gpui duel.** The crate makes one possible, but none is built here.

## Anti-goals (failure even if it "works")

- A brain that wins by exploiting a rules bug, by stalling, or by spamming one move.
- Two copies of the rules, JS and Rust, kept alive side by side.
- Fights whose winner can be called from the school alone (one school above 55% in the mirror match).
- A page that stutters: a decision must cost well under 1 ms on a mid-range phone.
- Thinking faster than a person can see. The brain keeps a reaction delay, about 0.2 s, as the current one does.

## Constraints

- **The duel's rules are frozen** (skills, tiers, numbers, statuses) from the start of the port until the page runs
  on the Rust rules. The user agreed to this on 2026-11-15. Bug fixes found by the replay check are ported both
  ways.
- **After the port, the rules may be tuned** (user, 2026-11-15). Once the page runs on the Rust rules, a balance
  pass may change skill numbers and stats to bring the mirror match to 45-55%. A rule bug the brain exploits is
  fixed in the rules, not penalised in the reward. Every rule change is reported to the user. It resets the golden
  event-log hashes, and training continues from the last checkpoint.
- **The repo's rules** (`CLAUDE.md`):
  - clippy denies `unwrap_used`, `expect_used` and `panic`, and `unsafe_code` is forbidden;
  - no platform float maths in engine code: use `libm` and `pixfx_draw::math`;
  - randomness comes only from the seed;
  - each crate tests itself, with no repo-wide gate.
- **Determinism.** The same seed and the same choices give the same fight on every target. JS numbers are `f64`,
  so the port uses `f64` throughout. The duel's dice (mulberry32 on `u32`) port bit for bit.
- **Speed is play speed.** Release builds keep `opt-level = 3` with fat LTO.
- **The training machine is the user's Apple M4** (10 cores, 16 GB), with PyTorch 2.8 and its MPS GPU backend.
  Training must fit there.

## Approaches (the user picked B)

| | Approach | Trade-off |
|---|---|---|
| A | Train against the JS rules: PyTorch drives headless `bun` duels | No port, and it always trains on the live rules. Slower to train, and the rules stay in JS against the repo's "Rust decides" boundary. |
| **B** | **Port the rules to a Rust crate first, then train on it** | **1,343 lines to port before learning starts, and the rules are frozen meanwhile. Training runs 10-50x faster, there is one copy of the rules in Rust, gpui games can use it, and search becomes possible later.** |
| C | Keep the utility brain; the network only picks among its candidate moves | Learns fastest and keeps postures readable. It can never beat the ceiling of the hand-written moves, which is the problem. |

Models researched and left (2026-11-15):

- **Small pretrained LLMs** (Gemma 3 270M, SmolLM2, Qwen3.5-0.8B, LFM2.5-350M): hundreds of MB, and tens of
  ms per token on a phone.
- **Generalist agents.** JAT is weak (Atari IQM 0.14); NVIDIA NitroGen reads pixels, not state.
- **Mamba** is unstable in RL.
- **Decision Transformer** needs data from strong play.
- **DreamerV3 and TD-MPC2** pay to plan every step, which a cheap simulator does not need.

Evidence for the pick:

- PPO with LSTM agents beat pro players 62% in Blade & Soul arena, with a reaction delay imposed (arXiv 1904.03821).
- Mixing scripted opponents with self-play reached 94% where self-play alone stalled (Kim et al., CoG 2020).
- Residual MLPs with LayerNorm (SimBa, arXiv 2410.09754; BRO, arXiv 2405.16158) make small networks learn
  reliably.

## Part 1: the rules in Rust

### The crate

`crates/pixfx-duel` sits in a new **Game** layer above Engine. It depends on `pixfx-fighter` (`Body`),
`pixfx-mind` (`Mind`, `Sense`, `Choice`) and `libm`, and never on a shell. `CLAUDE.md`'s layer table gets the row,
and `crates/pixfx-mind/AGENTS.md` points "the rules of a fight" at `pixfx-duel` in place of `duel.js`.

It holds everything `duel.js` decides today:

- skills, schools and stats (`SPELLS`, `SCHOOLS`, `STATS`, `RARITY`, `TRAITS`, `LUCKY`, `PAYOFF`);
- the skill bar: draws, tiers, traits, lucky casts, stale slots;
- statuses: burn, chill, shock, freeze, silence, defence down, grit, unyielding, attune;
- bodies' rules (begin, release, hurt, block, clash, knock-out);
- shots in flight;
- rounds;
- the brain's senses: `senseOf`, `threatsOf`, `doingOf` and `apply`.

Its API:

- `Duel::new(DuelConfig { seed, left, right, ground, sky, effects })`. `effects` is the set of effect ids the
  shell has; it plays the part of today's `has`.
- `Duel::step(dt) -> Vec<Out>` advances the rules only in whole 1/60 s ticks: it carries the remainder of `dt`
  to the next call, with today's 0.25 s cap. Today `step` also runs a leftover tick of any length
  (`duel.js:463-473`), and the page hands it the frame time after hit-stop and slow-motion (`landing.js:173`).
  So the same seed fights differently on a 60 Hz and a 120 Hz screen. With whole ticks, a seed is one fight on
  every screen. Betting needs that, and the policy trains on whole ticks. (Builder's call, made in the design: the
  only behaviour change in the port. The replay test feeds both sides whole ticks, and the JS side gets the same
  change first.) `Out` is every command and
  event `step` returns today, with the same fields:
  - `play`, `move`, `stop`, `hit`, `tick`, `block`, `clash`, `land`, `bump`, `streak`, `dodge`, `heal`, `ko`;
  - `barrier`, `ward`, `lucky`, `tier`, `ultimate`, `unyielding`.
- A read-only view of each fighter: everything `duel-art.js`, `duel-info.js` and `bout.js` read today, including
  `skillBar`, `buffsOf` and the data tables. The shell reads data from Rust and never copies it.
- Minds are pluggable, as `createDuel`'s `minds` is today. A fighter's mind is one of:
  - the utility `Mind`;
  - the trained policy;
  - the first mind (`site/tests/mind-baseline.js`, ported as a ladder rung);
  - a scripted test mind.

### Proving the port: replay

Until the JS rules are deleted, `site/tests/replay.test.js` runs `duel.js` and the wasm `Duel` side by side:

- **Coverage:** 200 seeds, 120 s of game time each, with the utility minds and with the first mind.
- **Pass condition:** the `step` output and both fighters' view are equal at every step.
- **Float maths:** during the test, `duel.js`'s `Math.sin` and `Math.hypot` are swapped for the engine's `libm`
  ones, exported for the test, so a last-bit difference cannot fork a fight. `Math.sqrt` is exact in both.
- **After the switch:** golden event-log hashes per seed pin the Rust rules, as `frame_hash` pins effects. The JS
  rule tests (`duel`, `bar`, `traits`, `status`, `guard`, `buffs`, `fx`, `info`) become crate tests or run
  against the wasm.

### The page on the Rust rules

- `pixfx-wasm` gains a `duel` feature exposing `Duel`.
- `site/assets/duel.js` shrinks to a thin wrapper. It builds the wasm `Duel`, hands `step`'s output on, and
  re-exports the data and views under today's names (`createDuel`, `step`, `SPELLS`, `skillBar`, `buffsOf`, …), so
  `landing.js`, `duel-art.js`, `duel-info.js` and `bout.js` change as little as possible.
- The JS rules are deleted when replay passes and the page runs on Rust.
- `bout.js` (the effect pages' staging) is not fight rules: it stays JS and drives bodies itself. It reads its
  numbers (`SPELLS`, `STATS`, `damageOf`, `orbOffset`, the knockback it copies today) from the Rust data. (Builder's
  call, made in the design.)

## Part 2: the trained brain

### What the policy sees and does

- **Observation:** about 120 floats, encoded in Rust in `pixfx-mind` from the same delayed `Sense` the utility
  brain gets, so the reaction delay stays. Training and the page call the same encoder. It covers:
  - both fighters' position, velocity, life, qi, rage and statuses;
  - what each is doing;
  - its own 3 bar slots and its ultimate (skill kind, ready, cost, power, rarity, trait bits);
  - the nearest few threats in flight, padded and masked;
  - its school.

  Skills are known by `SkillKind`, never by name, as `pixfx-mind` requires today.
- **Action:** one discrete choice per decision, about 8 decisions a second:
  - wait, walk left, walk right, jump, duck, guard, fly (high or land), meditate;
  - cast slot 1, 2 or 3, or the ultimate, at a target picked from a small set (at the foe, ahead of it, behind it).

  Choices that cannot happen now (a slot not ready, not enough qi, silenced) are masked out. The choice maps to
  `pixfx_mind::Choice`.
- **Network:** a SimBa-style MLP of two residual LayerNorm blocks, width about 128, with a policy head and a value
  head, about 50-300K weights. A GRU layer is added only if the ladder shows it cannot read the foe's patterns.
- **On the page:**
  - the policy samples its choice with the fighter's seeded dice, so fights vary and stay deterministic;
  - its weights ship as a plain data file;
  - `pixfx-mind` runs them with its own scalar `f32` matrix code, with no FMA and no SIMD;
  - a test pins the outputs native against wasm by hash.

### Training

- **Env binding:** `crates/pixfx-duel-py`, a thin pyo3/maturin binding, the only Python-facing code. It runs many
  `Duel`s in a batch and steps them all from arrays of actions, returning observations, masks, rewards and dones.
- **Trainer:** a Python PPO trainer (CleanRL-style, PyTorch on MPS) in `train/` at the repo root, a `uv` project.
  It is scripts, not a CLI, the way each crate's `examples/` are.
- **One policy for both schools and all styles** (user, 2026-11-15). The observation tells it its school and its
  style (aggressive, balanced or defensive). It plays both sides, and one weights file ships.
- **Styles on the page:** each fight draws a style per fighter from its seed. The style is shown before the fight,
  so bettors can study it. It is part of each fighter's view for the HUD; the betting project decides where it
  shows.
- **League:** opponents are drawn from the policy's latest self, its frozen past snapshots, the utility brain, the
  first mind, and scripted styles (aggressive, turtling).
- **Reward:**
  - +1 for a round won, −1 for a round lost;
  - a small shaped term for damage dealt minus damage taken, decayed away during training;
  - penalties for stalling (no damage either way for a long stretch) and for repeating one move.

  The style input sets the reward's weights (an aggressive fighter is paid more for damage dealt, a defensive one
  for damage avoided), so each style plays recognisably.
- **Export:** `train/` writes the weights file that `pixfx-mind` loads, after checking it in Rust against PyTorch's
  outputs on a fixed batch.

## Done when

All measured over 150 seeds, each fought both ways (the method of the 55% run):

| Check | Bar |
|---|---|
| New brain, balanced style, vs utility brain | ≥ 75% of rounds overall, ≥ 65% as each school |
| New brain, aggressive and defensive styles, vs utility brain | ≥ 60% of rounds each |
| New brain, balanced style, vs first mind | ≥ 80% of rounds |
| Mirror match (new brain both sides, styles drawn) | Thunder wins 45-55%, after the balance pass if needed |
| Mean round length | 15-45 s; no fight hits the time limit |
| Most-used action | Under 40% of decisions |
| Styles | A viewer told the style can see it in the fight: share of attack actions, distance kept |
| Decision cost | Under 0.2 ms in wasm on the desktop; the phone checked once by hand |
| On the page | The user watches rounds with `?mind` and agrees they look skilled |

`site/tests/mind-match.test.js` turns from `todo` into a passing test against the new brain. Its bar is the table
above, and the file name it cites is this document.

## Left to the builder

- The exact observation layout, action set and target set, within the shapes above.
- PPO hyperparameters, league mixing ratios, reward weights, snapshot cadence and training length.
- The crate's module split, and how the wrapper exposes the views.
- How the trainer's runs, checkpoints and logs are laid out under `train/`. Checkpoints stay out of git; the
  shipped weights file goes in.
