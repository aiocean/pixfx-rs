# pixfx-mind Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Give each fighter of the landing duel a Rust brain that is a state machine of postures, reading its own
state, its skills, the foe and the arena, and prove it beats today's JS mind head to head.

**Architecture:** A new pure crate `crates/pixfx-mind` (Engine layer, `libm` only) holds the brain: a delayed view of
the senses, a `Reading`, a foe habit table, yomi levels, utility-scored postures with commitment and interrupt bands,
candidate generators per skill kind and a rank-then-weight pick. `pixfx-wasm` exposes it as `Mind` behind a `mind`
feature. `site/assets/duel.js` builds a `Sense` per tick and carries out the `Choice`; the old JS mind moves to
`site/tests/mind-baseline.js` as the yardstick of a match test.

**Tech Stack:** Rust 2024 (`libm`, `serde`/`tsify` behind `ts`), wasm-bindgen, plain ES modules, `bun test`.

**Spec:** `docs/superpowers/specs/2026-11-14-pixfx-mind-design.md`

## Global Constraints

- `pixfx-mind` depends on `libm` only; `serde`, `tsify`, `wasm-bindgen` optional behind `serde` / `ts` features
  (same shape as `crates/pixfx-light/Cargo.toml`). Never `pixfx-fighter`, `pixfx-scene`, `pixfx-wasm`.
- Workspace lints: `unwrap_used`, `expect_used`, `panic` denied, `unsafe_code` forbidden; platform float maths
  banned by `clippy.toml` (use `libm::*`).
- Pure and deterministic: no clock, no I/O; randomness only from the seed given to `Mind::new`.
- The brain never names a duel spell or school: skills are data with a `SkillKind`.
- serde shapes are camelCase; enums are internally tagged with `"type"` in lowerCamel (`{"type":"move","dir":-1}`).
- The page passes no `minds` and gets the Rust brain for both fighters; the posture caption shows only with `?mind`.
- Match pass bar: the new brain wins **at least 60%** of rounds against the baseline, schools swapped per seed.
- Tests: `cargo test -p pixfx-mind`, `cargo clippy -p pixfx-mind --all-targets -- -D warnings`, the `pixfx-wasm`
  "Before you finish" list, `bun test site/tests`. Never `cargo test --workspace`.
- Shared tree: commit only this plan's files through a temporary index (`GIT_INDEX_FILE`), never `git commit -a`.

## Review Focus

1. A `Sense` from JS with a misspelt or missing field: `new Mind`/`think` throws naming the field (strict parse),
   never silently defaults. Test in Task 8.
2. A fighter that holds no ranged skills, or every skill on cooldown: the brain still moves, guards or waits and
   never returns a `Cast` of an unready skill. Test in Task 6.
3. A round reset mid-thought (`reset()`) leaves no memory of the old round: habits, calm, delayed view and posture
   are cleared. Test in Task 7.
4. `dt = 0` (hit-stop) for many ticks: no thought fires, no clock advances, the delayed view does not age. Test in
   Task 7.
5. A threat list with a hundred entries or none, and an arena narrower than the preferred range: no NaN in any
   score, a choice is still made. Test in Task 6.

---

## File Structure

```
crates/pixfx-mind/
  Cargo.toml
  AGENTS.md, CLAUDE.md -> AGENTS.md
  src/lib.rs        re-exports; crate doc
  src/sense.rs      Sense, Me, Foe, Doing, Phase, Arena, Threat, Shape (what JS hands in each tick)
  src/skill.rs      Skill, SkillKind, Inflicts, Profile, BodySize (handed in once)
  src/choice.rs     Choice, Posture, Thought, Why, Outcome (what comes out / is told)
  src/rng.rs        Rng (mulberry32, u32 maths)
  src/curve.rs      Curve, Input, Consideration (pure scoring primitives)
  src/view.rs       Delayed: ring of past senses, foe seen `reaction` seconds late
  src/reading.rs    Reading::of(&Sense, &[Skill], &BodySize, &Profile) and foe posture guess
  src/habits.rs     Habits: decaying n=1,2 counts of FoeMove per foe posture
  src/yomi.rs       Yomi: level 0/1/2 weights moved by payoff
  src/posture.rs    POSTURES tables, choose_posture with commitment, hysteresis, interrupt bands
  src/candidates.rs Candidate, Category, generators per skill kind and threat shape
  src/pick.rs       rank-then-weight pick, punishment feedback weights
  src/mind.rs       Mind: new, reset, hold, think, tell
  tests/{curve,view,reading,habits,posture,candidates,mind}.rs
crates/pixfx-wasm/src/mind.rs, Cargo.toml (feature `mind`), src/lib.rs
packages/pixfx-canvas2d/src/index.ts  re-export Mind
site/assets/duel.js, site/assets/landing.js, site/tests/engine.js
site/tests/mind-baseline.js, site/tests/mind-match.test.js
README.md, AGENTS.md (Engine row), CHANGELOG.md
```

### Task 1: Crate, types and seeded random

**Files:** Create `crates/pixfx-mind/{Cargo.toml,AGENTS.md,CLAUDE.md,src/lib.rs,src/sense.rs,src/skill.rs,src/choice.rs,src/rng.rs}`, `tests/types.rs`.

**Interfaces — Produces** (all `#[derive(Debug, Clone, PartialEq)]`, serde behind `serde`, tsify behind `ts`):

```rust
// skill.rs
pub enum SkillKind { Melee, Dive, Dash, Blast, Bolt, Seeker, Area, Field, Heal, Ward, Barrier, Blink }
pub struct Inflicts { pub freeze: f64, pub shock: bool, pub silence: f64, pub def_down: f64, pub burn: bool, pub chill: bool, pub pull: f64 }
pub struct Skill {
    pub name: String, pub kind: SkillKind, pub cost: f64, pub windup: f64, pub recover: f64, pub damage: f64,
    pub reach: f64,  // melee/blast range, dash/blink distance, area/field reach
    pub radius: f64, pub speed: f64,
    pub lands: f64,  // seconds from release until an area/field lands (sigil + impact, or life)
    pub strikes: u32, pub ultimate: bool, pub grounded: bool, pub inflicts: Inflicts,
}
pub struct BodySize { pub half_width: f64, pub chest: f64, pub body: f64, pub duck: f64, pub walk: f64 }
pub struct Profile {
    pub aggression: f64, pub tempo: f64, pub spread: f64, pub range: f64, pub dry: f64, pub sky: f64,
    pub risk: f64, pub floor: f64, pub lead: f64, pub prior: f64, pub reaction: f64, pub think: f64,
    pub posture_scale: [f64; 8],   // indexed by Posture as usize; 1.0 = as tabled
}
// sense.rs
pub struct Arena { pub left: f64, pub right: f64, pub top: f64 }
pub enum Phase { Windup, Recover }
pub enum Doing { Idle, Move { dir: f64 }, Guard, Meditate, Fly, Act { kind: SkillKind, phase: Phase, t: f64, windup: f64, recover: f64, reach: f64, damage: f64 } }
pub struct Me { pub x: f64, pub air: f64, pub face: f64, pub hp: f64, pub qi: f64, pub rage: f64, pub rage_full: f64,
    pub flying: bool, pub hover: f64, pub vx: f64, pub can_act: bool, pub silenced: bool, pub sealed: bool,
    pub barrier: bool, pub shield: bool, pub seeker_out: bool }
pub struct Foe { pub x: f64, pub air: f64, pub flying: bool, pub hp: f64, pub qi: f64, pub doing: Doing,
    pub out_for: f64, pub open_in: f64, pub frozen: f64, pub shocked: f64, pub barrier: bool,
    pub ready: Vec<Skill> }        // the foe's ready damaging skills, for the punisher read
pub enum Shape { Projectile { x: f64, y: f64, vx: f64 }, Area { x: f64, radius: f64, reach: f64, lands_in: f64, grounded: bool },
    Seeker { x: f64, y: f64, radius: f64 }, Field { x: f64, reach: f64 }, Barrage { lands_in: f64 } }
pub struct Threat { pub age: f64, pub damage: f64, pub shape: Shape }
pub struct Sense { pub time: f64, pub me: Me, pub foe: Foe, pub arena: Arena, pub threats: Vec<Threat>, pub ready: Vec<bool> }
// choice.rs
pub enum Posture { Neutral, Pressure, Zone, Punish, Defend, Evade, Recover, Finish }  // `as usize` 0..8, Posture::ALL
pub enum Choice { Move { dir: f64 }, Jump, Duck { hold: f64 }, Guard { hold: f64 }, Fly { height: f64 }, Meditate, Wait, Cast { skill: String, target: Option<f64> } }
pub enum Outcome { Landed { skill: String }, Punished { skill: String } }
pub struct Why { pub postures: Vec<(Posture, f64)>, pub top: Vec<(Choice, f64)>, pub level: u8, pub predicted: Option<String> }
pub struct Thought { pub choice: Choice, pub posture: Posture, pub why: Option<Why> }
// rng.rs
pub struct Rng(u32); impl Rng { pub fn new(seed: u32) -> Rng; pub fn next(&mut self) -> f64 /* [0,1) */ }
```

`Sense.ready[i]` is the readiness of the i-th skill passed to `Mind::new`.

- [ ] **Step 1:** Write `tests/types.rs`: `rng_replays_a_seed` (two `Rng::new(7)` give the same 100 values, all in
  `[0,1)`; `Rng::new(8)` differs), `rng_matches_mulberry32` (first value of seed 1 equals what `random(1)()` in
  `site/assets/duel.js:204` gives: `0.6270739405881613`), and behind `#[cfg(feature = "serde")]`
  `sense_reads_camel_case` (a JSON `Sense` with `"canAct"`, `"outFor"`, `{"type":"act","kind":"melee","phase":"windup",...}`
  parses; `Choice::Move{dir:-1.0}` serialises to `{"type":"move","dir":-1.0}`).
- [ ] **Step 2:** Run `cargo test -p pixfx-mind --all-features` → fails (crate missing).
- [ ] **Step 3:** Write the crate: `Cargo.toml` like `crates/pixfx-light/Cargo.toml` without `pixfx-draw`;
  `AGENTS.md` in the shape of `crates/pixfx-light/AGENTS.md` (Layer Engine; Job: a fighter's brain; does not belong:
  the rules of a fight, which stay in the game; Before you finish: the two cargo commands); `CLAUDE.md` symlink;
  the types above; `Rng` as mulberry32 on `u32` with `wrapping_*`, matching duel.js bit for bit.
- [ ] **Step 4:** `cargo test -p pixfx-mind --all-features` and `cargo clippy -p pixfx-mind --all-targets --all-features -- -D warnings` pass.
- [ ] **Step 5:** Commit `pixfx-mind: the crate, its senses, skills and choices`.

### Task 2: Curves and considerations

**Files:** Create `src/curve.rs`, `tests/curve.rs`.

**Interfaces — Produces:**
```rust
pub enum Curve { Linear { m: f64, b: f64 }, Power { k: f64 }, Logistic { k: f64, mid: f64 }, Step { at: f64 }, Inverse }
impl Curve { pub fn at(&self, x: f64) -> f64 }       // input clamped to [0,1], output clamped to [0,1]
pub enum Input { Qi, Hp, FoeHpLow, Lead, Opening, Threat, InRange, Close, CloseReady, RangedReady, AreaReady,
    Cornered, FoeCornered, FoePressing, FoeZoning, FoeRecovering, RageFull, Calm }  // each a Reading field in [0,1]
pub struct Consideration { pub input: Input, pub curve: Curve }
pub fn score(considerations: &[Consideration], reading: &Reading) -> f64  // product; returns 0 at the first 0 (veto, early out)
```
(`Reading` comes from Task 3; implement `score` in this task against `Reading::input(Input) -> f64`, a method Task 3
defines. To keep the order, Task 2 defines `pub trait Inputs { fn input(&self, i: Input) -> f64; }` and `score` takes
`&impl Inputs`; Task 3's `Reading` implements it.)

- [ ] **Step 1:** Tests: `linear_clamps` (`Linear{m:2,b:0}.at(0.8) == 1.0`, `.at(-1) == 0`), `inverse_flips`
  (`Inverse.at(0.25) == 0.75`), `logistic_is_half_at_mid` (`Logistic{k:10,mid:0.4}.at(0.4) == 0.5`), `step_is_a_gate`,
  `score_vetoes_on_zero` (a stub `Inputs` that counts calls: considerations `[zero, one, one]` score 0 after one call),
  `score_multiplies` (`0.5 * 0.5 == 0.25`).
- [ ] **Step 2:** Run → fail. **Step 3:** Implement with `libm::exp` for the logistic. **Step 4:** Run → pass, clippy clean.
- [ ] **Step 5:** Commit `pixfx-mind: response curves and considerations`.

### Task 3: Seeing late, and the reading

**Files:** Create `src/view.rs`, `src/reading.rs`, `tests/view.rs`, `tests/reading.rs`.

**Interfaces — Produces:**
```rust
pub struct Delayed { /* ring of (time, Foe, Vec<Threat>) */ }
impl Delayed { pub fn new() -> Delayed; pub fn push(&mut self, sense: &Sense); pub fn seen(&self, now: f64, reaction: f64) -> Option<(Foe, Vec<Threat>)>; pub fn clear(&mut self) }
pub struct Reading { /* every Input in [0,1] plus raw values below */ pub d: f64, pub level: bool, pub opening: f64,
    pub busy: f64, pub open_in: f64, pub free: bool, pub punisher: f64, pub foe_posture: Posture, pub lead: f64 }
impl Reading { pub fn of(me: &Me, foe: &Foe, threats: &[Threat], arena: &Arena, skills: &[Skill], ready: &[bool], body: &BodySize, profile: &Profile, calm: f64) -> Reading }
impl Inputs for Reading { ... }
```
`seen` returns the newest entry at least `reaction` old, and drops threats younger than `reaction` by their `age`;
before any entry is that old it returns the oldest. Ring capacity: entries newer than `now - 1.0` s are kept.
`busy`, `opening`, `free`, `open_in`, `punisher` follow `attacks()`/`punisher()` in `site/assets/duel.js:962-1076`
with `foe.doing` in place of `foe.act`. Foe posture guess: `Act` of Melee/Blast/Dash/Dive in reach or `Move` toward
me → `Pressure`; `Act` of Bolt/Seeker/Area/Field → `Zone`; `Meditate` or `qi < 25` → `Recover`; `out_for > 0` or
frozen/shocked → `Punish` is mine to take, the foe reads as `Defend`; `Guard` → `Defend`; else `Neutral`.

- [ ] **Step 1:** Tests: `a_windup_is_seen_after_reaction` (push senses every 1/60 s where the foe starts a melee
  windup at t=1.0; with reaction 0.2, `seen(1.1)` shows `Idle`, `seen(1.25)` shows `Act`), `a_young_threat_is_hidden`,
  `dt_zero_does_not_age` (pushing the same time twice keeps one entry), `opening_counts_recovery` (foe `Act` recover
  t=0.1 of recover 0.32 → `opening ≈ 0.22`), `a_striking_windup_is_no_opening`, `punisher_reads_foe_blade_in_reach`
  (foe ready slash-like Melee reach 24, d 20 → `punisher == damage`), `foe_casting_from_range_reads_as_zone`,
  `inputs_stay_in_unit_range` (random senses from `Rng`, every `Input` in `[0,1]`, never NaN, including arena
  narrower than `profile.range`).
- [ ] **Step 2:** Run → fail. **Step 3:** Implement. **Step 4:** Run → pass, clippy clean.
- [ ] **Step 5:** Commit `pixfx-mind: the foe seen late, and the reading`.

### Task 4: Habits and yomi

**Files:** Create `src/habits.rs`, `src/yomi.rs`, `tests/habits.rs`.

**Interfaces — Produces:**
```rust
pub enum FoeMove { Approach, Retreat, Guard, Close, Ranged, Area, Jump, Fly, Meditate, Blink, Idle }  // from Doing transitions
pub fn foe_move(before: &Foe, after: &Foe, me_x: f64) -> Option<FoeMove>   // None when nothing changed
pub struct Habits { /* counts[foe_posture][prev][next] and [prev2][prev][next], f64, decaying */ }
impl Habits { pub fn new() -> Habits; pub fn see(&mut self, posture: Posture, m: FoeMove, now: f64);
    pub fn predict(&self, posture: Posture, now: f64) -> Option<(FoeMove, f64)>;  // move and confidence in [0,1]
    pub fn clear(&mut self) }
pub struct Yomi { /* weights [f64; 3] */ }
impl Yomi { pub fn new() -> Yomi; pub fn draw(&self, rng: &mut Rng) -> u8; pub fn paid(&mut self, level: u8, payoff: f64); pub fn clear(&mut self) }
```
Counts decay with half-life 4 s of `now`; confidence = `seen / (seen + prior)` with prior 3; n=2 used when its
confidence beats n=1's. Yomi starts `[0.5, 0.35, 0.15]`, `paid` moves the drawn level's weight by `0.1 * payoff`,
floor 0.05, renormalised.

- [ ] **Step 1:** Tests: `a_repeated_pattern_is_predicted` (foe Guard after Approach five times → `predict` gives
  Guard with confidence > 0.6), `a_thin_read_barely_counts` (one sighting → confidence < 0.3), `old_habits_fade`
  (same five, then `now + 20` s → confidence < 0.2), `yomi_leans_toward_what_paid` (paid level 1 ten times with +1 →
  over 1000 draws level 1 is the most drawn), `yomi_never_drops_a_level` (paid level 2 −1 fifty times → still drawn
  sometimes).
- [ ] **Step 2:** Run → fail. **Step 3:** Implement. **Step 4:** Run → pass, clippy clean.
- [ ] **Step 5:** Commit `pixfx-mind: foe habits and yomi levels`.

### Task 5: Postures

**Files:** Create `src/posture.rs`, `tests/posture.rs`.

**Interfaces — Produces:**
```rust
pub enum Category { Approach, Retreat, Close, Ranged, Area, Defend, Evade, Buff, Meditate, Wait }  // `as usize` 0..10
pub struct PostureDef { pub posture: Posture, pub interrupt: bool, pub min_time: f64, pub considerations: &'static [Consideration], pub weights: [f64; 10] }
pub static POSTURES: [PostureDef; 8];
pub struct Held { pub posture: Posture, pub since: f64 }
pub fn choose_posture(held: &Held, now: f64, r: &Reading, profile: &Profile) -> (Posture, [f64; 8])  // the scores, for Why
```
Rules: a posture's score is `score(considerations) * profile.posture_scale[p]`. Interrupt postures (`Defend`,
`Punish`) whose score is above 0.25 get `+1.0` (the band) and win over any non-interrupt. The held posture gets a
commitment bonus `0.3 * max(0, 1 - (now - since) / min_time)`, and a different posture must beat the held score by
`0.1` (hysteresis). Starting considerations (curves are the implementer's, tuned in Task 9):
Neutral: always `0.3` floor; Pressure: Close, CloseReady, Qi, FoeRecovering or FoeCornered; Zone: RangedReady, Qi,
Inverse(Close), FoePressing; Punish: Opening, CloseReady or RangedReady; Defend: Threat; Evade: Threat, Cornered;
Recover: Inverse(Qi), Inverse(Threat); Finish: FoeHpLow or RageFull, Lead. Weights: each posture's own categories
2.0-3.0, the rest 0.2-1.0 (Defend: Defend 3, Evade 2, Close 0.6 for counters; never 0 so a choice always exists).

- [ ] **Step 1:** Tests on hand-made `Reading`s (a `Reading::neutral()` test helper with every input 0.5): `low_qi_far_foe_recovers`,
  `foe_recovering_in_reach_punishes`, `an_incoming_blow_interrupts_any_posture` (held Pressure since 0.01 s, Threat 0.9 → Defend),
  `commitment_holds_against_a_slightly_better_posture` (held Zone 0.05 s ago, Pressure scores 0.05 more → stays Zone),
  `commitment_fades` (same, 2 s later → Pressure), `finish_is_vetoed_at_full_foe_life`, `every_posture_is_reachable`
  (for each posture a reading exists that picks it).
- [ ] **Step 2:** Run → fail. **Step 3:** Implement. **Step 4:** Run → pass, clippy clean.
- [ ] **Step 5:** Commit `pixfx-mind: postures chosen by score, with commitment and interrupts`.

### Task 6: Candidates and the pick

**Files:** Create `src/candidates.rs`, `src/pick.rs`, `tests/candidates.rs`.

**Interfaces — Produces:**
```rust
pub struct Candidate { pub choice: Choice, pub category: Category, pub base: f64 }
pub fn candidates(ctx: &Ctx) -> Vec<Candidate>
pub struct Ctx<'a> { pub me: &'a Me, pub foe: &'a Foe, pub threats: &'a [Threat], pub arena: &'a Arena, pub skills: &'a [Skill],
    pub ready: &'a [bool], pub body: &'a BodySize, pub profile: &'a Profile, pub reading: &'a Reading,
    pub read: &'a Read, pub predicted: Option<(FoeMove, f64)>, pub level: u8, pub rng_roll: f64 }
pub struct Read { /* per skill: tried, landed, punished (decaying) */ }
impl Read { pub fn new(n: usize) -> Read; pub fn tried(&mut self, i: usize); pub fn landed(&mut self, i: usize); pub fn punished(&mut self, i: usize, now: f64); pub fn chance(&self, i: usize, base: f64, prior: f64) -> f64; pub fn penalty(&self, i: usize, now: f64) -> f64; pub fn clear(&mut self) }
pub fn pick(cands: &[Candidate], weights: &[f64; 10], rng: &mut Rng) -> Option<(usize, Vec<(usize, f64)>)>  // winner, ranked scores
```
Generators port `dangers`, `blows`, `attacks`, `positions` of `site/assets/duel.js:863-1127` with kinds in place of
names (`"slash"` → the cheapest ready `Melee` that is not a dive; `"bolt"` → a ready `Bolt`; `"blink"` → a ready
`Blink`; the barrier rule of `mind()` at `duel.js:810-813` → a ready `Barrier` with threat urgency > 10). Each
candidate gets its `Category`. Yomi level 1 adds `+30%` base to candidates that answer `predicted` (Guard predicted →
Area and Blast up, Melee down; Approach predicted → Melee up; Ranged predicted → Close/Dash up, Guard on); level 2
does the same against the foe's predicted answer to my most-tried skill. `Read::penalty` multiplies a skill's base by
`1 - 0.5 * punished-recency` (decays over 6 s). `pick`: score = base × category weight; keep scores ≥ 70% of the
best; draw by weight; with probability 0.08 take the second best if it is ≥ 50% of the best.

- [ ] **Step 1:** Tests: `only_ready_skills_are_cast` (every skill unready → no `Cast` in candidates; `pick` still
  returns a move/guard/wait), `a_bolt_at_chest_height_offers_guard_duck_jump`, `standing_in_an_area_offers_to_leave`,
  `a_foe_blade_winding_up_in_reach_offers_step_back_and_guard`, `no_ranged_skills_still_fights` (only Melee + Blink),
  `names_are_never_read` (a skill set named `"a".."k"` gives the same candidates, by kind, as the duel-named set),
  `a_punished_skill_scores_lower`, `hundred_threats_no_nan` and `no_threats_no_nan`, `pick_is_seeded` (same rng seed →
  same winner over 100 calls).
- [ ] **Step 2:** Run → fail. **Step 3:** Implement. **Step 4:** Run → pass, clippy clean.
- [ ] **Step 5:** Commit `pixfx-mind: candidate actions per skill kind, ranked and weighted`.

### Task 7: The Mind

**Files:** Create `src/mind.rs`, `tests/mind.rs`; modify `src/lib.rs`.

**Interfaces — Produces:**
```rust
pub struct Mind { /* profile, body, skills, rng, clock, calm, held, delayed, habits, yomi, read, last level, explain */ }
impl Mind {
    pub fn new(profile: Profile, body: BodySize, skills: Vec<Skill>, seed: u32) -> Mind;
    pub fn reset(&mut self);                 // new round: clears everything but profile/body/skills; rng continues
    pub fn hold(&mut self, seconds: f64);    // no thought for that long
    pub fn explain(&mut self, on: bool);     // fill Thought::why
    pub fn think(&mut self, sense: &Sense, dt: f64) -> Option<Thought>;
    pub fn tell(&mut self, outcome: &Outcome);
    pub fn posture(&self) -> Posture;
}
```
`think`: always pushes the sense into `Delayed` and feeds `Habits` from the delayed foe; counts down the thought
clock and `calm` by `dt`; returns `None` when `!sense.me.can_act` or the clock is not due; else the clock restarts at
`profile.think * (0.8 + 0.4 * rng)`, it builds the `Reading`, chooses the posture, draws the yomi level once per
posture entry, builds candidates and picks. A `Cast` of a skill with damage sets `calm = tempo * (1 - spread/2 +
spread * rng)` and counts `tried`. Yomi payoff: `+1` on `Landed`, `-1` on `Punished`, for the level in force.
First thought of a round waits `0.4 + 0.3 * rng` s (today's `startRound`).

- [ ] **Step 1:** Tests: `same_seed_same_thoughts` (two minds, 600 identical senses → identical thoughts),
  `no_thought_while_it_cannot_act`, `hit_stop_holds_the_clock` (1000 ticks of `dt = 0` → `None` each, then the next
  thought comes when it would have), `hold_keeps_it_quiet`, `reset_forgets_the_round` (after habits, calm and a
  `Pressure` posture, `reset()` → posture `Neutral`, `predict` None, first thought delayed again),
  `scenario_punishes_a_whiff` (foe recovering 0.3 s in reach, my Melee ready → `Cast` of it within two thoughts, posture
  `Punish`), `scenario_meditates_when_dry` (qi 10, foe 150 px away idle → `Meditate`, posture `Recover`),
  `scenario_leaves_a_falling_area` (an `Area` on me landing in 0.4 s, aged past reaction → a `Move` away, `Blink` or
  `Fly`, posture `Evade` or `Defend`), `explain_fills_why`.
- [ ] **Step 2:** Run → fail. **Step 3:** Implement. **Step 4:** `cargo test -p pixfx-mind --all-features`, clippy clean.
- [ ] **Step 5:** Commit `pixfx-mind: the mind thinks, remembers the round and learns what lands`.

### Task 8: `Mind` for JS

**Files:** Create `crates/pixfx-wasm/src/mind.rs`; modify `crates/pixfx-wasm/Cargo.toml` (dep `pixfx-mind` with
`ts`, optional; feature `mind = ["dep:pixfx-mind"]` in `default`), `crates/pixfx-wasm/src/lib.rs`,
`crates/pixfx-wasm/AGENTS.md` (Job paragraph names `Mind`), `packages/pixfx-canvas2d/src/index.ts` (re-export `Mind`),
`site/tests/engine.js` (export `Mind`); test `crates/pixfx-wasm/tests/mind.rs` if the crate tests JS-facing parses
natively, else assert through `site/tests/mind.test.js`.

**Interfaces — Produces (JS):**
```ts
new Mind(profile: Profile, body: BodySize, skills: Skill[], seed: number)
mind.think(sense: Sense, dt: number): Thought | null
mind.tell(outcome: Outcome): void
mind.reset(): void; mind.hold(seconds: number): void; mind.explain(on: boolean): void
mind.posture: Posture   // getter, "neutral" | "pressure" | ...
```
Every object is read with `crate::js::parse` (strict, naming the field), like `Body.step` in `src/fighter.rs`.

- [ ] **Step 1:** Write `site/tests/mind.test.js`: `a misspelt sense field throws naming it` (`think({...sense, me: {...me, canAkt: true}})`
  throws matching `/canAct|canAkt/`), `a mind thinks from a sense` (a valid sense after 1 s of ticks returns a `Thought`
  whose `choice.type` is one of the eight), `posture reads as a string`.
- [ ] **Step 2:** Run `bun test site/tests/mind.test.js` → fails (`Mind` undefined).
- [ ] **Step 3:** Implement the binding; `(cd packages/pixfx-web && bun run build:wasm)`.
- [ ] **Step 4:** `cargo test -p pixfx-wasm`, `cargo clippy -p pixfx-wasm --all-targets -- -D warnings`,
  `(cd packages/pixfx-web && bun run check)`, `bun test site/tests/mind.test.js` pass.
- [ ] **Step 5:** Commit `wasm: Mind, the fighter's brain, for JS`.

### Task 9: The duel thinks with the Rust brain, and the match proves it

**Files:** Modify `site/assets/duel.js`, `site/assets/landing.js`, `site/tests/duel.test.js` (the `standoff` helper);
create `site/tests/mind-baseline.js`, `site/tests/mind-match.test.js`.

**Interfaces:**
- `createDuel({ ..., Body, Mind, minds })`: `minds` optional `[m0, m1]`, each
  `{ think(duel, me, foe, dt) -> Choice | null, tell(outcome), reset() }`. Without it, duel.js wraps a `new Mind(...)`
  per fighter in that shape (`rustMind(duel, f, Mind)`), with `skills` from `SPELLS` (kind map: melee+dive → `dive`,
  orb → `seeker`, zone → `area`, void → `field`, the rest same name), `profile` from `SCHOOLS` + today's `MIND`,
  `PRIOR`, `REACTION`, `THINK`, `body` from `f.spec`, seed `duel.seed * 2 + f.index`.
- `senseOf(duel, me, foe) -> Sense`: true state; threats from `duel.shots` of the foe with `age` (bolt: `s.t`; zone:
  `s.t + lead - notice`; orb/void: `s.t`; barrage strikes: `Barrage{landsIn}`); `ready` from today's `ready()`.
- `apply(duel, me, choice)`: today's tail of `mind()` (`duel.js:816-833`).
- `tell`: `Landed` where `by.read.landed` is counted today (`duel.js:542`); `Punished` in `hurt` when the struck
  fighter's `act.phase === "recover"`.
- `site/tests/mind-baseline.js`: `export function baselineMind()` returning that shape, holding today's
  `mind`/`dangers`/`blows`/`attacks`/`positions`/`punisher`/`ready` and `MIND`, `PRIOR` unchanged except reading
  `duel`, `me`, `foe` passed in and returning a `Choice` instead of applying it. It draws from its own
  mulberry32 seeded like the Rust mind.
- `standoff` in `duel.test.js`: `f.mind.hold(1e9)` replaces `f.think = 1e9`.
- landing.js: passes `Mind`; with `?mind` in `location.search`, calls `explain(true)` and draws each fighter's
  posture as a caption above its bar.

- [ ] **Step 1:** Write `mind-match.test.js`: for seeds 1..20, two duels each (brain as Thunder, then as Frost),
  run until 3 rounds end or 300 s; count rounds won by the brain; `console.log` win rate, mean round seconds and
  posture entry counts; `expect(winRate).toBeGreaterThanOrEqual(0.6)`; also `expect(meanRound).toBeLessThan(60)`.
- [ ] **Step 2:** Run → fails (no `minds` option).
- [ ] **Step 3:** Move the old mind to `mind-baseline.js`, delete it from `duel.js` (with `MIND`, `PRIOR`, the
  `read`, `think`, `calm` fields of the fighter), add `senseOf`, `apply`, `rustMind`, the `minds` option, the tells,
  the landing caption.
- [ ] **Step 4:** `bun test site/tests` all pass except possibly the match bar. Record the match output.
- [ ] **Step 5:** Tune posture considerations, weights and yomi numbers in `crates/pixfx-mind/src/posture.rs` /
  `candidates.rs` (never the baseline, never the bar), rebuild wasm, rerun the match, until ≥ 60%. Each tuning pass
  keeps `cargo test -p pixfx-mind` green. If ten passes do not reach 60%, stop and report the numbers.
- [ ] **Step 6:** Commit `site: the duel's fighters think with pixfx-mind; a match against the old mind proves it`.

### Task 10: On the page, and the docs

**Files:** Modify `README.md` (crate list), `AGENTS.md` (Engine row: add `pixfx-mind`, "a fighter's brain"),
`CHANGELOG.md`, `.claude/skills/pixfx/SKILL.md` (Engine row).

- [ ] **Step 1:** `site/build.sh --serve` in the background; open `http://127.0.0.1:8770/?mind` in the browser;
  watch three rounds; screenshot one moment per posture seen. Pass: each fighter enters more than three postures in a
  round, rounds end by knockout, no console errors.
- [ ] **Step 2:** Update the docs (one row each, in the existing voice).
- [ ] **Step 3:** Commit `docs: pixfx-mind`.
