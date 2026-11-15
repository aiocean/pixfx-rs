# The Duel's Rules in Rust (Plan A of 2) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Move the landing duel's rules from `site/assets/duel.js` into a Rust crate, `crates/pixfx-duel`, prove the
port equal to the JS rules by replay, and run the page on it through wasm, so Plan B can train a brain on it.

**Architecture:**
- **Prepare the JS rules for replay.** They first change in two small ways that exact replay needs: whole 1/60 s
  ticks, and maths that can be swapped.
- **Port, module by module.** Each module of `pixfx-duel` ports one section of `duel.js` with that section's JS
  tests ported alongside.
- **Prove the port.** A bun replay test steps both and compares every output.
- **Switch the page.** `duel.js` becomes a thin wrapper over a wasm `Duel`.
- **Plan B** (the ladder, the training binding, PPO and the weights) is written after this plan lands, from the
  `Duel` API it produces.

**Tech Stack:** Rust 2021 (workspace crates, `libm`, `pixfx-draw::math`), `pixfx-fighter` (`Body`),
`pixfx-mind` (`Mind`), `wasm-bindgen` via `pixfx-wasm`, bun tests in `site/tests`.

**Spec:** `docs/superpowers/specs/2026-11-15-duel-rules-in-rust-design.md` (Part 1).

## Global Constraints

- clippy `unwrap_used`, `expect_used`, `panic` denied; `#![forbid(unsafe_code)]` (`CLAUDE.md` Rules).
- **No platform float maths in engine code:**
  - use `libm::sin`, `libm::hypot` and `libm::sqrt`;
  - use `pixfx_draw::math::{js_round, js_sign, or_one, js_min, js_max}` wherever `duel.js` uses `Math.round`,
    `Math.sign`, `x || 1`, `Math.min` and `Math.max`;
  - Rust's `f64::signum(0.0)` is `1.0` but JS `Math.sign(0)` is `0`: never call `signum`.
- **Randomness:** only the duel's mulberry32, ported bit for bit, drawn in the same order as `duel.js`.
- **Numbers:** `f64` everywhere a JS number was, `u32` only for the dice and keys.
- **The rules are frozen during this plan** (spec Constraints): no change to skills, tiers or numbers. The only
  behaviour change is whole ticks (Task 1).
- **Commits:** several sessions share this tree. Commit only your own files through a temporary index
  (`GIT_INDEX_FILE=/tmp/<x>/index git read-tree HEAD`, then `git update-index`), never `git commit -a`, never
  `Cargo.lock`. End every message with `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.
- **Cargo build dir:** `export CARGO_BUILD_BUILD_DIR=/tmp/pixfx-build` before cargo or wasm-pack.
- **Tests:** each crate tests itself (`cargo test -p pixfx-duel --all-targets`,
  `cargo clippy -p pixfx-duel --all-targets -- -D warnings`); never `cargo test --workspace`.

## Review Focus

1. **A fighter knocked out mid-cast while its shot is in flight.** The shot resolves or stops as in JS, and no
   effect key is left playing. Test in Task 5: `knockout_stops_every_effect_it_started`.
2. **`step(0.0)` during hit-stop, and a `dt` above 0.25 s after a stalled tab.** `step(0.0)` returns nothing and
   moves nothing. A large `dt` advances at most 15 ticks. Test in Task 6: `zero_dt_holds_and_big_dt_is_capped`.
3. **A school with an effect missing from `effects`.** The spells needing it are left out of the pool, as JS
   `usable` does, and the fight still runs. Test in Task 6: `missing_effects_leave_their_spells_out`.
4. **Equal x for two bodies, or for a body and a blow** (`Math.sign(0)`, `x || 1`). The JS answer is kept. Test in
   Task 5: `clash_at_equal_x_pushes_as_js`.
5. **A seed of 0, and seeds above 2^31.** mulberry32 matches JS (`seed >>> 0`). Test in Task 2:
   `rng_matches_js_for_edge_seeds`.

---

### Task 1: Make the JS rules replayable

**Files:**
- Modify: `site/assets/duel.js` (`step`, lines 463-473; `orbOffset` 364-367; `shot` 934; `blockChance` 210;
  `shockInterval` 61-63; `body` 533)
- Test: `site/tests/duel.test.js`

**Interfaces:**
- Produces:
  - `step(duel, dt)` runs only whole ticks of `TICK = 1/60`. It keeps `duel.carry`, the leftover `dt`, for the
    next call, and caps the time added per call at 0.25 s.
  - `export function useMath({ sin, hypot, sqrt })` swaps the three maths functions the rules call. It is for the
    replay test only, and the page never calls it.
  - Every `a ** n` with whole `n` (`SHOCK.fast ** (stacks - 1)`, `CHILL.slow ** me.chill`) becomes
    `powi(a, n)`, which multiplies in a loop starting from 1, so JS and Rust compute it the same way.

- [ ] **Step 1: Write the failing test** in `duel.test.js`, `describe("whole ticks")`.
  - Test `"a seed fights the same at 60 Hz and 144 Hz"`: one duel stepped at `1/60` for 60 s, one at `1/144` for
    60 s, same seed 5. Expect equal `wins`, `round` and both fighters' `hp` and `x`.
  - Test `"step(0) moves nothing"`.

- [ ] **Step 2: Run** `bun test site/tests/duel.test.js -t "whole ticks"`. Expected: FAIL on the 144 Hz duel.

- [ ] **Step 3: Implement** `step` with the carry, the `powi(base, n)` helper and `useMath`.
  - The `sin` swap covers `Math.sin` in `orbOffset`, the `hypot` swap covers `Math.hypot` in `shot`, and the
    `sqrt` swap covers `Math.sqrt` in `blockChance`.
  - Keep `useMath` out of `index`'s exports.

- [ ] **Step 4: Run** `bun test site/tests` (several minutes; a lone 30 s timeout passes when the test is rerun
  alone). Expected: every test passes, `1 todo`.

- [ ] **Step 5: Look at the page** at http://127.0.0.1:8770/ after
  `cp site/assets/duel.js site/dist/assets/`: rounds run and end by knockout, and the hit-stop still freezes the
  fight. **Commit:** `duel: whole ticks, so a seed is one fight on every screen; maths the replay can swap`.

### Task 2: The crate, its data, dice and maths

**Files:**
- Create:
  - `crates/pixfx-duel/{Cargo.toml,AGENTS.md,CLAUDE.md}`;
  - `src/lib.rs`, `src/data.rs` (every table and constant of `duel.js` lines 10-367), `src/rng.rs`;
  - `tests/data.rs`, `tests/fixtures/js_numbers.json`.
- Create: `site/tests/tools/dump-numbers.js` (writes the fixture from `duel.js`)
- Modify: root `Cargo.toml` (workspace member)

**Interfaces:**
- **Produces, in `pixfx_duel::data`:**
  - `pub enum School { Thunder, Frost }`;
  - `pub enum Kind { Melee, Dash, Blast, Bolt, Orb, Zone, Void, Heal, Ward, Barrier, Blink }`;
  - `pub enum Blow { Physical, Magic }`;
  - `pub struct Spell`: every field any entry of `SPELLS` has, as `f64` or `Option`, plus `name: &'static str`;
  - `pub fn spell(name: &str) -> Option<&'static Spell>`, and `pub fn spells() -> &'static [Spell]` in `SPELLS`'
    order;
  - `pub struct SchoolData` (`basics`, `pool`, `ultimate`, `signature`, `range`, `dry`, `aggression`, `tempo`,
    `sky`, `postures`, `bolt`, `circle`, `tint`);
  - `pub fn school(s: School) -> &'static SchoolData`;
  - `pub struct Stats`, and `pub fn stats(s: School) -> &'static Stats`;
  - `RARITY`, `TRAITS`, `LUCKY`, `PAYOFF`, `EVOLVES` and the constants (`RAGE`, `STALE`, `POOL_SIZE`, `FLY`,
    `FREEZE`, `CHILL`, `SHOCK`, `GRIT`, `DEFENCE`, `SPREAD`, `FURY`, `DEF_DOWN`, `BURN`, `WARD`, `BAR`, `EVOLVED`,
    `QI_REGEN`, `MEDITATE_REGEN`, `TEMPERAMENT`, `GUARD_COST`, `CHIP`, `GUARD`, `KNOCKBACK`, `ROUND_PAUSE`, `RING`,
    `BARRIER_THREAT`, `CAST_CIRCLE`, `CLASH`, `GUARDED`, `PAY_DRAW`, `DECOR`), with the same values;
  - `pub fn orb_offset(t: f64) -> (f64, f64)`, `pub fn block_chance(power: f64) -> f64`,
    `pub fn shock_interval(stacks: f64) -> f64`, `pub fn slot_power(rarity: Rarity, traits: &[Trait], evolved: bool)
    -> f64`, `pub fn fits(trait_: Trait, name: &str) -> bool`, and `pub fn powi(base: f64, n: i32) -> f64`.
- **Produces, in `pixfx_duel::rng`:** `pub struct Rng(u32)`, with `Rng::new(seed: u32)` and
  `fn next(&mut self) -> f64`. This is mulberry32 exactly as `duel.js:370-379`.
- **Deps:** `pixfx-fighter`, `pixfx-mind`, `pixfx-draw` (for `math`), `libm`. Optional `serde` / `ts` features as in
  `pixfx-mind`.

- [ ] **Step 1: Write `dump-numbers.js`.** It writes JSON holding:
  - the first 50 draws for seeds `[0, 1, 2, 7, 2**31 + 5, 2**32 - 1]`;
  - `orbOffset(t)` for t in 0..8 step 0.37;
  - `blockChance` for powers `[0, 1, 5, 20, 33.3, 80]`;
  - `shockInterval(1..5)`;
  - `slotPower` for every rarity × `{[], ["heavy"]}` × evolved;
  - `fits` for every trait × spell.

  Run it to write `tests/fixtures/js_numbers.json`.
- [ ] **Step 2: Write the failing tests** `tests/data.rs`:
  - `rng_matches_js_for_edge_seeds`;
  - `orb_offset_matches_js`, compared to 1e-12 (JS `Math.sin` against `libm::sin`; exact equality is pinned later
    by replay);
  - `block_chance_matches_js`, `shock_interval_matches_js`, `slot_power_matches_js`, `fits_matches_js`, all exact;
  - `every_spell_and_school_is_ported`: names and counts equal the JS keys in the fixture.
- [ ] **Step 3: Run** `cargo test -p pixfx-duel --test data`. Expected: FAIL (crate items missing).
- [ ] **Step 4: Implement** `data.rs` and `rng.rs`. Tables are `static` arrays of structs. Tints become a
  `&'static [(&'static str, Tint)]` per school; `Tint` holds optional `palette`, `accent`, `stars`, `fire`, `glow`,
  `width` and `height`.
- [ ] **Step 5: Run the tests and clippy.** Expected: PASS.
- [ ] **Step 6: Write the crate's docs.** `AGENTS.md` and `CLAUDE.md` follow `crates/pixfx-mind/AGENTS.md`'s
  sections. Layer: Game. It holds the duel's rules and never draws.
- [ ] **Step 7: Commit:** `pixfx-duel: the crate, the duel's tables, dice and maths`.

### Task 3: Fighters, statuses and damage

**Files:**
- Create: `src/fighter.rs`, `src/status.rs`, `src/out.rs`; tests `tests/status.rs`
- Port from: `duel.js` 42-84, 427-438, 815-868; `site/tests/status.test.js`, the damage cases of
  `site/tests/guard.test.js`

**Interfaces:**
- **Produces, `out.rs`:** `pub enum Out`, one variant per command and event `step` emits today, with JS's field
  names (serde `tag = "type"`, `rename_all = "camelCase"`):
  - `Play { key: u32, fx, x, y, opts: PlayOpts }`, `Move { key, x, y }`, `Stop { key }`;
  - `Hit { by, spell, power, launch, ranged, damage, crit, share, x, y }`, `Tick { by, damage, share, x, y }`;
  - `Block`, `Clash`, `Dodge`, `Heal`, `Ward`, `Barrier`, `Ko`, `Land`, `Bump`, `Streak`, `Lucky`, `Tier`,
    `Ultimate`, `Unyielding`, each with its fields from `duel.js`.

  `PlayOpts` holds optional `palette`, `accent`, `stars`, `fire`, `glow`, `width`, `height`, `flip` and `seed`.
- **Produces, `fighter.rs`:** `pub struct Fighter`. It has every field `fighter()` and `startRound` set
  (`duel.js:432, 451`), typed, with `body: pixfx_fighter::Body`; `act: Option<Act>`; `bar: Vec<Slot>`;
  `cool` and `cool_full` as `Vec<(&'static str, f64)>`, in insertion order as JS objects iterate.
  `pub enum Slot { Empty { left: f64, full: f64 }, Held { spell, power, evolved, rarity, traits, idle } }`.
- **Produces, `status.rs`** (each takes `&mut State` or the fighters it touches, as its JS twin; `State` is defined
  in Task 6, and until then a `pub struct State { rng, fighters: [Fighter; 2], out: Vec<Out>, .. }` stub may live
  in `fighter.rs`):
  - `control(state, f, ControlKind, seconds) -> f64`;
  - `add_chill(state, f, n)`, `add_shock(state, f, by, n)`, `shocking(state, f, dt)`, `burning(state, f, dt)`;
  - `take(state, f, amount) -> f64`, `gain_rage(f, n)`;
  - `pub fn damage_of(by: &Fighter, f: &Fighter, power, blow, crit, roll) -> f64`.

- [ ] **Step 1: Write the failing tests.** Port each `test(...)` of `status.test.js` to a `#[test]` with the same
  name in snake_case and the same assertions. Add `damage_matches_js_table`: `damage_of` for 12 cases, compared
  with values `dump-numbers.js` adds to the fixture.
- [ ] **Step 2: Run** `cargo test -p pixfx-duel --test status`. Expected: FAIL.
- [ ] **Step 3: Implement** the three files. Order of dice draws and of emitted `Out`s must equal the JS.
- [ ] **Step 4: Run the tests and clippy.** Expected: PASS.
- [ ] **Step 5: Commit:** `pixfx-duel: fighters, statuses and damage`.

### Task 4: The skill bar

**Files:**
- Create: `src/bar.rs`; tests `tests/bar.rs`, `tests/traits.rs`, `tests/buffs.rs`
- Port from: `duel.js` 145-161, 413-424, 1063-1196; `site/tests/{bar,traits,buffs}.test.js` (the rule cases, not
  the page cases)

**Interfaces:**
- **Consumes:** `Fighter`, `Slot`, `State` (Task 3).
- **Produces:**
  - `fight_pool(state, school, usable: &dyn Fn(&str) -> bool) -> Vec<&'static str>`;
  - `draw(state, f, i, first: bool, gone: Option<&str>)`, `spend(state, f, i)`, `raise`, `add_traits`;
  - `pub fn ready(f: &Fighter, name: &str) -> bool`, `cost_of`;
  - `pub fn skill_bar(f: &Fighter) -> Vec<SlotView>`, `pub fn buffs_of(f: &Fighter) -> Vec<Buff>`. Their fields are
    the JS objects' fields: `SlotView { spell, empty, ultimate, ready, left, full, charge, power, evolved, rarity,
    traits, stale }` and `Buff { id, good, stacks }`.

- [ ] **Step 1: Write the failing tests.** Port the rule tests of the three JS files, same names, same assertions.
  A test that needs a whole fight with a scripted mind waits for Task 6. Mark it `#[ignore = "Task 6"]` and port its
  body now.
- [ ] **Step 2: Run** `cargo test -p pixfx-duel --test bar --test traits --test buffs`. Expected: FAIL.
- [ ] **Step 3: Implement** `bar.rs`.
- [ ] **Step 4: Run the tests and clippy.** Expected: PASS, with only the `Task 6` ignores ignored.
- [ ] **Step 5: Commit:** `pixfx-duel: the skill bar`.

### Task 5: Casting, blows and shots

**Files:**
- Create: `src/act.rs` (`duel.js` 554-912), `src/shot.rs` (914-1059); tests `tests/act.rs`
- Port from: `site/tests/guard.test.js`, `site/tests/fx.test.js`

**Interfaces:**
- **Consumes:** Tasks 3 and 4.
- **Produces:**
  - `begin`, `release`, `blocks`, `turns_aside`, `turn_aside`, `drop_barrier`, `hurt(...) -> bool`, `clash`,
    `knock_out`, `break_act`, `settle`, `set_meditating`, `pose`;
  - `pub enum Shot { Bolt{..}, Orb{..}, Zone{..}, Spray{..}, Void{..} }` with the JS shot fields;
  - `shot(state, index, dt)`;
  - `play(state, fx, x, y, PlayOpts) -> u32` and `cast(...)` (tint merged as `{...tint, seed, ...opts}`).

  Shots iterate a snapshot of the list as JS does (`[...duel.shots]`), and removal keeps JS's order.
- [ ] **Step 1: Write the failing tests.**
  - Port `guard.test.js` and `fx.test.js`, same names.
  - Add `knockout_stops_every_effect_it_started`: over seeds 1-8, every `Play` key of a looping effect gets a `Stop`
    within 12 s, as `fx.test.js` checks; and at a knockout, no rune circle or barrier key stays open.
  - Add `clash_at_equal_x_pushes_as_js`: two fighters at the same x clash, and each gets the `vx` the JS rules give
    (`Math.sign(0 || 1)`), pinned from the fixture.

  The whole-fight tests stay `#[ignore = "Task 6"]` until Task 6.
- [ ] **Step 2: Run** `cargo test -p pixfx-duel --test act`. Expected: FAIL.
- [ ] **Step 3: Implement** `act.rs` and `shot.rs`.
- [ ] **Step 4: Run the tests and clippy.** Expected: PASS, except for the `Task 6` ignores.
- [ ] **Step 5: Commit:** `pixfx-duel: casting, blows and shots`.

### Task 6: The duel loop and its minds

**Files:**
- Create: `src/duel.rs`, `src/seat.rs`, `src/sense.rs` (`duel.js` 381-492, 1198-1317), `src/first_mind.rs` (port of
  `site/tests/mind-baseline.js`); tests `tests/duel.rs`, `tests/first_mind.rs`
- Port from: `site/tests/duel.test.js`; un-ignore every `Task 6` test

**Interfaces:**
- **Produces:**
  - `pub struct DuelConfig { seed: u32, left: f64, right: f64, ground: f64, sky: f64, effects: Vec<String> }`.
  - `pub struct Duel`, holding `pub state: State` and two seats:
    - `Duel::new(config, seats: [Box<dyn Seat>; 2]) -> Duel`;
    - `Duel::with_brains(config) -> Duel`, both seats `Brain`;
    - `fn step(&mut self, dt: f64) -> Vec<Out>`.
  - `pub struct State`: `seed, rng, left, right, ground, top, time, carry, round, wins: [u32; 2],
    winner: Option<usize>, pause, key: u32, shots: Vec<Shot>, fighters: [Fighter; 2], out: Vec<Out>, effects`, and
    `fn has(&self, fx: &str) -> bool`.
  - `pub trait Seat`:
    - `fn think(&mut self, state: &State, me: usize, dt: f64) -> Option<pixfx_mind::Choice>`;
    - `fn tell(&mut self, outcome: &pixfx_mind::Outcome)`;
    - `fn reset(&mut self)`;
    - `fn hold(&mut self, seconds: f64)`;
    - `fn posture(&self) -> Option<pixfx_mind::Posture> { None }`.
  - `pub struct Brain` (wraps `pixfx_mind::Mind`, built as `brain()` in `duel.js:1217-1239`); `pub struct
    FirstMind`; `pub struct Script(Box<dyn FnMut(&State, usize) -> Option<Choice>>)` for tests.
  - `pub fn sense_of(state: &State, me: usize) -> pixfx_mind::Sense`; `skill_of(name) -> pixfx_mind::Skill`.
- [ ] **Step 1: Write the failing tests.**
  - Port `duel.test.js`'s rule tests, same names.
  - Add `zero_dt_holds_and_big_dt_is_capped`.
  - Add `missing_effects_leave_their_spells_out`.
  - Add `first_mind_matches_js_choices`: for seeds 1-5, the first 600 choices of `FirstMind` equal those of
    `mind-baseline.js` in the same JS duel. `dump-numbers.js` records them, stepping with whole ticks.
- [ ] **Step 2: Run** `cargo test -p pixfx-duel`. Expected: FAIL.
- [ ] **Step 3: Implement** the four files. Keep `tick`'s order: think, apply and body for fighter 0 then 1;
  `separate`; shots; knockouts.
- [ ] **Step 4: Run** `cargo test -p pixfx-duel --all-targets` and clippy. Expected: PASS, no ignores left.
- [ ] **Step 5: Commit:** `pixfx-duel: the duel loop, its seats, the brain and the first mind`.

### Task 7: The wasm Duel, and replay against the JS rules

**Files:**
- Create: `crates/pixfx-wasm/src/duel.rs`; `site/tests/replay.test.js`
- Modify: `crates/pixfx-wasm/Cargo.toml` (feature `duel = ["dep:pixfx-duel"]`, in `default`), `src/lib.rs`,
  `crates/pixfx-wasm/CLAUDE.md`

**Interfaces:**
- **Produces (JS):**
  - `new Duel(config, seats?)`, where `seats` is `["brain" | "first", "brain" | "first"]` and defaults to brains;
  - `duel.step(dt) -> Out[]`;
  - `duel.view()`, returning `{ time, round, wins, winner, left, right, ground, fighters: FighterView[] }`.
    `FighterView` has every field `duel-art.js`, `duel-info.js` and `landing.js` read today: `index`, `school`,
    `hp`, `qi`, `rage`, `x`, `air`, `vx`, `vy`, `face`, `pose`, `poseT`, `flash`, `hitT`, `hitPower`, `frozen`,
    `shocked`, `dodged`, `rang`, `charges`, `barrier`, `burn`, `lucky`, `act`, `stats`, `untouchable`, `skillBar`,
    `buffs`, and the rest the HUD reads, found with `rg`.
  - Data getters: `duelSpells()`, `duelSchools()`, `duelStats()`, `duelRarity()`, `duelTraits()`, `duelConstants()`.
  - `damageOf(...)` and `orbOffset(t)`.
  - The replay maths: `libmSin(x)`, `libmHypot(a, b)` and `libmSqrt(x)`.

  It translates only. Every decision stays in `pixfx-duel`.
- [ ] **Step 1: Write the failing test** `site/tests/replay.test.js`.
  - Import `useMath` from `duel.js` and call `useMath({ sin: libmSin, hypot: libmHypot, sqrt: libmSqrt })`
    before creating any duel.
  - For seeds 1-200, a JS `createDuel` (brains) and a wasm `Duel` (brains) share the config `{ left: 0, right: 220,
    ground: 150, effects: every effect }`.
  - Step both by `1/60` for 120 s of game time. At every step, expect the `step` outputs to be deeply equal, and
    each JS fighter's view fields equal the wasm view's.
  - Seeds 1-20 also run with `["first", "first"]` against `mind-baseline.js`.
  - On a mismatch, print the seed, the time and the first differing field.
- [ ] **Step 2: Build and run.** Run `(cd packages/pixfx-web && bun run build:wasm)`, then
  `bun test site/tests/replay.test.js`. Expected: FAIL until `duel.rs` exists. Then, while porting slips remain,
  FAIL with a named first difference.
- [ ] **Step 3: Implement** `duel.rs` (bindings only). Then fix each named difference in `pixfx-duel`, with a crate
  test that pins it, until replay passes all seeds. A difference that is a bug in the JS rules is fixed in both;
  name it in the commit message.
- [ ] **Step 4: Run** `cargo test -p pixfx-duel -p pixfx-wasm`, clippy for both, and the replay test. Expected:
  all PASS.
- [ ] **Step 5: Commit:** `wasm: the duel's rules from Rust; replay proves them equal to duel.js`.

### Task 8: The page on the Rust rules

**Files:**
- Modify: `site/assets/duel.js`, rewritten as a wrapper. It is about 100 lines and keeps the names it exported:
  `createDuel`, `step`, `SPELLS`, `SCHOOLS`, `STATS`, `RARITY`, `TIERS`, `TRAITS`, `RAGE`, `DECOR`, `PAYOFF`,
  `damageOf`, `orbOffset`, `skillBar`, `buffsOf`, `ready`, `slotPower`.
- Modify: `site/assets/landing.js`, `duel-art.js`, `duel-info.js`, `bout.js`, only where they reached past those
  names.
- Delete: the rule tests ported to the crate (`site/tests/{bar,traits,buffs,status,guard,fx}.test.js`, the rule
  parts of `duel.test.js`), `site/tests/mind-baseline.js`, `site/tests/replay.test.js`,
  `site/tests/tools/dump-numbers.js`.
- Modify: `site/tests/info.test.js` (feeds `skillInfo` plain `SlotView` objects), `site/tests/mind-match.test.js`
  (seats `["brain", "first"]` through the wasm `Duel`).
- Create: `crates/pixfx-duel/tests/goldens.rs` and `tests/goldens/event_hashes.json`.
- Modify: `CLAUDE.md` (Game layer row in the layer table), `crates/pixfx-mind/AGENTS.md` (rules belong to
  `pixfx-duel`), `README.md` (crate list).

**Interfaces:**
- **Consumes:** Task 7's JS API.
- **Produces:** `createDuel({ seed, left, right, ground, has, sky })` returns an object with `fighters`, `time`,
  `round`, `wins`, `winner`, `left` and `right`, refreshed from `view()` after each `step`. The page's call sites stay
  as they are.

- [ ] **Step 1: Write the failing test** `goldens.rs`, `event_log_hashes_are_pinned`:
  - for seeds 1-20, hash every `Out` of 120 s of brain-against-brain play (FNV-1a over its debug string);
  - compare with `event_hashes.json`;
  - the first run writes the file when it is missing, and the test says so.
- [ ] **Step 2: Rewrite `duel.js`** as the wrapper and run the page's own tests:
  `bun test site/tests/info.test.js site/tests/duel.test.js site/tests/bout.test.js site/tests/duel-light.test.js
  site/tests/mind.test.js`. Expected: PASS.
- [ ] **Step 3: Delete** the JS rule tests and the replay tooling listed above.
  `rg "mind-baseline|dump-numbers|useMath" site` returns nothing.
- [ ] **Step 4: Run** `bun test --todo site/tests/mind-match.test.js`. Expected: it runs to the end and prints a
  win rate. It stays `todo`; Plan B sets its bar.
- [ ] **Step 5: Check the page.**
  - Run `bun site/preload.js site/dist` and copy the assets to `site/dist/assets/`.
  - In a fresh agent-browser session at http://127.0.0.1:8770/: rounds end by knockout, tooltips and buffs show,
    and the console shows no errors.
  - Measure `step` cost: the mean ms per frame over 600 frames, before (HEAD~) and after. Expected: not slower.
- [ ] **Step 6: Commit:** `duel: the page runs on pixfx-duel; the JS rules are gone`.
