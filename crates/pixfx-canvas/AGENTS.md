# pixfx-canvas

Layer: **Shell** (Canvas 2D). Read the root `AGENTS.md` ("The boundary" and "Layers and where code goes") before
changing anything here; this file only says what this crate holds.

## Job

A Canvas 2D shell in wasm: `new Stage(canvas, scale)` runs its own `requestAnimationFrame` loop (or the host calls
`tick` / `update(dt)` and `drawLayer`), implements `Shell` over canvases, and presents what `pixfx_scene::Scene`
decides. Demo in `demo/`.

## A shell may only

Create and free host objects (canvases, `ImageData`), copy pixels Rust painted into them, place them where `Pass`
says, run the host loop and hand Rust the elapsed time, and turn host input into calls.

## Does not belong here

| If the code... | It goes to |
|---|---|
| decides placement, layering, follow, fade, lifetime or which frame shows | `pixfx-scene` |
| decides feel (how far a shake moves, how long a flash lasts) | `pixfx-core` |
| parses JS options or wraps an engine type for JS | `pixfx-wasm` |
| paints or composites pixels | `pixfx-draw` |

The test: a Phaser or Pixi shell would have to write the same line. If yes, it is not shell code.

## Dependencies

The engine crates, `pixfx-scene`, `pixfx-wasm` (for `js::parse` and `parse_options`), `js-sys`, `web-sys`,
`wasm-bindgen`. No other shell's code.

## Known drift

- `src/fighter.rs` and `src/light.rs` are JS bindings of `pixfx-fighter` and `pixfx-light`, and `light.rs` keeps the
  list of live lights. Bindings belong in `pixfx-wasm` so every JS shell gets them; the list of lights is a decision
  for `pixfx-scene` or the game. Do not add more bindings here.
- It depends on `pixfx-export` for `decode`; see `crates/pixfx-wasm/AGENTS.md`.

## Before you finish

```
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
wasm-pack build crates/pixfx-canvas --target web --out-dir demo/pkg   # then open demo/ in Chrome
```
