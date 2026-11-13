# pixfx-light

Layer: **Engine**. Read the root `AGENTS.md` ("The boundary" and "Layers and where code goes") before changing
anything here; this file only says what this crate holds.

## Job

Scene light: the glow a spell, a blow or a lamp throws on the world. A `Light` is a value that ages (`step`), shows
as a `Glow` at any instant (`Light::glow`), and `draw` adds glows to a frame's light layer in hard-edged rings. How
a light swells, fades and rings is data (`Style`).

## Belongs here

- How a light looks and ages; how glows are drawn onto a light layer.

## Does not belong here

| If the code... | It goes to |
|---|---|
| holds the list of lights a game has alive, or keys them | the caller (the game, or `pixfx-scene` when every shell needs the same list) |
| decides which game event casts which light | the game |
| is a JS binding of `Light` / `Glow` / `Style` | `pixfx-wasm` |
| reads a clock or draws to a host canvas | a shell |

## Dependencies

- Required: `pixfx-draw`, `libm`. Not `pixfx-core`: light needs only frames.
- Optional, behind `serde` / `ts`: `serde`, `tsify`, `wasm-bindgen`.

## Traps

- Pure: no state, no clock, no dice. The caller passes time in.
- `draw` matches what Chrome's Canvas 2D `fillRect` adds under `"lighter"` with `globalAlpha` (8 bits a channel).
  Changing the rounding breaks that match.

## Before you finish

```
cargo test -p pixfx-light
cargo test --workspace
```
