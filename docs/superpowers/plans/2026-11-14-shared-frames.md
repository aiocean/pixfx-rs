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

## 2. Phaser: frame cache

- Lanes by key: every effect with one key `(id, canonical engine options, margin)` lives in one worker lane (or
  inline).
- Worker: after `update(dt)`, an effect reports `frameIndex()`. If its key's frame k is not on the main thread,
  that effect draws it (it is on frame k), crops it to its lit box and sends it once as `(key, k, rect, buffers)`.
  A stopped effect draws and sends its own fade frames, as today.
- Main thread `FrameCache`: per `(key, k)` the cropped bytes (layers composite from them) and four textures
  (effects show them). Byte budget (default 64 MB), least recently shown dropped first; drops are sent to the
  worker so the next effect on that frame draws it again.
- `PixfxEffect` shows the cached frame's textures at the frame's offset; `runner.ts` redraws by `frameIndex()`.
- Tests: fifty effects with one key upload one set of textures per frame; inline and pool give the same bytes with
  a late worker; a dropped frame comes back with the same bytes; a stopped effect fades on its own textures.

## 3. Bake

`pixfx bake <id>[:<options json>]… --out <dir>`: per clip an atlas PNG (each frame cropped to its lit box, four
layers) and a JSON of rects and offsets. `scene.pixfx.loadBaked(key, url)` in `preload` fills the cache.

## 4. SIMD (measured: no gain, not adopted)

The wasm golden test (25 effects × 3 seeds) takes the same CPU with and without `+simd128` (user 26.0 / 26.5 s vs
26.1 / 26.5 s, bytes equal): the draw code is scalar f64 per pixel with branches. The wasm keeps the default target.

## 5. Measure

`bun run perf` on the owner's machine before (main before task 1) and after; Chrome check of the demo.
