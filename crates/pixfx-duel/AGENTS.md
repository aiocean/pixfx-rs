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

- `tests/goldens/` pins what the rules produce: `event_hashes.json` (every command and event of 120 s of seeds
  1-20), `first_mind.json` (the first mind's first 600 choices of seeds 1-5) and `arenas.json` (each seed's pools and
  opening bars). A change that alters fights fails them; when the change is meant, delete the files it fails and run
  the tests once to rewrite them.
- `tests/fixtures/js_numbers.json` pins the tables, dice and formulas (`tests/data.rs`), one key per number: a
  deliberate change to a number edits its key there too.
- One extra or missing roll of the dice changes every fight after it.
- JS maths: `Math.sign(0)` is 0 (never `f64::signum`), `Math.round` rounds half up (`js_round`), `**` is `powi`.

## Before you finish

```
cargo test -p pixfx-duel --all-targets
cargo clippy -p pixfx-duel --all-targets -- -D warnings
```
