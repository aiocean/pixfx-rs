# pixfx-effects

Layer: **Engine**. Read the root `AGENTS.md` ("The boundary" and "Layers and where code goes") before changing
anything here; this file only says what this crate holds.

## Job

The effects, as ports of pixelfx (https://github.com/patch-gg/pixelfx). Each lives in its category folder
(`src/fire/candle.rs`), is listed in `EFFECTS` and is looked up with `pixfx_effects::effect(id)`.

## Belongs here

An effect: what it draws on each frame, its options and their defaults, its meta, and the cues it fires
(through `pixfx_core::cue::impact`).

## Adding an effect

1. `src/<category>/<name>.rs`, with `mod` and `pub use` in `src/<category>/mod.rs`, each under
   `#[cfg(feature = "<category>-<name>")]`.
2. In `Cargo.toml`: the feature `<category>-<name> = []` (the id with `/` as `-`), listed in its category feature
   and the category listed in `all`.
3. Its `EFFECTS` entry in `src/lib.rs` under the same `#[cfg]`.
4. `tests/features.rs` fails if any of these is missing. A golden hash in `tests/golden*` pins its pixels.

## Does not belong here

| If the code... | It goes to |
|---|---|
| is shared by several effects as a drawing primitive | `pixfx-draw` |
| is shared by several effects as particles, physics, cues or feel | `pixfx-core` |
| decides where an effect plays, follows a sprite, fades after stop or picks the frame to show | `pixfx-scene` |
| reads a clock, a file or a host | a shell |

An effect never knows which shell shows it.

## Dependencies

`pixfx-core`, `pixfx-draw`, `libm`. Nothing else; never a host crate.

## Traps

- Parity: every effect in `EFFECTS` must pass `pixfx parity` against pixelfx. A port keeps pixelfx's JS number
  semantics: same order of operations, `js_round`, `libm`. Do not tidy maths.
- Determinism: same options and time give the same `frame_hash` on every target. No `HashMap` iteration order, no
  unseeded randomness.
- A `frame_hash` change to an existing effect needs a reason in the commit message; "the golden was updated" is not
  a reason.

## Before you finish

```
cargo test -p pixfx-effects
cargo test -p pixfx-effects --test determinism -- --ignored
wasm-pack test --node crates/pixfx-effects --test determinism -- --include-ignored
cargo run -p pixfx-cli -- parity <id> --pixelfx /tmp/pixelfx --out target/parity
```
