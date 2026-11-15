# pixfx-duel

Layer: **Game**. Read the root `AGENTS.md` ("The boundary" and "Layers and where code goes") before changing anything
here; this file only says what this crate holds. The design is
`docs/superpowers/specs/2026-11-15-duel-rules-in-rust-design.md`.

## Job

The landing duel's rules, ported from `site/assets/duel.js` with the same numbers and the same dice: two cultivators,
Thunder and Frost, their stats and skills (`data`), the seeded dice every roll comes from (`rng`), and, as the port
lands, the fighters, statuses, the rotating bar, casts and shots, and the round loop. A seed replays the same fight
on every target. The page plays the effect commands and feels the events the duel returns; it decides nothing.

## Belongs here

- The duel's rules: damage, defences, cooldowns, qi, rage, statuses, wards, barriers, the bar, rounds.
- The duel's tables: spells, schools, stats, tiers, traits, luck.

## Does not belong here

| If the code... | It goes to |
|---|---|
| decides what a fighter does | `pixfx-mind` (or a trained brain) |
| moves a body or poses it | `pixfx-fighter` |
| draws, shakes, plays an effect | the page, from the commands and events the duel returns |
| is a JS binding | `pixfx-wasm` (`duel` feature) |

## Dependencies

- Required: `libm`, `pixfx-draw` (JS-exact maths).
- Optional, behind `serde` / `ts`: `serde`, `tsify`, `wasm-bindgen`.
- Never `pixfx-scene`, `pixfx-wasm` or a shell.

## Traps

- Parity with `duel.js` is the gate while the port lands: `tests/data.rs` checks every table and formula against
  `tests/fixtures/js_numbers.json`, written by `site/tests/tools/dump-numbers.js`. Change a number in both, then
  re-run the dump.
- Dice are drawn in `duel.js`'s order. One extra or missing roll changes every fight after it.
- JS maths: `Math.sign(0)` is 0 (never `f64::signum`), `Math.round` rounds half up (`js_round`), `**` is `powi`.

## Before you finish

```
cargo test -p pixfx-duel --all-targets
cargo clippy -p pixfx-duel --all-targets -- -D warnings
```
