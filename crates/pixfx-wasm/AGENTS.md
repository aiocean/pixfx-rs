# pixfx-wasm

Layer: **JS binding**. Read the root `AGENTS.md` ("The boundary" and "Layers and where code goes") before changing
anything here; this file only says what this crate holds.

## Job

The engine for JavaScript. It translates and decides nothing: `effects()`, `meta(id)`, `new Fx(id, opts)`,
`update(dt)`, `draw(layer)`, `color()`, `light()`, `rgba()`, `takeCues()`, `new Feel(spec?)`, `new Squash(spec?)`,
`new Blink(spec?)`, and `parse_options`. Every JS shell (Canvas 2D, Phaser, Pixi) loads the engine through it.

`src/scene.rs` is the `pixfx-scene` scene for JS shells: `new Frames(workers)` once per game, `frames.scene(scale)`
per stage, `scene.play/moveTo/stop/update/events`, then `scene.draw(frames)` and `frames.endFrame()`, which return a
draw list (`Int32Array`, `OP_STRIDE` numbers per op: make, refill, show, free) with the pixels at
`frames.colorPtr(id)` / `lightPtr(id)` when painted on the spot. With workers (every JS shell), `frames.takeJobs()`
gives the frames to paint, a worker's `new Painting()` paints, cuts and turns each into RGBA (`painted.shape()`,
`color()`, `light()`), the worker transfers the bytes, and `scene.deliver(frames, job, shape)` takes back only the
shape: the make or refill it returns uploads the worker's bytes, which never pass through wasm on the main thread.
Presets come from `palettePresets()` and `outlinePresets()`; `Squash.step(dt, x, y, vx?, vy?)` works out velocity from movement.

## Belongs here

- A `wasm-bindgen` wrapper over a Rust type that already exists in a lower crate.
- Reading a JS object once, strictly, into Rust types (`js::parse`): a misspelt, missing or mistyped field throws,
  naming it.
- Handing engine-owned data (presets, effect meta, option specs) to JS, so no shell copies it.

## Does not belong here

| If the code... | It goes to |
|---|---|
| makes a decision: a default, a clamp, a timing rule, a placement, which frame shows | the lower crate that owns it (`pixfx-core`, `pixfx-scene`, ...) |
| touches Canvas, Phaser, Pixi or the DOM | a JS shell |
| is a new effect or feel rule | `pixfx-effects` / `pixfx-core` |

The test: if a gpui game would need the same behaviour, it is not translation, and it moves down.

## Dependencies

`pixfx-core` (with `ts`), `pixfx-draw`, `pixfx-scene` (with `ts`), `pixfx-effects` (`default-features = false`; the `all` feature turns every
effect on), `js-sys`, `wasm-bindgen`, `serde`, `serde-wasm-bindgen`, `serde_json`, `serde_path_to_error`, `tsify`.

## Known drift

- It depends on `pixfx-export` (a file shell) for `pixfx_export::decode` in `src/lib.rs`: a binding reaching up into
  a shell. Decoding a bake is a format every host reads; it belongs lower. Do not add more uses of `pixfx-export`
  here.

## Before you finish

```
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
(cd packages/pixfx-phaser && bun run build:wasm && bun run check)
```

A changed JS-facing name or shape breaks every JS shell; update them in the same change.
