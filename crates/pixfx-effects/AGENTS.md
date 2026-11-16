# pixfx-effects

Layer: **Engine**. Read the root `AGENTS.md` ("The boundary" and "Layers and where code goes") before changing
anything here; this file only says what this crate holds.

## Job

The effects, as ports of pixelfx (https://github.com/patch-gg/pixelfx), or of Sprite Fusion's pixel art VFX
(https://www.spritefusion.com/pixel-art-vfx) where pixelfx has no such effect, or made here when neither has it
(`feedback/outline`). Each lives in its category folder
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

- Parity: every port in `EFFECTS` must pass the parity script (`crates/pixfx-export/examples/parity.rs`) against
  its source: pixelfx, or the Sprite Fusion file vendored at `tools/spritefusion/<id>.ts`. An effect made here has no
  source; its golden hashes pin its pixels, and its module doc says it is made here. A port keeps its source's
  JS number semantics: same order of operations, `js_round`, `libm`. Do not tidy maths.
- An effect that reads the sprite it wraps reads it through `pixfx_core::Silhouette` and `Actor::opaque`, built once
  in `set_actor`, never per drawn frame (`.claude/skills/pixfx-sprite-aware/SKILL.md`).
- Determinism: same options and time give the same `frame_hash` on every target. No `HashMap` iteration order, no
  unseeded randomness.
- A `frame_hash` change to an existing effect needs a reason in the commit message; "the golden was updated" is not
  a reason.
- Work that depends on nothing the game sets (a grown tree, cracks, a layout) is baked once by a script in
  `examples/bake_<name>.rs` into a data file the effect reads, never redone per instance or per frame
  (`.claude/skills/pixfx-effect/SKILL.md`, "Bake what nothing changes").

## Before you finish

Working on one effect, build and test it alone (its id with `/` as `-` is its feature); every test about other
effects is left out, and the golden check covers that effect only:

```
cargo test -p pixfx-effects --no-default-features --features fire-candle --all-targets
cargo test -p pixfx-effects --no-default-features --features fire-candle --test determinism -- --ignored
cargo run --release -p pixfx-effects --no-default-features --features fire-candle --example bench
cargo run -p pixfx-export --example parity -- fire/candle --pixelfx /tmp/pixelfx --out target/parity
```

A change shared by many effects (`lib` modules, a category's helpers) runs the same with the default `all`, and the
wasm golden check once: `wasm-pack test --node crates/pixfx-effects --test determinism -- --include-ignored`.
