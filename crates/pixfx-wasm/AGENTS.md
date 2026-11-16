# pixfx-wasm

Layer: **JS binding**. Read the root `AGENTS.md` ("The boundary" and "Layers and where code goes") before changing
anything here; this file only says what this crate holds.

## Job

The engine for JavaScript. It translates and decides nothing: `effects()`, `meta(id)`, `new Fx(id, opts)`,
`update(dt)`, `draw(layer)`, `color()`, `light()`, `rgba()`, `takeCues()`, `lightColor(id, opts)`, `new Feel(spec?)`,
`new Squash(spec?)`, `new Blink(spec?)`, and `parse_options`. Every JS shell (Canvas 2D, Phaser, Pixi) loads the engine through it.

`src/scene.rs` is the `pixfx-scene` scene for JS shells: `new Frames(workers)` once per game, `frames.scene(scale)`
per stage, `scene.play/moveTo/stop/order/update/events` (`order(handle, x, y, aim, caster)`: an effect that takes
orders launches its volley; each piece's strike comes back as a `"strike"` event naming the body; `setBody(id, x, y,
w, h)`, `clearBody(id)`, `burst(piece)`, `trail(handle, id, opts)`; a paint job carries a trail's trace as
`x, y, angle, age` per point), then `scene.draw(frames)` and `frames.endFrame()`, which return a
draw list (`Int32Array`, `OP_STRIDE` numbers per op: make, refill, show, free) with the pixels at
`frames.colorPtr(id)` / `lightPtr(id)` when painted on the spot. With workers (every JS shell), `frames.takeJobs()`
gives the frames to paint, a worker's `new Painting()` paints, cuts and turns each into RGBA (`painted.shape()`,
`color()`, `light()`), the worker transfers the bytes, and `scene.deliver(frames, job, shape)` takes back only the
shape: the make or refill it returns uploads the worker's bytes, which never pass through wasm on the main thread.
A worker makes an effect's state from its id and options as JSON (`options_json`, the reverse of `parse_options`),
taken from the effect's clip, so it paints what the scene played itself (a volley's shots) as well as what JS played.
Presets come from `palettePresets()`; `scene.setView(x, y, w, h, scrollX, scrollY)` and `scene.setScrollFactor(handle, f)` say what a camera sees and how
far each effect moves with it, so the scene draws only those it sees. `scene.bend(src, w, h, unit)` bends a picture of the
host's screen by the feel's live ripples (`pixfx_core::screen::ripple_rgba`), for a shell that bends pixels on the
CPU. Behind the `fighter`, `light` and `mind` features (all default): `src/fighter.rs` (`Body`, `FightFeel`, `Sprites`,
`lookColors`), `src/light.rs` (`stepLights`, `putOutLight`, `Lighting`; the game owns its list of lights) and
`src/mind.rs` (`Mind`, a fighter's brain: `meet(foeSkills)`, `think(sense, dt)`, `tell`, `reset`, `hold`, `explain`, `posture`).
Behind `duel` (default): `src/duel.rs`, the landing duel's rules from `pixfx-duel`: `new Duel(config, seats?)`
(`seats`: `["brain" | "first", ...]`, the brain when left out) or `Duel.withPolicies(config, seats?, left, right)`, a
trained policy per corner (`seats`: `["policy" | "brain" | "first", ...]`, each corner's policy when left out),
`step(dt)` (commands and events), `view()`, `hold(me, seconds)`; `new Policy(bytes)`, a trained policy read from a
weights file (`crates/pixfx-duel/weights/*.bin`), which the page fetches beside the wasm (the wasm never carries the
weights); its tables as
the page reads (`duelSpells()`, `duelTraits()`, `duelStats()`, `duelConstants()`), `damageOf` and `orbOffset`.
A `hit` event says what kind of blow it was (`how`: `"plain"`, `"counter"` on a fighter gathering a cast, `"punish"`
on one recovering); `FightFeel.react` reads it, stops longer and floats the word (`tests/hit_words.rs` keeps the
duel's and the feel's spelling the same).
`Sprites.picture()` and `Lighting.paint()` return a `Picture` (`src/picture.rs`: the lit box's colour and light
RGBA) that a shell draws. `Squash.step(dt, x, y, vx?, vy?)` works out velocity from movement.

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
effect on), `js-sys`, `wasm-bindgen`, `serde`, `serde-wasm-bindgen`, `serde_json`, `serde_path_to_error`, `tsify`;
optional behind their features: `pixfx-fighter`, `pixfx-light`, `pixfx-mind`, and `pixfx-duel` (`duel`).

## Known drift

- It depends on `pixfx-export` (a file shell) for `pixfx_export::decode` in `src/lib.rs`: a binding reaching up into
  a shell. Decoding a bake is a format every host reads; it belongs lower. Do not add more uses of `pixfx-export`
  here.

## Before you finish

```
cargo test -p pixfx-wasm
cargo clippy -p pixfx-wasm --all-targets -- -D warnings
(cd packages/pixfx-web && bun run build:wasm && bun run check)
```

A changed JS-facing name or shape breaks every JS shell; update them in the same change.
