# Duel rotating skill bar Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** The landing duel's fighters fight with a rotating bar of 3 random skills, each with its own energy and power, over stacking statuses, anti-lock control, size-aware guard, ward charges, evolution and cleanse.

**Architecture:** All rules stay in `site/assets/duel.js` (the Rust port comes later). Pure rule helpers are exported for tests; the bar lives on each fighter as `f.bar`; `skillBar(f)` is what the screen and the brain read. The brain is handed its school's whole pool and sees a spell off the bar as not ready.

**Tech Stack:** JavaScript (ES modules), bun test, the pixfx wasm engine (`site/tests/engine.js`), ImageMagick + the pixel snapper for icons, the codex-image skill for the icon sheet.

**Spec:** `docs/superpowers/specs/2026-11-15-duel-rotating-skills-design.md` (research: `docs/superpowers/research/2026-11-15-survivors-combat-research.md`)

## Global Constraints

- Every number is a named constant at the top of `duel.js`, with the spec's values: casts 1–4 equal odds; power ×1.4 / ×1.2 / ×1.1 / ×1.0; empty countdown `3 + 2 × roll` s; synergy draw 1 in 3; chill max 3, 2.5 s, walk ×0.85 per stack; shock max 5, interval `0.6 × 0.854^(stacks−1)` s, 3% attack, hold 0.12 s; payoffs +12% per shock stack (palm, burst), +15% per chill stack (shatter, lotus); grit +1 per hard control, −0.4/s, duration `/ (1 + grit)`, freeze → chill at grit ≥ 2; `GUARD = 20`, chance `min(1, min(0.5r, 0.5√r))`; ward charges `1 + floor(3 × roll)`, 6 s; `SPREAD = 0.03`; chip 0.2; evolution power ×1.5, +1 stack.
- Every roll comes from `duel.rng()` (seeded): the same seed plays the same fight.
- Slash, dive, blink and the ultimate are always at hand; at most 3 slot skills.
- A skill is known to the brain by kind; `pixfx-mind` gets no change. A spell whose effect is not in the registry is left out of the pool (as today).
- Shared tree: commit only this plan's files and hunks through a temporary index (`.superpowers/sdd/2026-11-15-pixfx-mind/commit.sh` pattern); `landing.js` and `duel-art.js` may hold other sessions' edits.
- Tests: `bun test site/tests`; whole-fight simulations carry an explicit timeout sized to their fighting time.

## Review Focus

1. A pool smaller than 3 usable spells (effects missing from the registry): the bar draws what exists and leaves the rest empty, never loops forever.
2. A cast that empties a slot while the spell is mid-windup: the spell still lands with the power it was cast at.
3. A round ending mid-countdown: the next round starts with 3 fresh draws, not the old countdown.
4. Grit, stacks and charges reset each round.
5. `skillBar(f)` before the first round and after a knockout: no throw, no NaN.

---

### Task 1: The bar

**Files:**
- Modify: `site/assets/duel.js` (constants; `SCHOOLS.*.pool` and `.ultimate` replace `.slots`; `fighter`, `startRound`, `body`, `ready`, `begin`, `skillBar`, the brain's kit in `brain`)
- Modify: `site/assets/duel-art.js` (`drawSkills` reads the new `skillBar`)
- Test: `site/tests/bar.test.js`

**Interfaces:**
- Produces: `export function castsPower(casts: 1|2|3|4): number`; `export function draw(duel, f, i): void` (fills slot `i`); `export function spend(duel, f, i): void` (one cast of slot `i`; at 0 casts the slot empties); `f.bar: Array<{ spell, casts, left, power, evolved: false } | { empty: number }>` of length 3; `export function skillBar(f): Array<{ spell: string|null, empty: boolean, ultimate: boolean, ready: boolean, left: number, full: number, charge: number, casts: number, power: number, evolved: boolean }>` (3 slots then the ultimate); `powerOf(f, name): number` (internal: `SPELLS[name].damage × slot power`, used at every strike site in place of `spell.damage`).

- [ ] **Step 1: Write the failing tests** in `site/tests/bar.test.js`:
  - `castsPower`: `[1,2,3,4].map(castsPower)` equals `[1.4, 1.2, 1.1, 1.0]`.
  - `a fresh round draws 3 different pool skills`: for seeds 1–10, after `createDuel` each fighter's `bar` has 3 slots, none empty, spells distinct, each in `SCHOOLS[school].pool`, `casts` in 1..4.
  - `a spent skill empties its slot and refills with a skill not on the bar`: drive a fighter's cast through `step` (or call the exported `spend(duel, f, i)`) until `left` hits 0; slot is `{ empty: t }` with `3 ≤ t ≤ 5`; step `t` seconds; slot holds a spell not on the other two.
  - `basics and the ultimate stay`: `f.spells` always includes slash, dive, blink and the school's ultimate.
  - `the same seed plays the same bar`: two duels with seed 7 stepped 60 s give equal `bar` sequences.
  - `skillBar before the first round and after a knockout`: no NaN in any field.
- [ ] **Step 2: Run** `bun test site/tests/bar.test.js` — Expected: FAIL (`castsPower` not exported).
- [ ] **Step 3: Implement.** Pools as listed in the spec (Task 5 adds the new spells; until then the pools are today's slot spells). `f.spells` is the whole usable pool + basics + ultimate (the brain's kit, fixed for the fight); `ready(me, name)` adds "on the bar with casts left" for pool spells; `begin` spends a cast and empties the slot at 0 (the act keeps `power` it was cast with); each slot spell runs its own cooldown; `body` counts empty slots down and calls `draw`; `startRound` draws 3 fresh. `drawSkills` keeps the slot-position icons for now (Task 6 swaps them).
- [ ] **Step 4: Run** `bun test site/tests` — Expected: PASS (duel.test.js still green).
- [ ] **Step 5: Commit** `duel: a rotating bar of three skills, each with its own casts and power`.

### Task 2: Stacking statuses and payoffs

**Files:** Modify `site/assets/duel.js` (`chill`/`shock` become stacks; `hurt`, `shock()`, the frostfire tick, `body`); Test `site/tests/status.test.js`

**Interfaces:**
- Produces: `f.chill: number` (0..3, with `f.chillT`), `f.shock: { stacks, tick, by }`; `export function addChill(duel, f, n)`, `export function addShock(duel, f, by, n)`; `export function shockInterval(stacks): number`; payoff in `powerOf`: palm/burst `× (1 + 0.12 × shock)`, shatter/lotus `× (1 + 0.15 × chill)`, then the stacks are spent.

- [ ] **Step 1: Failing tests:** three `addChill` freeze the target (`f.frozen > 0`) and clear stacks; chill stacks expire 2.5 s after the last; `shockInterval(1) = 0.6`, `shockInterval(5) ≈ 0.6 × 0.854^4`; 3 shock stacks tick out in order, each dealing 3% of the caster's attack and holding 0.12 s; palm on a foe with 2 shock stacks deals 1.24× and leaves 0; shock hits add a stack without a dice roll.
- [ ] **Step 2: Run** `bun test site/tests/status.test.js` — Expected: FAIL.
- [ ] **Step 3: Implement**; the body's `chilled` drive reads `chill > 0`; walking slows ×0.85 per stack where the drive applies it (`pixfx-fighter` takes `chilled` as a flag today: pass the flag, scale `intent.move` by `0.85^chill` in duel.js).
- [ ] **Step 4: Run** `bun test site/tests` — Expected: PASS.
- [ ] **Step 5: Commit** `duel: chill and shock stack, and palm, burst, shatter and lotus spend them`.

### Task 3: Grit (control gets harder to repeat)

**Files:** Modify `site/assets/duel.js` (every write to `frozen`, the shock hold, the bell's stun goes through `control`); Test `site/tests/status.test.js`

**Interfaces:** Produces `export function control(f, kind: "freeze"|"hold"|"stun", seconds): number` (applied duration) and `f.grit`.

- [ ] **Step 1: Failing tests:** a first 0.9 s freeze lasts 0.9 s; a second at once lasts `0.9 / 2`; at grit 2 a freeze adds one chill stack and no freeze; grit falls 0.4/s; grit resets each round.
- [ ] **Step 2: Run** — Expected: FAIL. **Step 3: Implement.** **Step 4: Run** `bun test site/tests` — PASS. **Step 5: Commit** `duel: grit, so freezes and stuns get shorter when repeated`.

### Task 4: Guard by hit size, ward charges, less variance

**Files:** Modify `site/assets/duel.js` (`blocks`, the ward in `strike`, `take`, `damageOf`, constants); Test `site/tests/guard.test.js`

**Interfaces:** Produces `export function blockChance(power): number`; `f.charges` (ward/hex) replacing `f.shield`'s share; `SPREAD = 0.03`, `CHIP = 0.2`.

- [ ] **Step 1: Failing tests:** `blockChance(5) = 1`; `blockChance(11) ≈ 0.674`; `blockChance(24) ≈ 0.416`; a guarded failed block deals full damage and still costs `GUARD_COST`; a ward with 2 charges erases two blows whole and the third lands; burn ticks pass a ward; `damageOf` over 1000 rolls stays within ±1.5% of its mean.
- [ ] **Step 2: Run** — FAIL. **Step 3: Implement** (the brain's `Me.shield` reads `charges > 0`). **Step 4: Run** `bun test site/tests` — PASS. **Step 5: Commit** `duel: guard by hit size, ward charges, ±1.5% spread`.

### Task 5: New spells and draw rules

**Files:** Modify `site/assets/duel.js` (`SPELLS` + spark, burst, quake, hex, shard, jet, mist; pools; `draw`); Test `site/tests/bar.test.js`

**Interfaces:** Produces the spells in the spec's table; `SCHOOLS.*.signature` (Thunder `"blast"`, Frost `"zone"`); `PAYS` map status → spells that pay it off.

- [ ] **Step 1: Failing tests:** every spell in both pools is cast in some fight of seeds 1–20 (whole fights, explicit timeout) and its `fx` is in `effects()`; the first draw of each round is of the school's signature kind; with a status on the foe, over 300 draws about a third (0.25–0.42) are payoff spells; never a duplicate on the bar.
- [ ] **Step 2: Run** — FAIL. **Step 3: Implement.** Each new spell plays like its kind (no new kind); read each effect's timing (impact, life) from its source in `crates/pixfx-effects` the way the existing comments do (e.g. icicle's impact), and note it in a comment. **Step 4: Run** `bun test site/tests` — PASS. **Step 5: Commit** `duel: seven new spells, signature first draws, payoff draws`.

### Task 6: Icons by spell and the bar on screen

**Files:** Modify `site/art/skill-icons.sh` (a grid of N cells by spell name), `site/art/skill-icons.prompt.txt`, `site/assets/duel-art.js` (`drawSkills`: icon by spell, a casts strip under each icon, empty slot shows its countdown sweep, evolved gets a bright frame); Create `site/assets/skills-index.json` (spell → cell); regenerate `site/assets/skills-32.png`, `skills-24.png`

- [ ] **Step 1:** Generate one sheet with an icon per spell (the 23 pool spells + 2 ultimates) with the codex-image skill, in the existing icons' style (`site/assets/skills-32.png` as the style reference); run `site/art/skill-icons.sh` to make both atlases.
- [ ] **Step 2: Test** in `site/tests/bar.test.js`: every spell in either pool or an ultimate has a cell in `skills-index.json`.
- [ ] **Step 3:** `drawSkills` draws by `skillBar(f)[i].spell`.
- [ ] **Step 4:** `site/build.sh`, open `http://127.0.0.1:8770/` in the browser (agent-browser-cloak), screenshot: three icons per side, casts strip, an empty slot's countdown; no console errors.
- [ ] **Step 5: Commit** `duel: an icon for every spell, casts and countdowns on the bar`.

### Task 7: Evolution and cleanse

**Files:** Modify `site/assets/duel.js` (`EVOLVES` pairs; `spend`/`draw`; heal, hex, ward cleanse); Test `site/tests/bar.test.js`, `site/tests/status.test.js`

- [ ] **Step 1: Failing tests:** a spent skill whose partner is on the bar refills at once as evolved (power ×1.5 × its casts power, `evolved: true`, inflicts one more stack); without the partner it counts down as usual; heal, hex and ward clear burn, silence, defDown, chill and shock.
- [ ] **Step 2: Run** — FAIL. **Step 3: Implement.** **Step 4: Run** `bun test site/tests` — PASS. **Step 5: Commit** `duel: evolution on the bar, and cleanse`.

### Task 8: Fights still work, and the docs

**Files:** Test `site/tests/duel.test.js` (both schools win rounds over seeds 1–20); Modify `CHANGELOG.md` (one line, hunk only)

- [ ] **Step 1:** Test: over seeds 1–20, every fight ends in knockouts and each school wins at least 15% of rounds.
- [ ] **Step 2:** `bun test site/tests` — PASS; `bun test --todo site/tests/mind-match.test.js` — report the brain's rate (no bar).
- [ ] **Step 3:** Browser check of three rounds: bars turn over, no console errors.
- [ ] **Step 4: Commit** `duel: rotating bar balance check; changelog`.
