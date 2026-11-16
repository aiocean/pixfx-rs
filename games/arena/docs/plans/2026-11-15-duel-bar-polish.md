# The duel's bar: readable, fair, more chance — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make the rotating skill bar readable on screen, the two schools even, and each fight more varied (random pool, rarity, traits, lucky casts), and fix the four deferred review findings.

**Architecture:** Rules in `site/assets/duel.js` (bar slots gain `rarity` and `traits`; casts read them), drawing in `site/assets/duel-art.js` `drawSkills`, and one field added to pixfx-mind's `Sense` (`power`) so the AI values a bar skill at its real strength.

**Tech Stack:** JavaScript (bun test), Rust (pixfx-mind, pixfx-wasm), the site's static build.

**Spec:** `games/arena/docs/specs/2026-11-15-duel-bar-polish-design.md` (builds on `2026-11-15-duel-rotating-skills-design.md`).

## Global Constraints

- All randomness from `duel.rng` (seeded); same seed, same fight.
- Every number is a named constant at the top of `duel.js`.
- Shared tree: commit only own files/hunks through a temp index; `duel-art.js`, `landing.js`, `CHANGELOG.md`, `crates/pixfx-wasm/*` carry other sessions' edits.
- Rust: clippy `unwrap_used`/`expect_used`/`panic` denied, no `unsafe`. `export CARGO_BUILD_BUILD_DIR=/tmp/pixfx-build`.
- Tests: `bun test site/tests`; `cargo test -p pixfx-mind --all-targets` and clippy for the Rust change.

## Review Focus

1. A trait that cannot apply (Heavy on a heal, Lingering on a skill with no stacks) is never rolled; an epic with only one fitting trait gets one.
2. Cheap halves the cost that `ready` and `begin` both use; the AI is never told a skill is ready that `begin` cannot pay.
3. A lucky Again on the ultimate or a basic never touches the bar; lucky casts roll only for bar skills.
4. A pool of 8 that lacks an evolution partner just never evolves; the signature draw still finds its kind.
5. `Sense.power` shorter than the skills (or absent, from older callers) reads as 1.

---

### Task 1: The four findings

**Files:** `site/assets/duel.js`; tests in `site/tests/status.test.js`, `site/tests/guard.test.js`, `site/tests/bar.test.js`.

- [ ] Tests (RED): an orb that lands adds a shock stack; `SCHOOLS.frost.pool` has no `bolt` and `EVOLVES.frost` pairs shard–icicle (assert through `spend`: a spent shard with icicle on the bar evolves); a guarded blow's chip on a high-`pdef` target is smaller than on a low one.
- [ ] Implement: `orb: shock: true` (its hit path calls `addShock` with the shot's stacks); Frost's pool without `bolt`, pair `["shard","icicle"]`; chip = `damageOf(by, f, damage, type, false, 0.5) * CHIP` in `blocks`.
- [ ] `bun test site/tests` PASS; commit `duel: orb shocks, Frost's own bolt, chip follows defence`.

### Task 2: A random pool per fight

**Interfaces:** Produces `POOL_SIZE = 8`; `f.pool` is the fight's 8 (rolled once in `createDuel`), `f.spells` the basics + those 8 + ultimate.

- [ ] Tests (RED): over seeds 1–30 each fighter's pool has 8 distinct spells from its school, at least one of its signature kind and one payoff (`PAYOFF` key); the same seed gives the same pool; different seeds give at least 5 different pools.
- [ ] Implement in `fighter()`/`createDuel`: shuffle with `duel.rng`, keep the first signature-kind and first payoff spell, fill to 8.
- [ ] Widen "every pool spell is cast" to seeds 1–40 if needed (each spell now plays in fewer fights). PASS; commit `duel: each fight plays a random 8 of each school's pool`.

### Task 3: Rarity and traits

**Interfaces:** Produces slot fields `rarity: "common"|"rare"|"epic"`, `traits: string[]`; `skillBar` passes both; exported `RARITY`, `TRAITS`, `fits(trait, spell)`, `costOf(f, name)`, `cooldownOf(f, name)`.

- [ ] Tests (RED): over 3000 draws rarity odds within ±3% of 70/22/8; power = casts power × rarity factor; traits only fitting (`fits`), common ≤1, rare 1, epic 2 distinct when two fit; Swift sets the cooldown to ×0.6; Cheap: `ready` true at half the cost and `begin` takes half; Heavy hits ×1.3; Piercing goes through a guard (no block roll); Lingering adds one stack (2 with evolution → 3).
- [ ] Implement: roll in `draw`; evolution keeps rarity and traits; `begin` reads cost/cooldown/power/stacks/pierce from the slot; `act.pierce` skips `blocks` in the strike and shot paths.
- [ ] PASS; commit `duel: rarity and traits on every draw`.

### Task 4: Lucky casts

**Interfaces:** Produces `LUCKY = { chance: 0.08, kinds: ["double","free","again"] }`; event `{ type: "lucky", by, kind, x, y }`.

- [ ] Tests (RED): over many bar casts the lucky rate is 8% ± 2%; Double doubles the cast's power; Free gives the qi back; Again clears its cooldown; a basic or ultimate is never lucky.
- [ ] Implement in `begin`; PASS; commit `duel: lucky casts`.

### Task 5: The AI reads real power

**Files:** `crates/pixfx-mind/src/sense.rs`, `mind.rs`, `view.rs`, `tests/`; `crates/pixfx-wasm/src/mind.rs` only if its types need it (hunk only); `site/assets/duel.js` `senseOf`.

- [ ] Rust test (RED): with two identical bolt skills, power `[1, 2]`, the mind picks the second far more often over many thinks; an absent `power` behaves as all 1.
- [ ] Implement: `Sense.power: Vec<f64>` with `#[serde(default)]`; `think` scales each skill's `damage` by `power.get(i)` (default 1) before `Reading` and `Ctx`. `cargo test -p pixfx-mind --all-targets`, clippy; `bun run build:wasm` in `packages/pixfx-web`.
- [ ] JS: `senseOf` passes `power` for `me.spells` (bar slot's power × traits, 1 otherwise). `bun test site/tests` PASS; commit `mind: a skill's damage scales with its power on the bar`.

### Task 6: The bar on screen

**Files:** `site/assets/duel-art.js` (`drawSkills`, hunk only), `site/assets/landing.js` only for the lucky word if numbers are drawn there (hunk only).

- [ ] Casts digit on the tab (slot numbers removed); unready 0.6; empty slot: seconds digit + frame lit clockwise; ultimate: clean icon, frame fills with rage from the bottom; rarity frame colours; 5×5 trait glyphs; lucky flash and word.
- [ ] Build (`site/build.sh`, or rsync if the wasm is fresh), browser at 3× zoom: each of those visible, no console errors. Commit `duel: a bar you can read`.

### Task 7: Fair fights

- [ ] Test (RED first if it fails): over seeds 1–40 × 120 s, fights end in knockouts and each school wins ≥35% of rounds; print the split.
- [ ] Tune `STATS`, costs and damages until each school wins 40–60%. Record each change and the split it gave in the ledger.
- [ ] PASS; browser check of three rounds; CHANGELOG line (hunk only); commit `duel: even fights; changelog`.

### Final

Fresh review on opus with this plan's Review Focus and the ledger's rulings; one fix pass; deploy to `site/dist`.
