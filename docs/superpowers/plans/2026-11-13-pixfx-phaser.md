# pixfx-phaser Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A TypeScript package any Phaser 4 game imports to play pixfx effects in its scenes, driven by the `pixfx-wasm` engine.

**Architecture:** The engine gains an optional glow margin. The package splits into a pure core (options, clock, pixel conversion, effect lifecycle, layer layout — tested with bun against a fake engine) and a Phaser shell (a Scene plugin that owns textures and images, ticks effects on the scene clock and cleans up). A Vite demo proves it in the browser on the WebGL and Canvas renderers.

**Tech Stack:** Rust + wasm-bindgen (engine), TypeScript strict, zod, Phaser `^4.2.1`, bun, Vite, ego-browser.

**Spec:** `docs/superpowers/specs/2026-11-13-pixfx-phaser-design.md` (approved 2026-11-13). Read it before any task.

## Global Constraints

- TypeScript `strict: true`; zod at every boundary where untyped data enters (play options, effect metadata from wasm).
- typescript-eslint as errors: `no-explicit-any`, `no-unsafe-*` (all), `no-non-null-assertion`, `consistent-type-assertions: { assertionStyle: "never" }`, `ban-ts-comment`.
- Package manager and test runner `bun`; demo built with Vite and no Vite plugins.
- `phaser` is a peer dependency `^4.2.1` (dev dependency at `4.2.1` for the demo and types).
- Rust: clippy `-D warnings`, `cargo fmt`, no unwrap/expect/panic, libm-only transcendental math (repo `clippy.toml`).
- Every cargo command runs with `CARGO_BUILD_BUILD_DIR=<workspace dir>/target/build-dir` (the user's global config points build-dir at an unmounted volume).
- Light blend rule: dst.rgb += light.rgb, dst alpha unchanged; unlit light pixels upload as `0,0,0,0`; lit as `r,g,b,255`. Colour words are ABGR, light words xBGR.
- Glow margin = `max(w, h) / 2` of `meta.size` (integer division).
- Effect dt per frame = scene `update` delta (ms) / 1000 × `scene.time.timeScale`; values that are not finite or are < 0 become 0. The engine clamps a step at 0.25 s.
- Positions snap to whole effect pixels (`Math.round` of world position / object scale).
- Commits end with `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`. Never push.

## Review Focus

1. **Unknown effect id** in `play` → throws `Error` naming the id; creates no texture or engine (stats unchanged). Test in Task 2 (core) and Task 3 (plugin).
2. **Scene restart / shutdown with effects playing** → every texture, image and `Fx` freed; `stats()` back to `{engines: 0, textures: 0, images: 0}`. Test in Task 3 (browser assertion in Task 5).
3. **Followed object destroyed mid-play** → effect stops (fades) at the object's last position and later emits `complete`. Test in Task 2 (core) and Task 5 (browser).
4. **`timeScale` 0, negative or NaN; scene paused** → the effect does not advance (frame unchanged). Test in Task 2 (`stepSeconds`) and Task 5 (browser: paused frame unchanged).
5. **`play` before `loadPixfx()` resolved** → throws `Error("pixfx: call await loadPixfx() before play()")`. Test in Task 3.

---

### Task 1: Engine glow margin

**Files:**
- Modify: `crates/pixfx-core/src/effect.rs` (add `Meta::glow_margin`)
- Modify: `crates/pixfx-canvas/src/stage.rs:12-16` (use the core method; delete the local `margin` fn and update its callers)
- Modify: `crates/pixfx-wasm/src/options.rs` (parse `margin`), `crates/pixfx-wasm/src/lib.rs` (`Fx::new` passes it; `meta()` gains `glowMargin`; `originX/originY` include the margin)
- Test: `crates/pixfx-core` unit test, `crates/pixfx-wasm/tests/web.rs`

**Interfaces:**
- Produces: `impl Meta { pub fn glow_margin(&self) -> u32 }` = `max(size.0, size.1) / 2`.
- Produces (JS): `meta(id).glowMargin: number`; `new Fx(id, { margin?: number })` — non-negative integer, default 0; `fx.width()/height()` = `size + 2·margin`; `fx.originX()/originY()` = `origin + margin`.

- [ ] **Step 1: Write failing tests.** Core: `glow_margin_is_half_the_longer_side` — campfire's meta gives `max(w,h)/2`. Wasm (`tests/web.rs`): `margin_option_grows_the_canvas` (`new Fx("fire/campfire", {margin: m})` with `m = meta.glowMargin` → width/height = size + 2m, origin = meta.origin + m); `margin_frames_match_player` (`color()`/`light()` after 3 × `update(1/30)` equal a `Player::new(effect, opts, m)` stepped the same way); `bad_margin_errors` (`-1`, `1.5`, `"x"` → error whose message contains `margin`); `default_margin_unchanged` (no `margin` → same words as before; the existing `frames_match_render_frames` keeps passing).
- [ ] **Step 2:** `wasm-pack test --node crates/pixfx-wasm` and `cargo test -p pixfx-core` → FAIL.
- [ ] **Step 3: Implement.** `margin` is parsed in `parse_options` beside `seed` (same non-negative-integer check) but is not an effect option: return it alongside `Options` (e.g. `parse_options -> (Options, u32)`), so `extra` never sees it.
- [ ] **Step 4:** both test commands PASS; `cargo test --workspace`, clippy, fmt clean; `cargo run -q --release -p pixfx-cli -- parity fire/<e>` with `PIXELFX=/tmp/fxref/pixelfx` still 0.00 % for candle/campfire/brazier (re-clone pixelfx `044e12a` there if missing).
- [ ] **Step 5: Commit** `wasm: optional glow margin on Fx; core owns the margin formula`.

### Task 2: Package scaffold and pure core

**Files (all under `packages/pixfx-phaser/`):**
- Create: `package.json`, `tsconfig.json`, `eslint.config.js`, `.gitignore` (`wasm/`, `node_modules/`, `dist/`)
- Create: `src/engine.ts`, `src/options.ts`, `src/clock.ts`, `src/pixels.ts`, `src/lifecycle.ts`, `src/layout.ts`
- Test: `tests/options.test.ts`, `tests/clock.test.ts`, `tests/pixels.test.ts`, `tests/lifecycle.test.ts`, `tests/layout.test.ts`, `tests/fake-engine.ts`

**Interfaces (Produces):**
- `package.json` scripts: `build:wasm` = `wasm-pack build ../../crates/pixfx-wasm --target web --out-dir ../../packages/pixfx-phaser/wasm` (with the build-dir env), `test` = `bun test`, `typecheck` = `tsc --noEmit`, `lint` = `eslint .`, `check` = typecheck + lint + test.
- `engine.ts`: `interface FxLike { update(dt: number): void; stop(): void; done(): boolean; t(): number; width(): number; height(): number; originX(): number; originY(): number; draw(layer: "back" | "front"): void; color(): Uint32Array; light(): Uint32Array; free(): void }`; `interface Engine { effects(): readonly string[]; meta(id: string): EffectMeta; create(id: string, opts: EngineOptions): FxLike }`; `EffectMeta` = zod-parsed `{id, name, size: [w,h], origin: [x,y], duration, loop, fps, seed, glowMargin}`; `loadPixfx(): Promise<Engine>` (real wasm, from `../wasm/pixfx_wasm.js`); `getEngine(): Engine` throws `Error("pixfx: call await loadPixfx() before play()")` if not loaded.
- `options.ts`: `PlayOptions` zod schema: `seed?`, `flip?`, `angle?`, `light?`, `palette?: string[]`, `scrollFactor?: number` (default 1), `depth?: {back: number, front: number}`, plus passthrough numeric/array extras; `parsePlayOptions(input: unknown): PlayOptions` throws `Error` naming the bad field; `engineOptions(p: PlayOptions, margin: number): EngineOptions` strips `scrollFactor`/`depth`.
- `clock.ts`: `stepSeconds(deltaMs: number, timeScale: number): number`.
- `pixels.ts`: `colorToRgba(words: Uint32Array, out: Uint8ClampedArray): boolean` (true if any alpha ≠ 0); `lightToRgba(words: Uint32Array, out: Uint8ClampedArray): boolean` (lit → `r,g,b,255`, unlit → `0,0,0,0`); `snap(v: number): number`.
- `lifecycle.ts`: `class EffectCore` — `constructor(fx: FxLike, x: number, y: number)`; `state: "playing" | "stopping" | "done" | "destroyed"`; `follow(target: {x: number; y: number; active: boolean} | null, dx: number, dy: number)`; `tick(dt: number): "running" | "completed"` (moves to follow target if active, else stops once; steps fx; returns `"completed"` exactly once when `fx.done()`); `stop()` (no-op unless playing); `destroy()` (frees fx once); `topLeft(): {x: number; y: number}` = snapped position − origin.
- `layout.ts`: `clipRect(bounds: Rect, effect: Rect): Rect | null` — intersection used by layer mode.

- [ ] **Step 1: Write failing tests** (bun, with `tests/fake-engine.ts` implementing `Engine`/`FxLike` deterministically and counting live/freed instances):
  - `stepSeconds(16, 1) === 0.016`; `stepSeconds(16, 0.5) === 0.008`; `stepSeconds(16, 0)`, `(16, -1)`, `(16, NaN)`, `(NaN, 1)` all `=== 0`.
  - `lightToRgba` of `[0x00000000, 0x00102030]` → `[0,0,0,0, 0x30,0x20,0x10,255]`; `colorToRgba` of `[0x80102030]` → `[0x30,0x20,0x10,0x80]` and returns true; all-zero → false.
  - `parsePlayOptions({seed: -1})`, `({palette: [1]})`, `({scrollFactor: "x"})` throw with the field name; `parsePlayOptions(undefined)` → defaults (`scrollFactor: 1`).
  - `EffectCore`: completes once (second tick after done returns `"running"` with state `"done"`, never `"completed"` twice); `stop()` on done does nothing; `destroy()` frees exactly once (fake counts 0 live); follow tracks target + offset each tick; target with `active: false` → state `"stopping"` and position frozen at last seen; `topLeft()` snaps `10.6 → 11`.
  - unknown id: fake `create` throws → no instance counted.
  - `clipRect` full inside, partial, disjoint (`null`).
- [ ] **Step 2:** `bun test` → FAIL.
- [ ] **Step 3: Implement** the files above; `bun install` adds `zod`, `typescript`, `eslint`, `typescript-eslint`, `@types/bun`, `vite`, `phaser@4.2.1` (dev) with `phaser` peer `^4.2.1`.
- [ ] **Step 4:** `bun run check` PASS (tsc, eslint with the banned rules as errors, tests).
- [ ] **Step 5: Commit** `phaser: package scaffold and pure core`.

### Task 3: Phaser plugin and per-effect objects

**Files:**
- Create: `packages/pixfx-phaser/src/plugin.ts`, `src/effect.ts`, `src/index.ts`
- Test: `packages/pixfx-phaser/tests/plugin.test.ts` (pure bookkeeping via the fake engine and a fake texture/image factory)

**Interfaces:**
- Consumes: Task 2 (`Engine`, `getEngine`, `parsePlayOptions`, `engineOptions`, `stepSeconds`, `colorToRgba`, `lightToRgba`, `EffectCore`).
- Produces: `class PixfxPlugin extends Phaser.Plugins.ScenePlugin` registered by games as `{ key: "pixfx", plugin: PixfxPlugin, mapping: "pixfx" }`; `play(id: string, x: number, y: number, opts?: unknown): PixfxEffect`; `createLayer(cfg)` (Task 4); `stats(): {engines: number; textures: number; images: number}`.
- Produces: `class PixfxEffect extends Phaser.Events.EventEmitter` — `stop(): this`, `destroy(): void`, `follow(target: Phaser.GameObjects.GameObject & {x: number; y: number}, dx?: number, dy?: number): this`, `setDepths(back: number, front: number): this`, `setPosition(x, y): this`; events `"complete"` and `"destroy"`.
- Rendering: per effect four canvas textures: `back-color`, `back-light`, `front-color`, `front-light`, each `fx.width() × fx.height()`, `FilterMode.NEAREST`; light images `setBlendMode(Phaser.BlendModes.ADD)`; default depths back 0 / front 1 (or `opts.depth`); `scrollFactor` from options; uploads only when the fx stepped this frame.
- Scene time: listen to `Phaser.Scenes.Events.UPDATE` (gets `time, delta`); dt = `stepSeconds(delta, scene.time.timeScale)`. On `SHUTDOWN` and `DESTROY`: destroy every effect (textures removed from `scene.textures`, images destroyed, `fx.free()`).

- [ ] **Step 1: Write failing tests** with fakes: `play` of an unknown id throws and `stats()` stays `{0,0,0}`; one `play` → `{engines: 1, textures: 4, images: 4}`; completion → `"complete"` emitted once, then `stats()` `{0,0,0}`; `destroy()` emits `"destroy"` not `"complete"`; scene `shutdown` with 3 effects → `{0,0,0}`; `play` before `loadPixfx` → the exact Error from Global Constraints. (Keep Phaser-specific calls behind the small factory interface the test fakes.)
- [ ] **Step 2:** `bun test` → FAIL.  **Step 3:** implement.  **Step 4:** `bun run check` PASS.
- [ ] **Step 5: Commit** `phaser: scene plugin with per-effect objects`.

### Task 4: Scene-wide layer mode

**Files:**
- Create: `packages/pixfx-phaser/src/layer.ts`; Modify: `src/plugin.ts` (`createLayer`), `src/index.ts`
- Test: `packages/pixfx-phaser/tests/layer.test.ts`

**Interfaces:**
- Produces: `createLayer(cfg: {bounds: {x: number; y: number; width: number; height: number}; depths?: {back: number; front: number}; scrollFactor?: number}): PixfxLayer`; `class PixfxLayer` with `play(id, x, y, opts?) → PixfxEffect` (same API; its images are the layer's), `destroy()`. Four layer textures of `bounds` size; each tick clears them, then blits every member effect's back/front colour and light into its `clipRect`; effects outside bounds are clipped, partially outside drawn in part.
- [ ] **Step 1: Write failing tests:** two effects in one layer → `stats().textures` = 4 (not 8), `engines` = 2; an effect fully outside bounds draws nothing (fake blit records no writes) but still completes; layer `destroy()` → `{0,0,0}`.
- [ ] **Step 2–4:** FAIL → implement → `bun run check` PASS.
- [ ] **Step 5: Commit** `phaser: scene-wide layer mode`.

### Task 5: Demo and browser proof

**Files:**
- Create: `packages/pixfx-phaser/demo/index.html`, `demo/main.ts`, `packages/pixfx-phaser/README.md`; Modify: root `README.md`, `CHANGELOG.md` (one line each).

**Demo contents:** Phaser 4 game, `pixelArt: true`, zoom 4, renderer chosen by `?renderer=webgl|canvas`; dark ground; candle, campfire, brazier (per-effect); a coloured rectangle "character" walking back and forth through the campfire with depth between the campfire's back and front; a candle following a second moving sprite; keys: `P` pause/resume scene, `S` toggle `timeScale` 1 ↔ 0.25, `X` stop the campfire (fades, then `complete` logged), `D` destroy the sprite the candle follows, `R` restart the scene; a scene-wide layer at the right holding two braziers. `window.pixfxDemo = { scene, stats: () => plugin.stats(), events: string[] }` for checks.

- [ ] **Step 1:** `bun run build:wasm`; `bunx vite build demo` succeeds and `dist` contains the `.wasm` (no Vite plugin).
- [ ] **Step 2: Browser proof** with ego-browser (`ego-browser skill` first), on `?renderer=webgl` and `?renderer=canvas`; record numbers in the commit message:
  - two screenshots 0.5 s apart differ in each effect's region; after `P`, two screenshots are identical in those regions;
  - at zoom 4 every lit effect pixel is a uniform 4×4 block (screenshot read at CSS pixels; account for Display-P3 tagging by comparing uniformity, not absolute colours);
  - ground under the campfire brighter than ground far away;
  - where the walking character overlaps the campfire, front-layer pixels cover it;
  - after `D`, the candle stays at the sprite's last position, fades and logs `complete`;
  - after `X`, `events` contains `complete` once and `stats().engines` drops by 1; after `R`, `stats()` equals its value right after the first load;
  - WebGL only: read the campfire's `front-color` texture back (`renderer.snapshotArea` or a GL readPixels helper) and compare to a fresh `Fx` (same seed, same steps, `margin = glowMargin`) `colorToRgba` output — equal bytes.
- [ ] **Step 3:** README for the package: install (local link), `loadPixfx`, plugin registration, `play` / options / events / layer mode, the transparent-render-target limitation.
- [ ] **Step 4:** `bun run check` and `cargo test --workspace` PASS.
- [ ] **Step 5: Commit** `phaser: demo, browser proof, docs`.
