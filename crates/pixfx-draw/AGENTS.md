# pixfx-draw

Layer: **Drawing**, the lowest layer. Read the root `AGENTS.md` ("The boundary" and "Layers and where code goes")
before changing anything here; this file only says what this crate holds.

## Job

Pixels and nothing else: `Frame` with a colour layer and an additive light layer, the `painter`, `shapes`, `blend`
and `composite`, `frame_hash`, `color` (`Rgba`, `hex`), `palette` presets, `outline` rings, `RgbaImage`
(plain RGBA bytes: a sprite leaving the engine or a picture coming in), and the JS-number maths in `math`
(`js_round`, `js_max`, `js_min`, `clamp`). Every other crate paints through it.

## Belongs here

- A new way to put pixels in a frame: a primitive, a blend, a palette, an outline style.
- Sprites, text, nine-slice and UI layout when a real game needs them. That game decides the API; do not add one
  ahead of it.
- Number helpers that must give identical bits natively and in wasm.

## Does not belong here

| If the code... | It goes to |
|---|---|
| knows what an effect, a particle, an option or a cue is | `pixfx-core` / `pixfx-effects` |
| decides where something is placed, on which layer, or which frame shows | `pixfx-scene` |
| knows a host (`web-sys`, `js-sys`, gpui, files, a clock) | a shell |
| is a JS-facing type or option parser | `pixfx-wasm` |

## Dependencies

- Required: `libm` only. Nothing from this workspace: every other crate sits above this one.
- Optional, behind the `serde` / `ts` features only: `serde`, `tsify`, `wasm-bindgen`.
- No `std::time`, no I/O, no randomness that the caller does not pass in.

## Traps

- Float maths goes through `libm`, never `f64::sin` and friends; `clippy.toml` denies them so wasm and native give
  the same bits. Rounding is `math::js_round` (JS `Math.round`), not `f64::round`.
- The light word's top byte is unused (`composite_px` masks it). Never start reading it as alpha.
- Every effect's pixels go through this crate. Any change here can move every `frame_hash`.

## Before you finish

```
cargo test -p pixfx-draw
cargo test --workspace                                        # golden hashes of every effect
cargo test -p pixfx-effects --test determinism -- --ignored
```

A changed `frame_hash` without a stated reason is a failure, even if the result looks right.
