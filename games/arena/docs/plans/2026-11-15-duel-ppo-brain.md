# Duel PPO Brain Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A small PPO-trained network drives both duel fighters on the page and measurably beats the utility brain and the first mind on a ladder, while fights stay balanced and watchable.

**Architecture:** `pixfx-mind` gains a game-neutral policy runtime: an observation encoder over `Sense`, a fixed action set with a mask, a scalar `f32` network loaded from a weights file, and a `PolicyMind` that keeps the reaction delay. `pixfx-duel` gains styles, a `PolicySeat`, scripted seats, a training environment (rewards, episodes, the league's opponents) and a ladder. A thin pyo3 crate batches environments for a CleanRL-style PyTorch trainer in `train/`. The trained weights ship in `pixfx-duel` and the page seats the policy through wasm.

**Tech Stack:** Rust 2024 (`libm`, `rayon` only in the py binding), pyo3 + numpy + maturin, Python 3.12 with PyTorch 2.8 (MPS or CPU), `uv`.

**Spec:** `games/arena/docs/specs/2026-11-15-duel-rules-in-rust-design.md` (Part 2, "Done when")

## Global Constraints

- clippy denies `unwrap_used`, `expect_used`, `panic`; `unsafe_code` is forbidden (the pyo3 crate too: pyo3's macros are allowed, our code has no `unsafe`).
- No platform float maths in engine code: `libm` (`expf`, `sqrtf`, ...) and `pixfx_draw::math`. The network is scalar `f32`, no FMA, no SIMD, one fixed loop order.
- Randomness only from the seed. A seed and the same inputs give the same fight natively and in wasm.
- Each crate tests itself (`cargo test -p <crate> --all-targets`); never `cargo test --workspace`.
- Reaction delay stays: the policy sees the foe through `pixfx_mind::view::Delayed` with the duel's `TEMPERAMENT.reaction` (0.16 s) and thinks every `TEMPERAMENT.think` (0.12 s, jittered 0.8-1.2 by its dice).
- The duel's rules may be tuned only in the balance task (Task 9), every change reported to the user; a rules bug the brain exploits is fixed in the rules, never penalised in the reward.
- Checkpoints and logs stay out of git (`train/runs/` ignored); the shipped weights file goes in.
- `Cargo.lock` is never committed by this plan (other sessions share the tree); commit only this plan's hunks through a temporary index.
- Build with `CARGO_BUILD_BUILD_DIR=/tmp/pixfx-build CARGO_INCREMENTAL=0` (the disk is nearly full).

## Review Focus

1. A masked action is never chosen, on any target: sampling with every legal logit at `-inf`-like values or one legal action left.
2. A weights file of the wrong shape, truncated, or from an older layout fails loading with an error naming the tensor, never runs garbage.
3. Hit-stop (`dt = 0`) and `hold` freeze the policy's clock and dice exactly as they freeze the utility brain's.
4. Native and wasm play the same policy fight bit for bit (the `f32` path, `expf`, the sampling dice).
5. An env episode that never ends (both stall) is cut at 90 s and paid as a loss to the learner, so stalling never pays.

---

### Task 1: The policy's eyes and hands (`pixfx-mind`)

**Files:**
- Modify: `crates/pixfx-mind/src/sense.rs` (`Sense.slots`), `crates/pixfx-mind/src/lib.rs`
- Create: `crates/pixfx-mind/src/policy/mod.rs`, `crates/pixfx-mind/src/policy/observe.rs`, `crates/pixfx-mind/src/policy/action.rs`
- Modify: `games/arena/crates/pixfx-duel/src/sense.rs` (fill `slots`)
- Test: `crates/pixfx-mind/tests/policy_observe.rs`, `crates/pixfx-mind/tests/policy_action.rs`

**Interfaces:**
- Produces:
  - `Sense.slots: Vec<Option<usize>>` (serde default empty): the game's castable slots in a fixed layout, each an index into the skills handed to the mind, `None` for an empty slot. The duel's layout is `[slash, dive, blink, bar 0, bar 1, bar 2, ultimate]` (`SLOTS = 7`).
  - `pub const SLOTS: usize = 7; pub const OBS: usize; pub const ACTIONS: usize = 9 + SLOTS * 3;`
  - `pub fn observe(me: &Me, foe: &Foe, threats: &[Threat], arena: &Arena, skills: &[Skill], sense: &Sense, tag: &[f32; TAG]) -> [f32; OBS]` with `pub const TAG: usize = 5` (game-given identity: the duel puts school one-hot 2 + style one-hot 3).
  - `pub enum Act { Wait, Left, Right, Jump, Duck, Guard, FlyHigh, Land, Meditate, Cast { slot: usize, aim: Aim } }`, `pub enum Aim { Foe, Ahead, Behind }`, `Act::index(self) -> usize`, `Act::from_index(usize) -> Option<Act>`.
  - `pub fn legal(me: &Me, foe: &Foe, skills: &[Skill], sense: &Sense) -> [bool; ACTIONS]`
  - `pub fn choice(act: Act, me: &Me, foe: &Foe, skills: &[Skill], sense: &Sense) -> Choice`

Observation groups (exact layout is the implementer's, documented in `observe.rs`, every value finite and in [-4, 4]): me (x from each wall over arena width, air over top, face, hp/100, qi/100, rage/rage_full, flying, hover, vx/200, can_act, silenced, sealed, barrier, shield, seeker_out); foe relative to me (dx, air, flying, hp, qi, `Doing` one-hot incl. act phase, act `SkillKind` one-hot, act progress, reach, damage, out_for, open_in, frozen, shocked, barrier, ready share); the 4 nearest threats (present bit, shape one-hot, dx, y, vx, lands_in, radius/reach, damage), padded with zeros; each of the 7 slots (present, ready, `SkillKind` one-hot, cost, damage × power, reach, windup, recover, ultimate); the tag.

Masking rules: a `Cast` is legal only when `me.can_act`, its slot holds a skill and `sense.ready` says so; `Ahead`/`Behind` are legal only for `Area`, `Field` and `Blink` kinds (the others take `Foe` alone, so no two legal actions do the same thing); movement acts need `can_act`; `FlyHigh` only when not flying, `Land` only when flying; `Wait` is always legal. Aim: `Foe` = foe.x, `Ahead` = foe.x + 30 × sign(foe.x − me.x), `Behind` = foe.x − 30 × sign(foe.x − me.x), clamped to the arena. `FlyHigh` → `Choice::Fly { height: arena.top * 0.6 }`; `Duck`/`Guard` hold 0.4 s.

- [ ] **Step 1: Write the failing tests**

```rust
// policy_observe.rs
#[test] fn every_observation_value_is_finite_and_bounded()        // over a grid of senses incl. NaN-free extremes
#[test] fn the_tag_lands_at_the_end_unchanged()
#[test] fn threats_past_the_fourth_nearest_are_dropped_and_missing_ones_are_zero()
// policy_action.rs
#[test] fn every_index_round_trips_through_act()                  // 0..ACTIONS, and ACTIONS is None
#[test] fn wait_is_always_legal_and_casts_need_a_ready_filled_slot()
#[test] fn only_areas_fields_and_blinks_take_ahead_or_behind()
#[test] fn a_cast_maps_to_the_slot_skills_name_and_aim()
```

- [ ] **Step 2: Run them to see them fail** — `cargo test -p pixfx-mind --all-features --test policy_observe --test policy_action` → FAIL: unresolved `pixfx_mind::policy`.
- [ ] **Step 3: Implement** `Sense.slots`, `policy::{observe, action}`; fill `slots` in `pixfx_duel::sense::sense_of` from the fighter's basics, bar and ultimate.
- [ ] **Step 4: Run** `cargo test -p pixfx-mind --all-features` and `cargo test -p pixfx-duel --all-targets` → PASS (the duel's goldens are unchanged: `slots` is only read by the policy).
- [ ] **Step 5: Clippy** both crates with `-D warnings`; commit `pixfx-mind: the policy's observation and action set`.

### Task 2: The network (`pixfx-mind`)

**Files:**
- Create: `crates/pixfx-mind/src/policy/net.rs`
- Test: `crates/pixfx-mind/tests/policy_net.rs`

**Interfaces:**
- Produces:
  - Weights file: little-endian, magic `b"PXNET1\0\0"`, `u32` tensor count, then per tensor: `u32` name length, UTF-8 name, `u32` rank, `u32` dims, `f32` data row-major.
  - Layout (names fixed): `in.w [W, OBS]`, `in.b [W]`; for `k` in `0, 1`: `blockk.ln.g [W]`, `blockk.ln.b [W]`, `blockk.up.w [H, W]`, `blockk.up.b [H]`, `blockk.down.w [W, H]`, `blockk.down.b [W]`; `out.ln.g`, `out.ln.b [W]`; `sem.w [W, W]`, `sem.b [W]`; `pi.w [ACTIONS, W]`, `pi.b [ACTIONS]`. `W = 128`, `H = 256`, SEM groups of 8 (16 simplices, softmax per group, temperature 1). Block: `x + down(relu(up(ln(x))))`. Input: `relu(in(x))`.
  - `pub struct Net`, `pub fn Net::load(bytes: &[u8]) -> Result<Net, NetError>`; `NetError` names the tensor and what is wrong (missing, shape, truncated, magic).
  - `pub fn Net::logits(&self, obs: &[f32; OBS]) -> [f32; ACTIONS]`
  - `pub fn sample(logits: &[f32; ACTIONS], legal: &[bool; ACTIONS], u: f32) -> usize`: softmax over legal logits only (max-subtracted, `libm::expf`), inverse CDF at `u` in [0, 1); always a legal index, `Wait` if nothing else is.

- [ ] **Step 1: Write the failing tests**

```rust
#[test] fn a_file_with_a_missing_tensor_is_refused_by_name()      // NetError message contains "sem.w"
#[test] fn a_truncated_file_is_refused()
#[test] fn identity_like_weights_give_the_logits_worked_by_hand()  // tiny hand-set weights, pi.b = [0,1,2,..]
#[test] fn sampling_never_picks_a_masked_action()                  // u over 0..1 step 1e-3, random-ish masks
#[test] fn sampling_with_one_legal_action_picks_it()
#[test] fn logits_are_pinned_for_a_seeded_network()               // weights from pixfx_mind::rng, hash of bits pinned
```

- [ ] **Step 2: Run** `cargo test -p pixfx-mind --all-features --test policy_net` → FAIL (no `net`).
- [ ] **Step 3: Implement** `Net` with plain loops (`for o in 0..out { let mut s = b[o]; for i in 0..inp { s += w[o*inp+i] * x[i]; } }`), LayerNorm with eps `1e-5` and `libm::sqrtf`.
- [ ] **Step 4: Run** → PASS; clippy; commit `pixfx-mind: a scalar f32 policy network and masked sampling`.

### Task 3: `PolicyMind` (`pixfx-mind`)

**Files:**
- Create: `crates/pixfx-mind/src/policy/mind.rs`
- Test: `crates/pixfx-mind/tests/policy_mind.rs`

**Interfaces:**
- Consumes: Task 1 `observe`, `legal`, `choice`; Task 2 `Net`, `sample`; `view::Delayed`; `rng::Rng`.
- Produces: `pub struct PolicyMind`; `PolicyMind::new(net: Rc<Net>, skills: Vec<Skill>, tag: [f32; TAG], reaction: f64, think: f64, seed: u32)`; `think(&mut self, sense: &Sense, dt: f64) -> Option<Choice>`; `reset()`; `hold(seconds)`; `last(&self) -> Option<(usize, [f32; OBS], [bool; ACTIONS])>` (the training env reads what it saw). Also `PolicyMind::decide_with(&mut self, sense, dt, pick: impl FnOnce(&[f32; OBS], &[bool; ACTIONS]) -> usize)` — the same clock and view, with the action supplied from outside (the trainer's learner corner).

- [ ] **Step 1: Failing tests:** `hit_stop_holds_its_clock_and_dice_still`, `it_thinks_about_eight_times_a_second`, `it_sees_a_windup_late_by_its_reaction`, `hold_keeps_it_quiet_for_that_long`, `the_same_seed_makes_the_same_choices`.
- [ ] **Step 2: Run** `cargo test -p pixfx-mind --all-features --test policy_mind` → FAIL.
- [ ] **Step 3: Implement** with the utility `Mind`'s clock rules (`mind.rs:115-125`): clock and hold advance only when `dt > 0`; jitter `think * (0.8 + 0.4 * rng)`; the sampling `u` is the next draw of the same `Rng`.
- [ ] **Step 4: Run** → PASS; `cargo test -p pixfx-mind --all-features`, clippy; commit `pixfx-mind: PolicyMind, a network that sees late and thinks on a clock`.

### Task 4: Styles and seats (`pixfx-duel`)

**Files:**
- Create: `games/arena/crates/pixfx-duel/src/style.rs`, `games/arena/crates/pixfx-duel/src/scripted.rs`
- Modify: `games/arena/crates/pixfx-duel/src/seat.rs` (`PolicySeat`), `src/view.rs` (`style`), `src/fighter.rs` (`Fighter.style`), `src/duel.rs`, `src/lib.rs`
- Test: `games/arena/crates/pixfx-duel/tests/style.rs`, `tests/seats.rs`

**Interfaces:**
- Produces:
  - `pub enum Style { Aggressive, Balanced, Defensive }`, drawn per fighter at `Duel::new` from a separate `Rng::new(seed ^ 0x5717_1e00)` (never from `state.rng`: the rules' dice and the goldens stay as they are); `FighterView.style`.
  - `pub fn tag(school: School, style: Style) -> [f32; TAG]`.
  - `PolicySeat::make(net: Rc<Net>) -> MakeSeat` (style from the fighter), `PolicySeat::with_style(net, Style) -> MakeSeat`.
  - `pub fn shipped() -> Result<Rc<Net>, NetError>`: `Net::load(include_bytes!("../weights/policy.bin"))`. Until Task 8 the file is a seeded random network written by a test helper, so everything downstream builds.
  - `Scripted::aggressive() -> MakeSeat` (walks in, casts the highest-damage ready skill at the foe, never guards), `Scripted::turtle() -> MakeSeat` (keeps its distance, guards every windup it sees, casts only bolts and areas).

- [ ] **Step 1: Failing tests:** `styles_are_drawn_from_the_seed_and_the_rules_goldens_hold` (runs `goldens.rs`'s check too), `a_policy_seat_only_ever_makes_legal_choices` (60 s, every applied choice was legal at its sense), `scripted_seats_finish_rounds` (aggressive vs turtle ends ≥ 1 round in 120 s for seeds 1-5).
- [ ] **Step 2: Run** `cargo test -p pixfx-duel --all-targets --test style --test seats` → FAIL.
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run** `cargo test -p pixfx-duel --all-targets` → PASS with `event_hashes.json` unchanged; clippy; commit `pixfx-duel: styles, the policy seat and scripted seats`.

### Task 5: The training environment (`pixfx-duel`)

**Files:**
- Create: `games/arena/crates/pixfx-duel/src/train.rs`
- Test: `games/arena/crates/pixfx-duel/tests/train.rs`

**Interfaces:**
- Consumes: Task 3 `PolicyMind::decide_with`, Task 4 seats.
- Produces:
  - `pub enum Opponent { Brain, First, Aggressive, Turtle, Policy(Rc<Net>) }`
  - `pub struct EnvConfig { seed: u32, learner: usize, style: Style, foe_style: Style, opponent: Opponent }`
  - `pub struct Env`; `Env::new(EnvConfig) -> Env`; `Env::observe(&self) -> (&[f32; OBS], &[bool; ACTIONS])`; `Env::step(&mut self, action: usize, shaping: f32) -> Step { reward: f32, done: bool, won: Option<bool>, length: f64 }`.
  - One episode is one round of a fresh duel. `step` applies the learner's action at its decision tick, then runs ticks until the learner's next decision or the round's end.
  - Reward: `+1` round won, `−1` round lost; cut at 90 s game time → `−1` (stalling never pays); plus `shaping × (a × dealt − b × taken) / 100` per step with `(a, b)` = aggressive `(1.5, 0.5)`, balanced `(1, 1)`, defensive `(0.5, 1.5)`; minus `0.002` per decision once 6 s pass with no damage either way; minus `0.002` when the same action index is chosen 6 times in a row (`Wait` excepted).

- [ ] **Step 1: Failing tests:** `an_episode_pays_plus_or_minus_one_at_its_end`, `an_episode_cut_at_ninety_seconds_is_a_loss`, `the_same_config_and_actions_replay_the_same_episode`, `the_mask_it_hands_out_is_the_one_it_obeys`, `shaping_zero_pays_only_the_outcome`.
- [ ] **Step 2: Run** → FAIL. **Step 3: Implement.** **Step 4: Run** `cargo test -p pixfx-duel --all-targets` → PASS; clippy; commit `pixfx-duel: the training environment`.

### Task 6: The ladder (`pixfx-duel`)

**Files:**
- Create: `games/arena/crates/pixfx-duel/src/ladder.rs`, `games/arena/crates/pixfx-duel/examples/ladder.rs`
- Test: `games/arena/crates/pixfx-duel/tests/ladder.rs`

**Interfaces:**
- Produces: `pub fn match_up(a: &dyn Fn() -> MakeSeat, b: &dyn Fn() -> MakeSeat, seeds: Range<u32>, limit: f64) -> Record`, each seed fought both ways for 3 rounds. `Record { rounds_won: [u32; 2] /* a as Thunder, a as Frost */, rounds_lost: [u32; 2], lengths: Vec<f64>, timeouts: u32, actions: [u32; ACTIONS] /* a's, policy seats only */, attack_share: f64, mean_distance: f64, think_ns: Option<f64> }` and `Record::win_rate()`, `Record::as_school(i)`.
- `examples/ladder.rs [--seeds 150] [--weights path]` prints the "Done when" table: balanced/aggressive/defensive vs brain, balanced vs first mind, mirror (Thunder share), mean round length, timeouts, most-used action share, per-style attack share and distance, decision cost.

- [ ] **Step 1: Failing tests:** `the_brain_against_itself_is_counted_both_ways` (wins + losses = rounds played, 2 × seeds × 3), `a_round_past_the_limit_counts_as_a_timeout`.
- [ ] **Step 2: Run** → FAIL. **Step 3: Implement.** **Step 4: Run** → PASS; `cargo run --release -p pixfx-duel --example ladder -- --seeds 20` prints the table (with the random network, near 0% against the brain); commit `pixfx-duel: the ladder`.

### Task 7: The batch binding (`games/arena/crates/pixfx-duel-py`)

**Files:**
- Create: `games/arena/crates/pixfx-duel-py/{Cargo.toml, pyproject.toml, src/lib.rs, AGENTS.md, CLAUDE.md -> AGENTS.md}`; root `Cargo.toml` `exclude` gains it (pyo3 stays out of the workspace, like `pixfx-gpui`).
- Test: `train/tests/test_env.py`

**Interfaces:**
- Produces (Python module `pixfx_duel`):
  - `VecEnv(n: int, threads: int)`; constants `OBS`, `ACTIONS`.
  - `reset(i: int, seed: int, learner: int, style: int, foe_style: int, opponent: str, weights: bytes | None)`; opponent is `"brain" | "first" | "aggressive" | "turtle" | "policy"`.
  - `observe() -> (obs float32[n, OBS], mask bool[n, ACTIONS])`.
  - `step(actions: int64[n], shaping: float) -> (reward float32[n], done bool[n], won int8[n], length float32[n])`; a done env waits for `reset(i, ...)` before it steps again (the trainer draws its next opponent).
  - Envs step in parallel over `threads` with rayon; each env owns its own nets (`Rc` inside one thread; `policy` weights parsed once per `reset`).

- [ ] **Step 1: Failing test** (`uv run pytest train/tests/test_env.py`): `test_random_legal_play_finishes_episodes` (64 envs, random legal actions, every env reaches `done` within 2000 steps, rewards at done are ±1), `test_same_seeds_same_rewards`.
- [ ] **Step 2: Run** → FAIL: `ModuleNotFoundError: pixfx_duel`.
- [ ] **Step 3: Implement**; build with `uv run maturin develop --release -m games/arena/crates/pixfx-duel-py/Cargo.toml`.
- [ ] **Step 4: Run** → PASS; `cargo clippy --manifest-path games/arena/crates/pixfx-duel-py/Cargo.toml -- -D warnings`; measure env steps per second with 512 envs (print it, ledger it); commit `pixfx-duel-py: batched environments for the trainer`.

### Task 8: The trainer and the export (`train/`)

**Files:**
- Create: `train/{pyproject.toml, .gitignore (runs/), README.md, net.py, ppo.py, league.py, export.py, tests/test_net.py}`
- Create: `games/arena/crates/pixfx-duel/tests/policy_export.rs`, fixture `games/arena/crates/pixfx-duel/tests/fixtures/policy_check.json` (written by `export.py`)

**Interfaces:**
- Consumes: Task 2's weights layout and names; Task 7's `VecEnv`.
- Produces:
  - `net.py`: `Policy(obs, actions, width=128, hidden=256)` mirroring Task 2 exactly, plus a value head (`Linear(W, 101)`) trained by HL-Gauss over `[-1.5, 1.5]`, sigma `0.75 × bin width`; value clipping off.
  - `ppo.py`: CleanRL-style PPO. Starting values: 512 envs, 64 steps a rollout, γ 0.995, λ 0.95, clip 0.2, lr 3e-4 annealed to 0, 4 epochs, 8 minibatches, entropy 0.01 → 0.002, grad clip 0.5, shaping 1 → 0 linearly over the first 60% of updates. Device MPS if faster than CPU on a 10-update benchmark, else CPU (ledger which).
  - `league.py`: per reset, opponent = latest snapshot 35%, a past snapshot 25%, brain 15%, first mind 10%, scripted 15% (aggressive/turtle even); a snapshot every 20 updates; learner corner and both styles drawn uniformly.
  - `export.py <checkpoint> --out games/arena/crates/pixfx-duel/weights/policy.bin`: writes the file and `policy_check.json` (32 fixed observations, their masks, PyTorch's logits).
  - Rust test `exported_weights_match_pytorch`: loads `policy.bin`, logits within `1e-4` of PyTorch's, and the argmax of legal logits equal on all 32.
- [ ] **Step 1: Failing tests:** `test_net.py::test_forward_shapes_and_mask`, `test_hl_gauss_targets_sum_to_one`, and the Rust `exported_weights_match_pytorch` → FAIL (no export yet).
- [ ] **Step 2: Implement**; `uv run python train/ppo.py --updates 20 --run smoke` runs and writes a checkpoint; `uv run python train/export.py train/runs/smoke/last.pt`.
- [ ] **Step 3: Run** `uv run pytest train/tests` and `cargo test -p pixfx-duel --test policy_export` → PASS; commit `train: PPO trainer, league and export` (not the smoke weights: restore the Task 4 placeholder before committing).

### Task 9: Train, measure, balance

No new code beyond fixes; an experiment loop with a record.

- [ ] **Step 1:** Train: `uv run python train/ppo.py --run v1` in the background (Monitor on its log), until the balanced-vs-brain win rate in its eval stops rising or 200M steps.
- [ ] **Step 2:** Export and run `cargo run --release -p pixfx-duel --example ladder -- --seeds 150 --weights <file>`. Ledger the full table.
- [ ] **Step 3:** For every bar missed, rule: train longer, change reward weights or league mix (ledger), or, if the brain exploits a rules bug or the mirror is outside 45-55%, fix or tune the rules (each change reported to the user, `event_hashes.json` re-pinned, training resumed from the last checkpoint).
- [ ] **Step 4:** When the table meets "Done when" except the page check, export to `games/arena/crates/pixfx-duel/weights/policy.bin`, re-run `exported_weights_match_pytorch`, commit `pixfx-duel: the trained policy` with the ladder table in the message.

### Task 10: The page plays the policy

**Files:**
- Modify: `crates/pixfx-wasm/src/duel.rs` (seats `"policy"`, default), `site/assets/duel.js` (doc), `site/assets/duel-info.js` or HUD only if the style must show (view carries it), `site/tests/mind-match.test.js`
- Create: `games/arena/crates/pixfx-duel/tests/goldens/policy_fight.json` (Rust test writes the view of seeds 1-3 after 60 s with policy seats), `site/tests/policy.test.js`

**Interfaces:**
- Consumes: Task 4 `PolicySeat::make`, `shipped()`.
- Produces: `new Duel(config, seats?)` with `"policy"` the default seat; `FighterView.style`.

- [ ] **Step 1: Failing tests:** `site/tests/policy.test.js::the wasm policy fight equals the native one` (wasm view after 60 s deep-equals `policy_fight.json`, seeds 1-3); `mind-match.test.js` turned from `todo` into `test` with the spec's bars on 20 seeds × both ways (balanced policy vs brain ≥ 75%, vs first ≥ 80%); a decision-cost test (`< 0.2 ms` mean per `PolicyMind` decision in wasm, measured over 5,000 decisions).
- [ ] **Step 2: Run** `bun test site/tests` → FAIL. **Step 3: Implement.** **Step 4:** rebuild wasm, `bun test site/tests` → PASS; `site/build.sh`; open the landing page in agent-browser, watch a round with `?mind`, screenshot; commit `duel: the page plays the trained policy`.
- [ ] **Step 5:** Ask the user to watch rounds with `?mind` (the last "Done when" row).
