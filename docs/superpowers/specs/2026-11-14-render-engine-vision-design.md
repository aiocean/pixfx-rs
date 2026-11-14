# pixfx as the pixel render engine under every game

Date: 2026-11-14 · Approved by the owner in a brainstorming session.

## Intent

pixfx starts as a library of pixel-art effects. The owner wants it to become the Rust render engine under every game
in game-factory, the way gpui sits under Zed: Zed is the product, gpui is the drawing library built inside it, and the
product decides what the library grows. pixfx effects and game feel are the first product; sprites, UI and anything
else a game draws come next, all through one engine with one look.

Games are made two ways, and both sit on the same engine:

| | JS game | gpui game |
|---|---|---|
| Example | a Capacitor game like terminal-tails | a native mobile app in Rust |
| Game code | TypeScript | Rust |
| Reaches the engine through | the wasm binding (`pixfx-wasm`, `pixfx-canvas`) | Rust directly (`pixfx-gpui`) |
| Platforms | web, and iOS and Android through Capacitor | what the gpui fork supports |

## Goals

- One Rust engine draws sprites, UI and effects for a game, with colour and additive light, outline and palette shared
  by all of them.
- The same scene gives the same pixels on every platform and in both kinds of host.
- Fast enough for many sprites and effects at 60 fps on a mid-range phone, shown by measurement.
- Easy to adopt: a game draws with a few calls and plain-data options.
- Any way of showing: Canvas 2D, WebGL, CSS and DOM, Phaser, gpui, an exported file. Adding one is a new shell only.

## Non-goals

- A general game engine. Scenes, physics, input, audio, ads, leaderboards and saves stay with the game and its host
  (Capacitor plugins, Nakama client).
- Rebuilding Phaser or Bevy feature by feature.
- A separate repository for the drawing library now.

## Anti-goals

- Features no game uses. The engine grows only when a real game needs something.
- Logic in a shell or binding. If the JS layer or `pixfx-gpui` decides anything on its own, the ways of showing
  drift apart.
- Losing parity. Any step that changes an existing effect's `frame_hash` without a reason is a failure even if the
  engine works.

## Constraints (checked 2026-11-14)

- `pixfx-core` holds three jobs in about 5,800 lines: drawing (`frame`, `painter`, `shapes`, `blend`, `color`,
  `palette`, `outline`; about 1,900 lines), the effect model (`effect`, `particles`, `sim`, `render`, `player`, `key`)
  and game feel (`feel`, `cue`, `act`, `squash`, `hit_flash`, `screen`, `state`).
- `pixfx-core` needs only `libm`; JS bindings sit behind optional features.
- Every game in `projects/` draws with Phaser (catdoku, terminal-tails) or Pixi (color-connect) inside Capacitor. None
  uses pixfx yet.
- `packages/pixfx-phaser/src` holds about 2,100 lines of TypeScript. Sorted by file (read from each file's doc comments
  and functions, 2026-11-14):
  - **Logic to move into Rust:** `pixels.ts` (`compositeColor`, `compositeLight`, `snap`: a TypeScript copy of the
    Rust `composite`); `layer.ts` and `layout.ts` (scene-wide layer compositing, `clipRect` placement); `lifecycle.ts`
    and `effects.ts` (effect state playing/stopping/destroyed, follow, per-scene ticking: the `Scene`'s job);
    `runner.ts` (which frame to show from `floor(t × fps)`, already decided in Rust); `sprite-feel.ts` (velocity from
    distance moved when a body has none); `options.ts` `PALETTE_PRESETS` (a copy of `palette::PRESETS`).
  - **Shell, stays:** `plugin.ts`, `effect.ts`, `gfx.ts`, `surface.ts` (Phaser textures and images); `feel.ts` (applies
    the Rust `Feel` to the camera); `pool.ts`, `worker.ts`, `worker-core.ts`, `protocol.ts`, `backend.ts`,
    `inline.ts`, `load.ts`, `engine.ts`, `clock.ts` (loading wasm, workers, transport); `options.ts` parsing (zod at
    the boundary).
- CSS can present light: `mix-blend-mode: plus-lighter` is in Chrome 100, Firefox 99 and Safari 9.1, and iOS and
  Android WebViews follow their browsers (`mdn/browser-compat-data`, `css.properties.mix-blend-mode.plus-lighter`).
- The gpui fork (`tools/zed-gpui`) has `gpui_ios` and `gpui_web` and no Android crate, so a gpui game ships on iOS and
  the web only until Android support is added to the fork. It already carries nearest-neighbour sampling and additive
  blending for sprites (Metal, WebGPU, WebGL).

## Approaches

- **A. Split inside this repo (chosen).** The drawing part leaves `pixfx-core` for its own crate and grows sprites,
  text and UI layout as games need them. Effects and game feel depend on it as its first product. The bindings bridge
  it to TypeScript and gpui. One repo, one-way dependencies, and the parity gate guards existing effects at every step.
- **B. A new engine repo from scratch, pixfx moved onto it later.** Cleanest start, but a second painter drifts from the
  first, and the engine grows without a product pulling it.
- **C. Add sprites and UI to `pixfx-core` without splitting.** Fastest today, but core already mixes three jobs, and
  games could not depend on the drawing part alone.

A was picked because it is the incremental path to the owner's goal and keeps every step checked by the existing
tests and parity gate. Moving the drawing library to its own repo stays possible once something outside game-factory
needs it.

## Decided

- **Owner:** pixfx is the render engine under every future game; games call it directly (sprites, UI, effects).
- **Owner:** game code may be TypeScript (Capacitor, through wasm) or Rust (gpui, for example a mobile app); both use
  the same engine.
- **Owner:** structure A.
- **Owner:** Rust decides everything: effects, sprites, UI, and what is drawn where. JS is a thin layer. A shell
  only presents, so one game can be shown with Canvas 2D, WebGL, CSS and DOM elements, Phaser or gpui. The contract
  is the `Shell` trait in `crates/pixfx-canvas/src/stage.rs` (`make`, `refill`, `show`), driven by the Rust `Scene`.
- **AI's call:** the engine paints every surface's pixels; a shell keeps the composition identical by honouring
  whole-pixel placement, nearest-neighbour scaling and additive light (CSS: `image-rendering: pixelated`,
  `mix-blend-mode: plus-lighter`).
- **AI's call:** `Scene` moves from `pixfx-canvas` (a wasm crate) into a host-neutral crate so `pixfx-gpui` shares it
  instead of keeping its own layout.
- **AI's call:** a new engine feature ships for both kinds of game.
- **AI's call:** the first game that adopts the engine decides the order of new features; a shipped Phaser game is not
  migrated first.

- **Owner:** computing and showing are separate. `Scene` decides on the main thread with clocks only; a `Painter`
  paints, on workers for JS shells; a game-wide `Frames` is shared by every scene. Chosen by reasoning:
  - pixel work leaves the main thread;
  - pixels cross to the main thread once, straight to the GPU;
  - WebGL shells use one atlas;
  - layer groups (`createLayer`) are dropped.
- **AI's call, owner agreed:** a terminal (ASCII) or an OLED screen is a new shell; embedded targets would add
  `no_std`, frame memory and `f64` cost on top.

## Left to the builder

- The drawing crate's name and the exact module split, keeping every `frame_hash` unchanged.
- The sprite, text and UI APIs, designed against the first game that needs each one.
- Which game adopts the engine first, and its own plan.
