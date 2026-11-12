# pixfx-phaser — pixfx effects in Phaser 4

Date: 2026-11-13 · Status: draft for owner review

## Intent

Any Phaser 4 game can import one small package and play pixfx effects in its scenes: "play `fire/campfire`
at (x, y)" and the effect animates in the world, pixel-crisp at any zoom, its light glowing additively onto
whatever is beneath it, and the game's own sprites can stand between the effect's back and front layers.

Why: the Rust engine now runs in the browser (`pixfx-wasm`), and the game-factory games are Phaser 4. Today a
game that wants pixelfx effects vendors the JS library and writes its own Phaser glue (Terminal Tails:
`projects/terminal-tails/src/fx/fx-layer.ts`). One shared package replaces that per-game glue.

## Goals

- A TypeScript package `pixfx-phaser` that any Phaser 4 game imports; it drives the `pixfx-wasm` engine.
- **Two ways to place effects** (owner decision: both):
  - **Per-effect objects (default):** each effect is its own pair of Phaser images (back, front) with its own
    depths, so it sorts against sprites like any game object (behind one character, in front of another;
    y-sorting works).
  - **Scene-wide layer mode (opt-in):** a layer owns one back and one front texture covering a region at fixed
    depths; every effect played through it draws into those. For games with many small effects.
- **Controls** (owner decision: all four in the first version):
  - play at a world position; stop (the effect fades out as `Instance::stop` does); destroy at once;
  - **follow** a game object (position tracks it each frame, with an offset);
  - **completion event**: an effect that finishes (one-shot ends, or a stopped loop has faded) emits
    `complete` and removes itself;
  - **effect options**: seed, flip, angle, light on/off, palette, and per-effect extras — exactly what
    `pixfx-wasm`'s `Fx` constructor accepts;
  - **scene time**: effects advance on the scene's clock: they freeze when the scene pauses or sleeps and
    follow `scene.time.timeScale` (slow motion, hit stop).
- Pixel-crisp: effect textures use nearest-neighbour filtering whatever the game's `pixelArt` setting.
- Light is additive: the colour layer draws with normal blending; the light layer draws with Phaser's
  additive blend so ground and sprites under a fire brighten (dst.rgb += light.rgb, dst alpha unchanged —
  the same rule as the gpui and Canvas 2D renderers). Unlit light pixels add nothing.
- A demo scene that shows all of the above with the three ported effects (candle, campfire, brazier).

## Non-goals

- Porting more effects (the package plays whatever `pixfx-wasm` registers).
- Changing any game (Terminal Tails, Catdoku): adoption is a later, separate step.
- Publishing to npm (games link the package locally for now).
- A WebGPU path or custom Phaser shaders/pipelines.
- Phaser 3 support.

## Anti-goals (a failure even if it "works")

- Any pixel difference between what `pixfx-phaser` shows and the `pixfx-wasm` frame for the same effect, seed
  and time (before blending onto the scene).
- Blurry effect pixels at any camera zoom or object scale.
- Effects that keep running while their scene is paused or asleep, or that ignore `timeScale`.
- Textures, images or wasm instances left behind after an effect completes or its scene shuts down or is
  destroyed (leaks across scene restarts).
- A game having to write per-frame glue: once the plugin is installed, `play()` is the whole job.

## Constraints

- **Language and checks:** TypeScript strict, zod at every boundary where untyped data enters (play options,
  effect metadata from wasm). typescript-eslint bans: `no-explicit-any`, `no-unsafe-*`,
  `no-non-null-assertion`, `consistent-type-assertions: never`, `ban-ts-comment`.
- **Tooling:** `bun` as package manager and test runner; Vite for the demo (the game-factory games use Vite).
- **Phaser:** peer dependency `phaser` `^4.2.1` (the version the game-factory Phaser games pin).
- **Engine:** consumes the `pixfx-wasm` `--target web` build (`effects`, `meta`, `Fx` with `update`, `stop`,
  `done`, `t`, `width`, `height`, `originX`, `originY`, `draw(layer)`, `color()`, `light()`, `rgba()`).
  The colour words are ABGR, light words are xBGR (pixelfx packing).
- **Glow margin:** a per-effect texture must be large enough that the light glow is not clipped at the effect's
  own `meta.size`. `pixfx-canvas` already solved this with a margin of `max(w, h) / 2` and no edge fade
  (commit 80ce1e6); `Fx` in `pixfx-wasm` is margin 0. Expose a margin on the engine side rather than
  re-implementing the frame in TypeScript.
- **Wasm loading:** must work in a plain Vite game without extra Vite plugins; the game calls one async
  init before the first `play()`.
- **Upload cost:** upload a texture only when its frame changed; per-effect textures are sized to the effect
  (plus margin), not to the screen.
- **Repo:** lives in `pixfx-rs` (proposed path `packages/pixfx-phaser/`), next to the crates it builds on.

## Decided by the owner

- A reusable package, proven by a demo, before any game adopts it.
- Both placement modes: per-effect objects (default) and scene-wide layers (opt-in).
- First-version controls: follow, completion event, effect options, scene time.

## Calls made by the assistant (owner may override)

- Package location `pixfx-rs/packages/pixfx-phaser`, linked locally by games; npm later.
- Phaser integration as a **Scene plugin** (`scene.pixfx.play(...)`) so per-scene update and shutdown
  cleanup are automatic.
- One effect pixel = one world unit at scale 1; camera zoom and object scale enlarge it (nearest filtered).
- Additive light via Phaser's `BlendModes.ADD`; must be pixel-proven (WebGL renderer required; Canvas
  renderer should map ADD to `"lighter"` and is checked too).
  Phaser 4.2.1's WebGL ADD is `src·1 + dst·dstAlpha` (`WebGLRenderer.js:733-734`): over an opaque scene it adds
  exactly the light; over a transparent render target it adds nothing. Opaque scenes are in scope; transparent
  render targets are a documented limitation.
- Each effect is four drawables: back colour, back light, front colour, front light; light images use ADD.
  Light pixels are uploaded as rgb with alpha 255 where lit and 0,0,0,0 where unlit (adds nothing).
- **Glow margin:** the formula `max(w, h) / 2` moves from `pixfx-canvas` (`stage.rs:14`) into `pixfx-core`,
  used by both. `pixfx-wasm`'s `Fx` gains an optional `margin` option (default 0, so its current frames and
  tests are unchanged; edge fade applies only at margin 0, as today); `pixfx-phaser` passes the core margin.
- **Wasm distribution:** `pixfx-phaser` builds `pixfx-wasm` with `wasm-pack --target web` into its own
  git-ignored `wasm/` directory via a package script and ships it beside its JS, so a linked package keeps the
  `.wasm` next to the glue. The demo's `vite build` must emit the `.wasm` with no Vite plugin.
- **Scene time:** each frame's dt is the scene `update` delta × `scene.time.timeScale`, in seconds. Paused and
  sleeping scenes get no update, so effects freeze. A long gap (tab switch) is bounded by the engine's
  `MAX_DT` (0.25 s) — the effect does not try to catch up.
- **Space:** effects live in world space by default; `play` takes a `scrollFactor` option (0 = screen space,
  for HUD effects). Positions snap to whole effect pixels so blocks stay on a grid. A layer's bounds are fixed
  at creation; effects outside them are clipped.
- **Lifecycle:** `stop()` on a finished effect does nothing. `destroy()` removes at once and emits `destroy`,
  not `complete`. If a followed object is destroyed, the effect stops (fades) at its last position. Every `Fx`
  is freed (`.free()`) when its effect leaves; the plugin counts live engines, textures and images so the
  leak checks can read them.
- **Pixel-equality proof:** compare texture contents against `color()` and `light()` words for the same
  effect, seed, time and margin (not `rgba()`, which bakes light into colour). The WebGL renderer is the exact
  gate; the Canvas renderer is a behavioural gate (animates, crisp, additive brightening, layering), not exact
  pixels, since its premultiplication may differ.
- Bun unit tests cover the pure parts against a fake engine; real wasm is exercised in the browser checks.

## Left to the builder

- The exact public API names and types (sketch: `scene.pixfx.play(id, x, y, opts) → PixfxEffect` with
  `stop()`, `destroy()`, `follow(obj, dx?, dy?)`, `setDepths(back, front)`, `on("complete")`;
  `scene.pixfx.createLayer({ depths, bounds }) → layer.play(...)`).
- How textures are created and updated (canvas texture + `putImageData`, or a raw-pixel texture update),
  provided the upload-cost and crispness constraints hold.
- Default depths for per-effect objects relative to the follow target.
- Demo contents beyond: ground, the three fires, a moving character standing between a fire's layers, a torch
  (candle) following a moving sprite, a pause/slow-motion toggle, layer mode shown side by side.

## How it is verified

- Unit tests (bun) for the pure parts: clock (pause, timeScale, dt validation), completion, follow maths,
  option parsing, cleanup bookkeeping.
- Browser proof (ego-browser) on the demo, WebGL renderer and Canvas renderer:
  - two screenshots 0.5 s apart differ in each effect's region; nothing changes while paused;
  - effect pixels are uniform blocks at zoom 4 (no blur);
  - ground under the campfire is brighter than far from it (additive light);
  - a sprite between layers is covered by the front layer and over the back layer where they overlap;
  - texture count returns to its starting value after effects complete and after a scene restart;
  - pixel-equality of an unblended effect frame against `pixfx-wasm` for the same seed and time.
