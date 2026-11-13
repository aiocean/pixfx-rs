# Shared frames: implementation plan

Spec: `docs/superpowers/specs/2026-11-13-shared-frames-design.md`. Each task ends green on `cargo fmt --check`,
`cargo clippy --workspace --all-targets`, `cargo test --workspace`, the ignored golden test
(`cargo test --release -p pixfx-effects --test determinism -- --ignored`) and, for Phaser, `bun run check`.

## 1. Fixed ticks (core)

`Instance` keeps the host's clock `t` for cues, stop and the end, and shows frame `floor(t × fps)` (wrapped round a
loop, held at a one-shot's last frame): its state takes ticks of `1 / fps`, respawning when a loop comes round.
`render_frames` seeks frame k with the same ticks. `Meta::frames()` is the one frame count. `Player::frame_index`
and wasm `Fx.frameIndex()` expose the frame.

Test: `pixfx-effects/tests/live_frames.rs`: uneven steps, zero steps and stalls draw `render_frames` frame k for
every effect. Goldens unchanged.

## 2. Clock and Clip (core)

- `Clock`: the host's time, stop, end and cues of one effect, with no state to step. `Instance` = `Clock` + state.
- `Clip`: one effect with one set of options and margin, drawing any frame k on request (seek forward, respawn
  when behind). `render_frames` = every frame of a clip. `fade_pass(frame, fade)`: the stop fade as a pass.

## 3. wasm bindings

`FxClock` (per effect: `update`, `stop`, `done`, `frameIndex`, `takeCues`, `fade`) and `FxClip` (per key:
`draw(k)` into the four RGBA buffers, `drawFaded(k, fade)`), both read through the pointer views Phaser already
uses.

## 4. Phaser: clip cache

- `ClipCache` (game-wide): key = id + canonical engine options + margin. Per frame: four textures, uploaded once.
  LRU over a texture-byte budget (default 64 MB).
- A clip's frames are drawn in one worker lane (or inline), requested in frame order.
- `PixfxEffect` keeps an `FxClock` on the main thread; on a new frame index it shows the cached frame's textures
  (requesting it when missing). After `stop()` it draws its own faded frames.
- `runner.ts` decides redraws by `frameIndex()`, not `floor(t × fps)`.
- Tests: fifty effects with one key hold one clip's textures; inline and pool give the same frame bytes with a
  late worker; eviction recomputes the same bytes.

## 5. Bake

`pixfx bake <id>[:<options json>]… --out <dir>`: per clip an atlas PNG (each frame cropped to its lit box, four
layers) and a JSON of rects and offsets. `scene.pixfx.loadBaked(key, url)` in `preload` fills the cache.

## 6. SIMD

`.cargo/config.toml`: `[target.wasm32-unknown-unknown] rustflags = ["-C", "target-feature=+simd128"]`. Gate: the
wasm determinism test (frame hashes equal native) and a node timing of the RGBA conversion before and after.

## 7. Measure

`bun run perf` on the owner's machine before (main before task 1) and after; Chrome check of the demo.
