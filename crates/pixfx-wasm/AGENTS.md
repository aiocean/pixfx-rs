# pixfx-wasm

Layer: **JS binding**. Read the root `AGENTS.md` ("The boundary" and "Layers and where code goes") before changing
anything here; this file only says what this crate holds.

## Job

The engine for JavaScript. It translates and decides nothing: `effects()`, `meta(id)`, `new Fx(id, opts)`,
`update(dt)`, `draw(layer)`, `color()`, `light()`, `rgba()`, `takeCues()`, `new Feel(spec?)`, `new Squash(spec?)`,
`new Blink(spec?)`, and `parse_options`. Every JS shell (Canvas 2D, Phaser, Pixi) loads the engine through it.

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

`pixfx-core` (with `ts`), `pixfx-draw`, `pixfx-effects` (`default-features = false`; the `all` feature turns every
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
