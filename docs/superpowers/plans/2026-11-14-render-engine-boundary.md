# Render engine boundary Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Put every decision in Rust and leave each host a thin shell: drawing in its own crate, `Scene` host-neutral,
the Phaser package a pure shell, and a Pixi shell on the same contract.

**Architecture:** Five layers with one-way dependencies (`AGENTS.md`, "Layers and where code goes"): `pixfx-draw`
(pixels) ← `pixfx-core` and friends (engine) ← `pixfx-scene` (what is shown where) ← `pixfx-wasm` (JS translation)
← shells (Canvas 2D, Phaser, Pixi, gpui, files). A JS shell receives a draw list (plain data) from a wasm `Scene` and
executes it; it decides nothing.

**Tech Stack:** Rust (nightly pinned in `rust-toolchain.toml`), wasm-bindgen, wasm-pack, TypeScript with zod, Phaser 4,
PixiJS 8, bun.

**Spec:** `docs/superpowers/specs/2026-11-14-render-engine-vision-design.md` and `AGENTS.md` (the "Out of place today"
list is this plan's scope).

## Before starting

Work in the main tree on `main` (the owner's call: no worktree). Other sessions share this tree: before a task, tell
the sessions working in its files, and re-read every file named below before editing it, since line numbers move.

## Global Constraints

- Every effect's `frame_hash` is unchanged by every task: `cargo test --workspace` (golden hashes) and
  `cargo test -p pixfx-effects --test determinism -- --ignored` pass before and after.
- The gate after every task: `cargo fmt --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace`,
  `cargo clippy --manifest-path crates/pixfx-gpui/Cargo.toml -- -D warnings`, `(cd packages/pixfx-phaser && bun run check)`,
  `bun test site/tests`.
- Workspace lints stay: `unsafe_code = "forbid"`; clippy `unwrap_used`, `expect_used`, `panic`, `disallowed_methods` deny.
- No re-exports for moved items: callers import from the new crate (no `pub use pixfx_draw::*` in `pixfx-core`).
- `pixfx-draw`, `pixfx-core` and `pixfx-scene` take no `wasm-bindgen`, `js-sys` or `web-sys` dependency except behind
  the existing optional `ts` feature.
- The light word's top byte is unused (`composite_px` masks it). No move may start reading it as alpha.
- A task that changes a JS shell records `bun run perf` (in `packages/pixfx-phaser`) before and after in its commit
  message; the after numbers are no worse than noise (the sweep's own spread).

## Review Focus

1. An effect stopped while following a moving sprite keeps fading at the sprite's last position (today in
   `lifecycle.ts`); it must not jump to 0,0 after the move into `Scene`. Test in Task 3.
2. A hit-stop (`dt` = 0) holds every effect's frame and every sprite's squash; no frame advances, no squash
   velocity spikes from a zero `dt`. Tests in Task 3 and Task 4.
3. A scene with more effects than the frame budget (`FRAME_BUDGET`, 64 MiB) drops least-recently-shown frames and
   tells the shell to free those surfaces; the shell's texture count returns to 0 after all effects end. Test in
   Task 3 (draw list `Free` ops) and Task 4 (`stats()` leak check, existing). Goldens: `crates/pixfx-effects/tests/golden*`.
4. Changing scale mid-play (`setScale`) re-makes surfaces at the new size; no surface is shown with a stale size.
   Test in Task 3.
5. A Phaser scene shut down mid-effect frees every texture and wasm instance (no leak in `stats()`); same for Pixi.
   Tests in Task 4 and Task 5.

---

### Task 1: `pixfx-draw`, the drawing crate

**Files:**
- Create: `crates/pixfx-draw/Cargo.toml`, `crates/pixfx-draw/src/lib.rs`
- Move (git mv): `crates/pixfx-core/src/{math,color,frame,painter,shapes,blend,palette}.rs` → `crates/pixfx-draw/src/`
- Split: `crates/pixfx-core/src/outline.rs` → `crates/pixfx-draw/src/outline.rs` (`Ring`, `Outline`, `PRESETS`,
  `preset`, `OutlineError`, `MAX_WIDTH`, `SHAPE_NEIGHBOURS`, `DARK`, ring painting); the `impl Options` block stays in
  `pixfx-core` (into `crates/pixfx-core/src/effect.rs`)
- Move tests: `crates/pixfx-core/tests/{math,painter,palette,outline}.rs` → `crates/pixfx-draw/tests/`
- Modify: every `use pixfx_core::{math,color,frame,painter,shapes,blend,palette,outline}::…` and every crate-root
  import of `Rgba`, `hex`, `ColorError`, `Frame`, `composite`, `composite_px`, `frame_hash`, `Painter`, `Outline`,
  `OutlineError`, `Ring`, `Rng`, `rand` (`rg 'pixfx_core::' crates packages examples` lists them; 61 files at writing)
- Modify: `crates/pixfx-export/src/bake.rs` (uses `Frame`; import change only)
- Modify: `Cargo.toml` of every crate that now uses drawing items directly (add `pixfx-draw = { path = "../pixfx-draw" }`)

**Interfaces:**
- Produces: crate `pixfx_draw` with public modules `math`, `color`, `frame`, `painter`, `shapes`, `blend`, `palette`,
  `outline`, and root re-exports `Rgba`, `hex`, `ColorError`, `Frame`, `composite`, `composite_px`, `frame_hash`,
  `Painter`, `Outline`, `OutlineError`, `Ring`, `Rng`, `rand` (the same names `pixfx_core` exported). Features
  `serde` and `ts` as in `pixfx-core` (`color.rs` derives depend on them); `pixfx-core`'s `serde`/`ts` features
  enable `pixfx-draw/serde`/`pixfx-draw/ts`.
- `pixfx_core` no longer exports any of those names.

- [ ] **Step 1:** Record the baseline: run the gate and the determinism test on `main`; save the output.
- [ ] **Step 2:** Create `pixfx-draw` (deps: `libm`; optional `serde`, `tsify`, `wasm-bindgen`; `[lints] workspace = true`;
  `#![forbid(unsafe_code)]`), `git mv` the modules and tests, split `outline.rs` as listed.
- [ ] **Step 3:** `cargo build -p pixfx-draw && cargo test -p pixfx-draw`. Expected: PASS (moved tests).
- [ ] **Step 4:** Rewrite imports in `pixfx-core` (`crate::color` → `pixfx_draw::color`, …) and every other crate.
- [ ] **Step 5:** Run the full gate and the determinism test. Expected: PASS, golden hashes unchanged.
- [ ] **Step 6:** `rg -n 'pixfx_core::(math|color|frame|painter|shapes|blend|palette|outline)\b' crates packages examples`
  returns nothing.
- [ ] **Step 7:** Commit `draw: frames, colour, painting and outline move out of pixfx-core into pixfx-draw`.

### Task 2: `pixfx-scene`, host-neutral `Scene`

Done in two commits: 2a moves `Scene` and `Shell` (with `free` and `Scene::clear`) and the Canvas shell onto them;
2b ports `pixfx-gpui` (Step 5), which needs a per-handle draw since gpui paints one view per effect. Review Focus 4
(scale change) needs no test: surfaces hold frames in game pixels, and the scale only changes `Pass::rect`.

**Files:**
- Create: `crates/pixfx-scene/Cargo.toml`, `crates/pixfx-scene/src/lib.rs`
- Move (git mv): `crates/pixfx-canvas/src/stage.rs` → `crates/pixfx-scene/src/scene.rs`, and
  `crates/pixfx-canvas/tests/stage.rs` → `crates/pixfx-scene/tests/scene.rs`
- Modify: `crates/pixfx-canvas/src/lib.rs` (import `Scene`, `Shell`, `Pass`, `Rect`, `StageEvent`, `StageError`,
  `PlacementChange`, `FRAME_BUDGET`, `color_rgba`, `light_rgba` from `pixfx_scene`; keep `stage_options` here, it
  takes a `JsValue`)
- Modify: `crates/pixfx-gpui/src/lib.rs`, `crates/pixfx-gpui/Cargo.toml` (`FxView` drives a `Scene` through a gpui
  `Shell`; today it is `FxView` over `Player` plus `FxFrames` (`FrameStore<Painted>`), whose store `Scene` replaces)
- Keep: `Scene::load` (baked frames, used by `Stage.loadBaked` in `pixfx-canvas`) moves with `stage.rs` unchanged.

**Interfaces:**
- Consumes: `pixfx_draw` (Task 1).
- Produces: crate `pixfx_scene` exporting `Scene<S>` with the methods it has today (`new(scale: NonZeroU32)`, `play`,
  `move_to`, `placement`, `set`, `stop`, `remove`, `drain_events`, `handles`, `rect`, `t`, `update(dt) -> Result<f64, StageError>`,
  `set_frame_budget`, `frame_bytes`, `draw<H: Shell<Surface = S>>`, `draw_layer`, `feel`, `feel_mut`, `set_feel`,
  `set_screen_w`, `scale`, `set_scale`), trait `Shell { type Surface; type Error; fn make; fn refill; fn show; }`,
  `Pass`, `Rect`, `StageEvent`, `StageError`, `PlacementChange`, `FRAME_BUDGET`, `color_rgba`, `light_rgba`.
  Dependencies: `pixfx-draw`, `pixfx-core`; optional `serde` feature for `StageEvent` serialisation. No `wasm-bindgen`.
- Adds to `Shell`: `fn free(&mut self, surface: Self::Surface) -> Result<(), Self::Error>`, called when the frame
  budget drops a held frame or a layer's own surface goes with its effect (today the surface is dropped in Rust; JS
  and gpui shells hold host objects that need an explicit free).

- [ ] **Step 1:** Write the failing test `scene_frees_surfaces_it_drops` in `crates/pixfx-scene/tests/scene.rs`: a
  recording test shell counts `make` minus `free`; play two effects with `set_frame_budget(1)` (one byte), run them to
  their end with `update(1/60)` steps and `draw`; assert the live count is 0 and `frame_bytes() == 0`.
- [ ] **Step 2:** `cargo test -p pixfx-scene scene_frees_surfaces_it_drops`. Expected: FAIL (`free` missing).
- [ ] **Step 3:** Move the files, add `Shell::free`, call it where `Held` and `Own` surfaces leave the maps; implement
  `free` in the Canvas 2D shell (drop the `OffscreenCanvas`) and in the test shells.
- [ ] **Step 4:** Write `scale_change_remakes_surfaces` (Review Focus 4): after `set_scale` every `show` gets a surface
  made after the change, at the new size. Run; make it pass.
- [ ] **Step 5:** Port `pixfx-gpui`: a `GpuiShell` whose `Surface` is the gpui image pair; its `free` queues the image
  and drops it from the atlas at end of frame (as `FxFrames::end_frame` does today), never with `window.drop_image`
  mid-frame; `FxView` holds a
  `Scene<GpuiSurface>`; delete the layout code in `FxView::new` and `paint_fx` that `Scene` now does. `cargo clippy
  --manifest-path crates/pixfx-gpui/Cargo.toml -- -D warnings` and `examples/gpui-web` build.
- [ ] **Step 6:** Run the gate and `cargo test --manifest-path crates/pixfx-gpui/Cargo.toml --test frames` (headless
  gpui window: 20 campfires hold the bytes of 1; no upload within a frame; fading effects keep their own images).
  `rg -n 'wasm_bindgen|js_sys|web_sys' crates/pixfx-scene` returns nothing.
- [ ] **Step 7:** Commit `scene: Scene and the Shell contract move into host-neutral pixfx-scene; gpui uses it`.

### Task 3: a JS `Scene` in `pixfx-wasm` that returns a draw list

> **Shape as built (owner's choice, option 3).** Painting leaves `Scene` (commit 1b2bc0e): `Scene` keeps clocks and
> decides on the main thread; a game-wide `Frames` holds the shared frames and a `Painter` for every scene of the
> game; `InlinePainter` paints on the spot, a worker painter hands frames back with `Frames::deliver` /
> `Scene::deliver_own`. The JS side (`crates/pixfx-wasm/src/scene.rs`): `new Frames(workers)`,
> `frames.scene(scale)`, `scene.draw(frames)` / `frames.endFrame()` returning an `Int32Array` draw list
> (`OP_MAKE`, `OP_REFILL`, `OP_SHOW`, `OP_FREE`, `OP_STRIDE` numbers each) with pixels at `frames.colorPtr(id)` /
> `lightPtr(id)`; `frames.takeJobs()`; a worker's `Painting.paint(job)` paints,
> crops and turns the frame into RGBA, the worker transfers the bytes, and `scene.deliver(frames, job, shape)` takes
> back only their `Shape`: pixels never pass through wasm on the main thread (`Receive<D>` in `pixfx-scene`).
> `Squash.step(dt, x, y, vx?, vy?)` stands for `scaleMoved`. A followed effect is placed on the main thread each
> frame, so it never lags its target; only new pixels arrive a frame later from a worker. Task 4's worker protocol
> carries jobs and painted RGBA instead of batches of effects; the Phaser frame-cache invariants hold through
> `Frames` (budget, least recently shown first, crops, one surface per frame shared by every scene).

**Files:**
- Create: `crates/pixfx-wasm/src/scene.rs`
- Modify: `crates/pixfx-wasm/src/lib.rs`, `crates/pixfx-wasm/Cargo.toml` (depend on `pixfx-scene`)
- Modify: `crates/pixfx-wasm/src/squash.rs` (velocity from movement, below)
- Modify: `crates/pixfx-wasm/src/lib.rs` (export `palettePresets()`, `outlinePresets()`)
- Test: `crates/pixfx-scene/tests/scene.rs`, `crates/pixfx-wasm/tests/scene.rs` (native, through the recording shell)

**Interfaces:**
- Consumes: `pixfx_scene::{Scene, Shell, Pass, Rect, StageEvent}` (Task 2).
- Produces (JS, via wasm-bindgen):
  - `class Scene { constructor(scale: number); play(id: string, x: number, y: number, opts?: object): number;
    moveTo(handle, x, y): boolean; stop(handle): boolean; remove(handle): boolean;
    update(dt: number): number; draw(): DrawList; events(): StageEvent[]; setScale(scale: number): void;
    setFeel(on: boolean): void; free(): void }`
  - `DrawList`: `{ ops: DrawOp[] }` where
    `DrawOp = { op: "make", id: number, w: number, h: number, layer: "color" | "light" }
            | { op: "refill", id: number }
            | { op: "show", id: number, x: number, y: number, w: number, h: number, blend: "normal" | "add" }
            | { op: "free", id: number }`,
    in execution order; pixels for `make`/`refill` are read with `scene.rgba(id): Uint8ClampedArray` (a view into wasm
    memory, valid until the next `update`).
  - `palettePresets(): string[]`, `outlinePresets(): string[]` read from `pixfx_draw::palette::PRESETS` and
    `pixfx_draw::outline::PRESETS`.
  - `Squash.scaleMoved(dt: number, x: number, y: number, vx?: number, vy?: number): [number, number]`: velocity is
    `(vx, vy)` when given, else the distance from the last position over `dt`; `dt <= 0` holds the last shape.
- Inside: a `RecordingShell` (Rust, in `scene.rs`) implements `Shell` with `Surface = u32` ids and RGBA buffers, and
  turns each call into a `DrawOp`.

- [ ] **Step 1:** Write failing tests in `crates/pixfx-wasm/tests/scene.rs`:
  `draw_list_makes_then_shows` (first `draw` after `play` gives `make` before `show` for each layer, light shown with
  `add`); `stopped_effect_ignores_moves` (Review Focus 1: `moveTo` (40, 30), stop, `moveTo` (90, 90); every later `show`
  is at (40, 30)); `hit_stop_holds_frames` (Review Focus 2: `update(0)` gives no `refill`).
- [ ] **Step 2:** Run them. Expected: FAIL (no `scene` module).
- [ ] **Step 3:** Implement the `RecordingShell` and the JS `Scene` class. Following is `moveTo` each tick (as
  `Scene::move_to` does today); the shell stops calling it once the effect is stopped, and `Scene` ignores `move_to`
  on a stopping effect so a late call cannot move it.
- [ ] **Step 4:** Write `squash_scale_moved_holds_on_zero_dt` and `squash_scale_moved_uses_distance_without_velocity`
  (native tests on the Rust function behind `scaleMoved`). Run; make them pass.
- [ ] **Step 5:** Run the gate; `wasm-pack build crates/pixfx-wasm --target web` succeeds.
- [ ] **Step 6:** Commit `wasm: a Scene for JS shells that returns a draw list; squash velocity and presets from Rust`.

### Task 4: `packages/pixfx-phaser` becomes a pure shell

> **Shape as built.** The package follows the wasm draw list: `game.ts` holds the game-wide `Frames`, routes jobs
> to workers (one per effect for its life) and painted frames back; `stage.ts` holds one Phaser scene's `Scene`;
> `surfaces.ts` makes, refills and frees surfaces; `phaser-gfx.ts` puts every surface in a few WebGL atlas pages
> (`texSubImage2D`, one batch per layer) or a canvas texture on the Canvas renderer. `pixels.ts`, `layout.ts`,
> `lifecycle.ts`, `effects.ts`, `runner.ts`, `frames.ts`, `layer.ts` and the old pool and backends are gone, and so
> are layer groups (`createLayer`): no game needs them, and re-merging a region whenever one effect in it changes
> costs more than one atlas batch per layer. `Frames.loadBaked` replaces `bakedFrames`. `pixfx-scene` keeps one frame
> in flight per layer of an effect, so a slow worker never builds a backlog.

**Files:**
- Delete: `packages/pixfx-phaser/src/{pixels,layout,lifecycle,effects,runner,frames}.ts` and their tests, except
  `tests/frames.test.ts`, which is kept and rewritten against the draw list (its cases are the fence for the move)
- Modify: `packages/pixfx-phaser/src/layer.ts` (executes a `DrawList` into Phaser textures and images; no compositing),
  `effect.ts` (a handle on a `Scene` handle), `plugin.ts` (one wasm `Scene` per Phaser scene, `update(dt)` then
  `draw()` each tick), `sprite-feel.ts` (calls `Squash.scaleMoved`; no velocity maths), `options.ts`
  (`PALETTE_PRESETS` from `palettePresets()`), `engine.ts` (the `Scene` type), `pool.ts`/`worker*.ts`/`protocol.ts`
  (the worker runs a `Scene` and posts `DrawList` plus transferred pixel buffers; the main thread executes it)
- Modify: `packages/pixfx-phaser/README.md`

**Interfaces:**
- Consumes: the JS `Scene`, `DrawList`, `DrawOp`, `palettePresets`, `Squash.scaleMoved` (Task 3).
- Produces: the same public Phaser API as today (`scene.pixfx.play(id, x, y, opts)`, follow, stop, `complete` and
  `destroy` events, layers, `scene.pixfx.feel`, `scene.pixfx.squash(sprite)`): games see no change.

- [ ] **Step 1:** Record `bun run perf` on `main`.
- [ ] **Step 2:** Run the existing package tests as the behaviour fence: `bun run check`. Expected: PASS before any
  change; they must pass unchanged after, except tests of deleted modules, whose behaviour is now covered by the
  Rust tests named in Task 3.
- [ ] **Step 3:** Write `shutdown_mid_effect_frees_everything` (Review Focus 5): play three effects, shut the Phaser
  scene down mid-play, assert `stats()` reports 0 textures and 0 wasm instances. Run; FAIL if it leaks.
- [ ] **Step 4:** Move the TS frame cache (`frames.ts`: 64 MiB, least recently shown dropped first, frames cropped by
  `litBox`, one texture shared by every image showing a frame via `GfxImage.setTexture`, fading effects keep their own
  textures) onto the `Scene` frame store: the same semantics, decided in Rust, expressed as `make`/`show`/`free` ops
  with shared ids. `stats().textures` keeps counting only scene-owned textures. A `loadBaked` that fills the cache
  from a bake becomes `Scene.load` behind the wasm `Scene`.
- [ ] **Step 5:** Rewrite the files as listed. A `DrawOp` maps to: `make` → `scene.textures.createCanvas` (or a
  pooled texture) sized `w × h`; `refill` → put `rgba(id)` into it; `show` → place the image at `(x, y)`, size `(w, h)`,
  blend `NORMAL` or `ADD`, nearest-neighbour; `free` → destroy texture and image.
- [ ] **Step 6:** `bun run check` (including `tests/frames.test.ts`) and the Step 3 test pass; `bun run perf:check` passes.
- [ ] **Step 7:** `rg -n 'compositeColor|compositeLight|Math.floor\(.*fps|PALETTE_PRESETS = \[' packages/pixfx-phaser/src`
  returns nothing.
- [ ] **Step 8:** Record `bun run perf` after; compare with Step 1.
- [ ] **Step 9:** Commit `phaser: a pure shell over the wasm Scene; placement, timing, compositing and presets come from Rust`
  with both perf results in the message.

### Task 5: `packages/pixfx-pixi`

> **Shape as built.** The host-neutral half of the Phaser package moved into `packages/pixfx-web` (wasm, workers,
> game-wide frames and surfaces, `Stage`, `PixfxEffect`, `PixfxFeel`, on eventemitter3), and `pixfx-pixi` is two
> files over it: `pixfx.ts` (`createPixfx(app, { container, camera })`, the ticker step, shake and flash) and
> `pixi-gfx.ts` (WebGL atlas, buffer textures, sprites). Its tests drive real Pixi objects with the real wasm;
> `bun run check:browser` checks the demo in Chrome. Phaser switches to `pixfx-web` after Task 6.

**Files:**
- Create: `packages/pixfx-pixi/{package.json,tsconfig.json,eslint.config.ts,README.md}`, `src/{index,plugin,layer,engine}.ts`,
  `tests/shell.test.ts`, `demo/index.html`
- Modify: `README.md` (crates table), `site/` docs page for Pixi if `site/docs/phaser.html` has a sibling index to
  list it in

**Interfaces:**
- Consumes: the JS `Scene`, `DrawList`, `DrawOp` (Task 3); the same wasm build as Phaser (`build:wasm` into
  `packages/pixfx-pixi/wasm`).
- Produces: `createPixfx(app: Application, opts?: { scale?: number }): Pixfx` with `play(id, x, y, opts) -> PixfxEffect`,
  `effect.follow(x, y)`, `effect.stop()`, `effect.on("complete" | "destroy")`, `feel` (applies shake to a container the
  game passes), `destroy()`. Same names and semantics as the Phaser plugin where Pixi has the same concept.

- [ ] **Step 1:** Write `tests/shell.test.ts`: executing a recorded `DrawList` against a Pixi `Application` creates
  one `Texture` per `make`, places sprites per `show` with `blendMode` `"normal"`/`"add"` and `scaleMode`
  `"nearest"`, and destroys them per `free`; after `destroy()` the stage has no children (Review Focus 5).
- [ ] **Step 2:** Run `bun test`. Expected: FAIL (no package).
- [ ] **Step 3:** Implement the package as a shell only: it reads `DrawOp`s and calls Pixi; no placement, timing or
  pixel maths.
- [ ] **Step 4:** `bun run check` passes; the demo plays fire/campfire and an explosion with shake in Chrome.
- [ ] **Step 5:** Commit `pixi: a Pixi 8 shell over the wasm Scene`.

### Task 6: Docs say where things are

**Files:**
- Modify: `AGENTS.md` (empty the "Out of place today" list; the layer table names crates without "target" or "today"),
  `README.md` (crates table: `pixfx-draw`, `pixfx-scene`, `packages/pixfx-pixi`), `CHANGELOG.md` (Unreleased)

- [ ] **Step 1:** Update the three files to the final state.
- [ ] **Step 2:** `rg -n 'today in|\(target\)|Out of place' AGENTS.md` returns nothing; `rg -n 'pixfx-draw|pixfx-scene|pixfx-pixi' README.md`
  finds each.
- [ ] **Step 3:** Commit `docs: crates and boundary after the render engine split`.
