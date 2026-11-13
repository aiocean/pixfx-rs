# Measure, cull and hit-stop Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Know what a frame of effects costs on a mid-range phone, hand the engine's hot spots to their owner with
numbers, draw no effect the player cannot see, and let a game keep an effect playing through a hit-stop.

**Architecture:** Measurement first: the Chrome perf runner (`packages/pixfx-phaser/perf`) must finish its sweep, and a
native `pixfx bench` splits engine cost per effect. Culling and the hit-stop exemption are decisions, so they live in
the host-neutral `Scene` (today `crates/pixfx-canvas/src/stage.rs`; `crates/pixfx-scene/src/scene.rs` after Task 2 of
`2026-11-14-render-engine-boundary.md`); shells only pass the view in.

**Tech Stack:** Rust (nightly in `rust-toolchain.toml`), wasm-bindgen, wasm-pack, Phaser 4.2.1, bun, Chrome over CDP.

**Spec:** `docs/superpowers/specs/2026-11-14-render-engine-vision-design.md`: "Fast enough for many sprites and effects
at 60 fps on a mid-range phone, shown by measurement"; anti-goal "Logic in a shell or binding".

## Evidence (measured 2026-11-14, M-series Mac, Chrome 157)

| What | Result | Source |
|---|---|---|
| 50 mixed fires, Phaser, 1× CPU, main thread | frame 98 ms; engine (wasm) 75.04, texture upload 4.61, pixfx-phaser 2.07, phaser 1.79 ms | `bun run perf` |
| Same, default worker pool (4) | 20 fires 60 fps; 50 fires 14 fps; shared layer 50 fires 52 fps | `bun run perf` |
| 4× CPU (mid-range phone) | not measured: runner timed out on `window.pixfxStress !== undefined` opening the third worker-pool scenario | `perf/run.ts:61-67` |
| Native, 50 instances, 60 Hz display, draw on each new clock frame | update ≈ 0 ms; candle (132×152 px) draw 2.48 + rgba 0.35 ms; campfire (256×256) 28.52 + 2.49; brazier (260×296) 28.08 + 2.50 | temporary example, deleted |
| Where campfire draw time goes (macOS `sample`, 7266 samples) | `Painter::add_at` 1982, `Painter::glow` 615, `fmod` 512, `blend_px` 473, libm `pow` 362, `campfire::log` 310, `Painter::clipped` 272 | `painter.rs:278-288`, `:662-702` |
| wasm used by the perf run | built 00:42:39; `Cargo.toml` (now `lto = "fat"`, `codegen-units = 1`) last modified 01:32:34 | `stat` |
| Stress seeds | every effect gets its own seed (`seed: 1 + this.spawned`), so frames are never shared | `demo/stress.ts:140` |
| Culling today | none: `Scene::update` and `draw_layer` visit every entry (`stage.rs:366-367`, `:423-466`); Phaser 4.2.1 `willRender` has no camera bounds test (`GameObject.js:707-712`) | code |
| Baseline (Task 3): main 930dd2c, wasm with `lto = "fat"`, load average 29 to 46 (other sessions' builds), so noisy | Phaser 1× main thread: 10 fires 57 fps, 20 fires 22 fps; 100 fires engine 144.23 ms, texture upload 100.87 ms. 1× worker pool: shared layer holds 60 fps to 100 fires; 100 fires upload 121.79 ms. 4× main thread: 10 fires 10.6 fps; 100 fires engine 412.33, upload 135.03 ms. 4× worker pool: objects 60 fps up to 25, 30 fps up to 50; 100 fires upload 25.43 ms. Objects mode reports 0 textures while effects show. Native `pixfx bench` (50 each, 10 s, 60 Hz): update 0.01 ms; candle 132×152 draw 3.08 / rgba 1.11 ms; campfire 256×256 23.41 / 3.63; brazier 260×296 24.07 / 4.27 | `bun run perf`, `pixfx bench`; raw JSON `/var/folders/.../pixfx-perf-1794599012610.json` |
| Hit-stop today | every effect freezes (`stage.rs:364-367`, `feel.rs:156-159`, `effects.ts:103-106`) | code |

What this means:
- Drawing is the cost and simulation is free. Skipping the draw of an unseen effect saves almost all of its cost while
  its clock and cues keep running.
- Glow, the light the painter adds per pixel in f64, is about a third of a big fire's draw. Making the painter faster is
  the largest win for every game, including single-screen ones where culling saves nothing. That hot path belongs to
  session a04a260 and is out of this plan (see Handed to other owners).
- Texture upload is under 5% of the frame, so a texture atlas is not worth doing now.

## Handed to other owners (not tasks here)

- **Painter speed** (`add_at`, `glow`, `fmod`, `pow` above): session a04a260, sent 2026-11-14 with the numbers.
  Any change must keep every `frame_hash`.
- **Frame sharing in Phaser:** a04a260's rewrite (answer of 2026-11-14) gives Phaser a game-wide `FrameCache` in
  `packages/pixfx-phaser/src/frames.ts` (64 MiB, least recently shown dropped first) keyed by the Rust frame key
  (`Fx.shown()`, `Fx.clockFrame()`); frames are sent once, cropped to their lit box (`Fx.litBox`), and textures are
  shared through `GfxImage.setTexture`. It does not move Phaser onto the Rust `Scene`; that stays in the boundary
  plan's Tasks 3 and 4. It also lands `pixfx bake <id>` (cropped PNG atlas and JSON keyed by the frame key).
- **wasm build profile and size:** session fdd31fa2.

## Before starting

- Several sessions share this working tree: commit only your own hunks through a temporary index; never
  `git add -A`, stash or revert another session's change.
- Before editing `Scene`, check where it lives: `ls crates/pixfx-scene/src/scene.rs crates/pixfx-canvas/src/stage.rs`.
  Edit whichever exists. If both or neither exist, the boundary plan's Task 2 is mid-flight: wait.
- Before Tasks 4 to 7, message a04a260: "editing `Scene::update` and `Scene::draw_layer` (view test, hit-stop
  exemption); no change in `Player` or the painter". Wait for the reply.
- A crate that depends on `pixfx-effects` uses `default-features = false, features = ["all"]`.
- `bun run perf` and `perf:check` launch Chrome with a debug port: run them outside the sandbox.

## Global Constraints

- Every effect's `frame_hash` is unchanged: `cargo test --workspace` (golden hashes) passes before and after each task.
- Gate after every task: `cargo fmt --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace`;
  a task touching `packages/pixfx-phaser` also runs `(cd packages/pixfx-phaser && bun run check)`.
- Workspace lints stay: `unsafe_code = "forbid"`; clippy `unwrap_used`, `expect_used`, `panic` deny.
- Defaults do not change: a `Scene` with no view draws everything, and an unmarked effect freezes in a hit-stop. The
  existing tests in `crates/pixfx-canvas/tests/stage.rs` pass unchanged.
- Culling skips drawing only: every entry is still updated, fires its cues and ends on time.
- Perf numbers go in the commit message of the task that measured them, as the earlier perf commits do.

## Review Focus

1. A perf sweep that runs while another session edits `packages/pixfx-phaser/src` reloads the page through vite and
   breaks the run; the runner must say so instead of timing out silently. Test in Task 1.
2. An effect that leaves the view and comes back shows the frame for its current time, including a fading effect
   whose own surface was skipped while culled. Test in Task 4.
3. A hit outside the view still freezes the game and shakes the screen. Test in Task 4.
4. A glow reaching into the view from an effect whose origin is outside it is drawn: the test uses the full rect
   (canvas plus glow margin; a campfire's frame is 256×256 for a much smaller fire). Test in Task 4.
5. The own loop's shake moves the canvas by up to the feel's `shake` (16 game pixels): an effect just past the canvas
   edge is still drawn when the shake can bring it in. Test in Task 5.

---

## Phase A: measure

### Task 1: `bun run perf` finishes the 1× and 4× sweep

**Files:** `packages/pixfx-phaser/perf/run.ts`, `packages/pixfx-phaser/perf/cdp.ts`, `packages/pixfx-phaser/demo/stress.ts`
(whichever holds the cause)

- [ ] **Step 1:** Use `superpowers:systematic-debugging`. Reproduce: `PIXFX_PERF_COUNTS=20,50,100 bun run perf`. The
  2026-11-14 run failed in `open()` (`run.ts:64-67`, 20 s wait for `window.pixfxStress`) on the third worker-pool
  scenario, right after the 100-effect shared-layer sweep.
- [ ] **Step 2:** Rule out each hypothesis with its check, recording the result:
  - vite reloaded or failed a module because another session edited `src/` during the run: subscribe to
    `Runtime.exceptionThrown` and `Log.entryAdded` in `cdp.ts` and print them when `waitFor` times out;
  - the previous page's worker pool keeps the browser busy past 20 s: time `open()` per scenario and print it;
  - the vite server died: on timeout, `fetch(ROOT + "/stress.html")` and print the status.
- [ ] **Step 3:** Fix the cause. Keep the diagnostics from Step 2 that print on timeout (Review Focus 1): a timeout
  names the last page error or the server status.
- [ ] **Step 4:** `bun run perf` exits 0 and prints the 4× tables; `bun run check` passes.
- [ ] **Step 5:** Commit `perf: <cause>; the sweep reaches 4× and a timeout says why`.

### Task 2: `pixfx bench`, engine cost per effect natively

**Files:**
- Create: `crates/pixfx-cli/src/bench.rs`
- Modify: `crates/pixfx-cli/src/lib.rs` (`pub mod bench;`), `crates/pixfx-cli/src/main.rs` (a `bench` command beside
  `parity`, `sheet`, `render`; the file has another session's uncommitted change, so commit only your hunks), `README.md`
  (the CLI section)
- Test: `crates/pixfx-cli/tests/bench.rs`

**Interfaces:**
- Produces:
  - `pub struct BenchConfig { pub ids: Vec<String>, pub n: usize, pub secs: f64, pub hz: f64 }`.
  - `pub struct BenchRow { pub id: String, pub w: u32, pub h: u32, pub draws: u64, pub draw_ms: f64, pub rgba_ms: f64 }`:
    one effect id's frame size, layer pairs drawn, and mean milliseconds per display frame.
  - `pub struct BenchReport { pub frames: u64, pub update_ms: f64, pub rows: Vec<BenchRow> }`.
  - `pub fn run(cfg: &BenchConfig) -> Result<BenchReport, String>`: `n` instances per id, seeds 1..=n, built as
    `live_player` does (`Player::new(fx, opts, fx.meta().glow_margin())`); each display frame updates every
    instance by `1 / hz`, then draws both layers and converts them (`color_rgba`, `light_rgba`) for each instance
    whose `clock_frame()` changed. Unknown id or `n == 0` is an `Err`.
  - CLI: `pixfx bench [--ids fire/candle,fire/campfire,fire/brazier] [--n 50] [--secs 10] [--hz 60]` prints the
    update time and one table row per id: `| id | w×h | draws | draw ms | rgba ms |`.

- [ ] **Step 1:** Write failing tests:
  - `each_instance_draws_once_per_new_frame`: `ids = ["fire/candle"]`, `n = 3`, `secs = 1.0`, `hz = 60.0`;
    `frames == 60` and the row's `draws == 3 * 31` (candle `fps` is 30: frame 0 plus 30 more in 1 s; read `fps` from
    the effect's meta in the test, do not hard-code it).
  - `rows_report_the_frame_size_with_its_glow_margin`: the candle row's `(w, h)` equals
    `Player::new(..).frame()` size for the same effect and margin.
  - `an_unknown_effect_is_an_error_naming_it`: `ids = ["fire/nope"]` → `Err(e)`, `e.contains("fire/nope")`.
  - `no_instances_is_an_error`: `n = 0` → `Err`.
- [ ] **Step 2:** `cargo test -p pixfx-cli --test bench`: FAIL (no module).
- [ ] **Step 3:** Implement with `std::time::Instant` around the update, draw and rgba phases.
- [ ] **Step 4:** Tests PASS. `cargo run --release -p pixfx-cli -- bench` prints rows near the Evidence table
  (campfire draw near 28 ms for 50).
- [ ] **Step 5:** Gate, then commit `cli: pixfx bench reports engine cost per effect`.

### Task 3: the baseline, on a wasm built with the current release profile

Only measurement; no code change. It is the "before" every later speed change (here and a04a260's) compares with.

- [ ] **Step 1:** Tell fdd31fa2 and a04a260 you are rebuilding `packages/pixfx-phaser/wasm` (a gitignored build
  output they may be measuring with); wait for no objection.
- [ ] **Step 2:** `(cd packages/pixfx-phaser && bun run build:wasm)`; `stat` the new `pixfx_wasm_bg.wasm` is newer than
  `Cargo.toml`.
- [ ] **Step 3:** `bun run perf` (full sweep, 1× and 4×) and `cargo run --release -p pixfx-cli -- bench`.
- [ ] **Step 4:** Send both outputs to a04a260 as the baseline for painter work. Record them in the plan's Evidence
  table (this file) under a "Baseline" row and commit the plan with only that change:
  `docs: perf baseline on an LTO wasm build`.

## Phase B: draw only what is seen

### Task 4: `Scene` draws only what the view shows

**Files:**
- Modify: `crates/pixfx-canvas/src/stage.rs` (or `crates/pixfx-scene/src/scene.rs`)
- Test: `crates/pixfx-canvas/tests/stage.rs` (or `crates/pixfx-scene/tests/scene.rs`)

**Interfaces:**
- Produces:
  - `pub struct View { pub x: f64, pub y: f64, pub w: f64, pub h: f64 }`: game pixels (world units for a host with a
    camera).
  - `Scene::set_view(&mut self, view: Option<View>)`; `None` (the default) draws everything.
  - `fn rect_in_view(rect: Rect, view: View, scale: NonZeroU32) -> bool` (private, pure): true when `rect` (canvas
    pixels, as `Scene::rect` returns) overlaps `view` times `scale`; rectangles that only touch do not overlap.
  - `draw_layer` skips an entry outside the view before `e.player.shown(..)`: no `make`, `refill` or `show`. Its own
    surface keeps its `drawn` key, so it redraws when it returns on a new clock frame or placement.

- [ ] **Step 1:** Write failing tests (use the existing `Recorder`, `draw_once`, `player`, `HITTER` helpers):
  - `an_effect_outside_the_view_is_neither_drawn_nor_shown`: scale 1, view `{0, 0, 100, 100}`, candles at (50, 50)
    and (1000, 1000); `update(0.1)`; `draw_once`: `shown.len() == 2`, every pass's `handle` is the inside one.
  - `a_glow_reaching_into_the_view_is_drawn` (Review Focus 4): put a candle whose origin is outside the view so that
    `scene.rect(handle)` overlaps it by 1 pixel: shown. Move it 1 pixel further so the rect only touches: not shown.
  - `an_effect_coming_back_shows_its_current_frame` (Review Focus 2): scenes A (view) and B (no view) with the same
    candle and the same `update` steps; in A move it out for 1 s then back, in B leave it. After the return the frames
    shown by A and B are equal. Repeat with the candle stopped before it leaves (fading, own surface).
  - `a_hit_outside_the_view_still_freezes_and_shakes` (Review Focus 3): `HITTER` at (1000, 1000), view
    `{0, 0, 100, 100}`: `feel().frozen()` within 0.2 s and `feel().trauma() > 0` after it.
- [ ] **Step 2:** Run: FAIL (`set_view` missing).
- [ ] **Step 3:** Implement `View`, `set_view`, `rect_in_view` and the skip.
- [ ] **Step 4:** New and existing stage tests PASS.
- [ ] **Step 5:** Gate, then commit `scene: a view; effects outside it are not drawn and keep their time and cues`.

### Task 5: the Canvas stage culls to its canvas; a host loop can set a view

**Files:**
- Modify: `crates/pixfx-canvas/src/lib.rs` (`Renderer::tick`, `Stage` bindings), `crates/pixfx-canvas/demo/`
  (a world-wider-than-canvas mode), `README.md` (the `pixfx-canvas` row: `setView`, `clearView`)
- Test: `crates/pixfx-canvas/tests/stage.rs`

**Interfaces:**
- Consumes: `View`, `Scene::set_view` (Task 4).
- Produces:
  - `pub fn own_view(canvas_w: u32, canvas_h: u32, scale: NonZeroU32, shake_px: f64) -> View` in `stage.rs` (pure,
    so it tests natively): the canvas in game pixels grown by `shake_px` on every side.
  - `Renderer::tick` calls `scene.set_view(Some(own_view(..)))` before drawing, with `shake_px` read from the feel's
    spec `shake` field (16 by default; do not repeat the number). With the feel off, `shake_px` is 0.
  - JS: `stage.setView(x, y, w, h)` (game pixels; a non-finite value throws) and `stage.clearView()`. A host loop
    that never calls `setView` draws everything, as today.

- [ ] **Step 1:** Failing test `the_own_view_reaches_as_far_as_the_shake` (Review Focus 5): a 320×180 canvas at scale 2
  with shake 16 gives `View { x: -16.0, y: -16.0, w: 192.0, h: 122.0 }`; shake 0 gives `{0, 0, 160, 90}`.
- [ ] **Step 2:** Run: FAIL. Implement `own_view`, call it in `tick`, add `setView` and `clearView`. Run: PASS.
- [ ] **Step 3:** `wasm-pack build crates/pixfx-canvas --target web --out-dir demo/pkg` succeeds.
- [ ] **Step 4:** In the demo, 100 fires over a world 4× the canvas width with the view panning; measure the
  `requestAnimationFrame` frame time with the own loop (culled) and with a host loop that does not call `setView`
  (not culled), through `ego-browser`. Expected: culled costs about a quarter. Record both in the commit message.
- [ ] **Step 5:** Gate, then commit `canvas: the own loop draws only what its canvas shows; setView for host loops`.

### Task 6: Phaser culls to its camera

Start only when Phaser drives a wasm `Scene` (the boundary plan's Task 4, or a04a260's rewrite if it does the same):
`packages/pixfx-phaser/src/effects.ts` is gone and `rg -n 'class Scene' crates/pixfx-wasm/src` finds it. Before that,
Phaser runs its own TypeScript loop and culling there would put a decision in the shell.

**Files:**
- Modify: `crates/pixfx-wasm/src/scene.rs` (`setView`, `clearView` on the JS `Scene`), `packages/pixfx-phaser/src/plugin.ts`
  (each tick before `draw()`: `setView` from `this.cameras.main.worldView`), `packages/pixfx-phaser/README.md`,
  `packages/pixfx-phaser/demo/stress.ts` and `perf/run.ts` (scenario "world 4× the camera, camera pans":
  `world=4&pan=1`)
- Test: `crates/pixfx-wasm/tests/scene.rs`, `packages/pixfx-phaser/tests/plugin.test.ts`

- [ ] **Step 1:** `bun run perf` on the starting commit (the "before").
- [ ] **Step 2:** Failing test in `plugin.test.ts`: an effect outside the camera's `worldView` gives no `make` or
  `show` op; after the camera scrolls onto it, it shows its current frame.
- [ ] **Step 3:** Implement. `bun run check` and `bun run perf:check` pass.
- [ ] **Step 4:** `bun run perf` after: the panning scenario's engine time is about a quarter of "before"; the
  existing scenarios are within the sweep's own spread.
- [ ] **Step 5:** Gate, then commit `phaser: the camera's view culls effects`, both perf results in the message.

### Task 7: stress runs with both unique and shared seeds

Start after a04a260's Phaser `FrameCache` rewrite is on `main` (`ls packages/pixfx-phaser/src/frames.ts`). Today every
stress effect has its own seed, so the sweep only shows the case where nothing is shared; a game placing many
torches without a seed is the other end. Agreed with a04a260 on 2026-11-14.

**Files:** `packages/pixfx-phaser/demo/stress.ts` (a `seeds=shared` option: every effect of one id gets the same
seed; the default stays one seed per effect), `packages/pixfx-phaser/perf/run.ts` (scenario "fires, shared seeds")

- [ ] **Step 1:** Add the option and the scenario; `bun run check` passes.
- [ ] **Step 2:** `bun run perf`: at 50 fires the shared-seed scenario's engine time is near the cost of one fire of
  each id (3 draws per authored frame), against the unique-seed scenario's. Record both.
- [ ] **Step 3:** Commit `perf: a shared-seed scenario shows what frame sharing saves`.

## Phase C: hit-stop

### Task 8: an effect can play through a hit-stop

Default stays as today (every effect freezes) unless the owner decides otherwise before this task starts.

**Files:**
- Modify: `crates/pixfx-canvas/src/stage.rs` (or `scene.rs`), `crates/pixfx-canvas/src/lib.rs` (binding),
  `README.md` (Game feel, Hosts)
- Test: `crates/pixfx-canvas/tests/stage.rs` (or `scene.rs`)

**Interfaces:**
- Produces:
  - `Scene::set_plays_through_hitstop(&mut self, handle: u32, on: bool) -> bool` (false if no such handle).
  - `Scene::update`: `let frozen = feel.frozen()` before `feel.advance(dt)`; a marked entry plays `dt` when `frozen`,
    else `run` like the others (slow motion slows it too). With no feel, every entry plays `dt`, as today.
  - JS: `stage.playThroughHitstop(handle: number, on: boolean): boolean`. Phaser gets it with Task 6's `Scene`
    (`PixfxEffect.playThroughHitstop(on = true)`), not before.

- [ ] **Step 1:** Failing tests, each starting from the setup of
  `an_effects_hit_freezes_every_effect_and_tells_the_host_how_long`:
  - `a_marked_effect_plays_on_while_the_rest_freeze`: mark the candle; over 5 frozen updates `t(candle)` grows by
    `5 * FRAME`, `t(hitter)` stays, `update` returns 0.
  - `the_effect_that_fires_the_stop_can_play_through_it`: mark the hitter; its `t` grows during the freeze, an
    unmarked candle's does not.
  - `slow_motion_slows_a_marked_effect_too`: `feel_mut().slow(1.0, 0.5)`; a marked candle advances `0.5 * FRAME` per
    update.
- [ ] **Step 2:** Run: FAIL. Implement. Run: PASS; the existing hit-stop test is unchanged.
- [ ] **Step 3:** Gate, then commit `scene: an effect can play on through a hit-stop`.

## Order

Task 1 → Task 2 → Task 3 (Phase A, can start now) → Task 4 → Task 5 → Task 8. Task 6 waits for Phaser on a `Scene`
(boundary plan Tasks 3 and 4); Task 7 waits for a04a260's `FrameCache` on `main`.

## Not in this plan

- Texture atlas: upload is 4.61 of 98 ms, and a04a260's `pixfx bake` already produces cropped atlases.
- Half-resolution light buffer: the light cost is in the CPU painter (`glow`, `add_at`), not in compositing; it
  belongs with the painter work handed to a04a260.
- Skipping the update of unseen effects: update costs about 0 ms, and skipping it would change their time and cues.
