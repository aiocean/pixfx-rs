# pixfx-duel

Layer: **Game**. Read the root `AGENTS.md` ("The boundary" and "Layers and where code goes") before changing anything
here; this file only says what this crate holds. The design is
`docs/superpowers/specs/2026-11-15-duel-rules-in-rust-design.md`.

## Job

The landing duel's rules: two cultivators, Thunder and Frost, their stats and skills (`data`), the seeded dice every
roll comes from (`rng`), the fighters (`fighter`), statuses (`status`), the rotating bar (`bar`), casts and blows
(`act`), shots (`shot`), the round loop (`duel`), the seats that decide for each corner (`seat`: the brain from
`pixfx-mind`, or `first_mind`, the yardstick a brain is measured against), what a mind senses (`sense`), and the duel
as the page reads it (`view`). A seed replays the same fight on every target. The page (`site/assets/duel.js`, a
wrapper over the wasm `Duel`) plays the effect commands and feels the events the duel returns; it decides nothing.

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

- Required: `libm`, `pixfx-draw` (JS-exact maths), `pixfx-fighter` (bodies), `pixfx-mind` (the brain).
- Optional, behind `serde` / `ts`: `serde`, `tsify`, `wasm-bindgen`.
- Never `pixfx-scene`, `pixfx-wasm` or a shell.

## Traps

- `tests/goldens/event_hashes.json` pins every command and event of 120 s of seeds 1-20: a change that alters
  fights fails it. When the change is meant, delete the file and run the test once to rewrite it.
- `tests/fixtures/js_numbers.json` holds the tables, dice and first-mind choices the rules were ported against.
  A deliberate change to a number updates the fixture's value with it.
- One extra or missing roll of the dice changes every fight after it.
- JS maths: `Math.sign(0)` is 0 (never `f64::signum`), `Math.round` rounds half up (`js_round`), `**` is `powi`.

## Before you finish

```
cargo test -p pixfx-duel --all-targets
cargo clippy -p pixfx-duel --all-targets -- -D warnings
```
