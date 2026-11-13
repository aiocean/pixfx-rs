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

## 2. Frame key and Player in pixfx-core

- `key.rs`: `ClipKey` = canonical bytes of `(effect id, Options, margin, Placement)` (destructured, so a new field
  fails to compile until encoded), `Display` as a string for JS and bake files.
- Move `Player` from pixfx-wasm to pixfx-core (re-exported by pixfx-wasm); `Player::shown(place)` →
  `Option<(ClipKey, u64)>`, none while fading.
- Tests: equal inputs equal keys; each field changes the key.

## 3. Canvas Stage shares frames

`pixfx-canvas`: a frame store keyed by `(ClipKey, k)` holding one `OffscreenCanvas` per layer, under a byte budget;
`draw_layer` redraws an effect only when its shown frame changes and draws a frame only when the store lacks it.
Tests: native `Scene` counts draws (fifty same-key effects, one draw per frame; no redraw without a new frame).

## 4. Phaser shares frames

- Lanes by key: every effect with one key lives in one worker lane (or inline).
- Worker: after `update(dt)`, an effect reports its shown frame. If the main thread lacks it, that effect draws it
  (it is on that frame), crops it to its lit box and sends it once as `(key, k, rect, buffers)`. A stopped effect
  draws and sends its own fade frames.
- Main thread `FrameCache`: per `(key, k)` the cropped bytes (layers composite from them) and four textures;
  byte budget, least recently shown dropped first; drops are sent to the worker.
- Tests: fifty effects with one key upload one set of textures per frame; inline and pool give the same bytes with
  a late worker; a dropped frame comes back with the same bytes; a stopped effect fades on its own textures.

## 5. gpui shares frames

`pixfx-gpui`: `FxView` plays a core `Player`; one `FxFrames` per window holds the uploaded images in a
`FrameStore` keyed by `(key, k)`. `paint_fx(view, frames, …)` rasterises and uploads a frame only when the store
lacks it; a fading view keeps its own images, redrawn once per clock frame. `FxFrames::end_frame(window)` drops
replaced and over-budget images from the atlas. Tests (`crates/pixfx-gpui/tests/frames.rs`, gpui's headless test
window): twenty campfires hold the bytes of one; no upload within a frame; budget drops and a fading view's own
images. `FrameStore` itself: `crates/pixfx-core/tests/frames.rs`.

## 6. Bake

`pixfx bake <id> [--palette WORD] [--outline WORD] --out <dir>` (`pixfx_export::bake`): every frame as
`Player::live` draws it (back and front), each layer half (colour RGBA, light `r, g, b, 255`) cropped and
shelf-packed onto PNG pages of 2048 px, plus `<name>.bake.json` keyed by the `ClipKey`. `pixfx_export::decode`
reads it back exactly (light compared by rgb: a light word's top byte is unused). Hosts: the canvas
`stage.loadBaked(json, pages)` (`Scene::load` into the frame store); Phaser `loadBaked(url)` (wasm `bakedFrames`
crops each side as `Fx.litBox` does, `Backend.hold` puts it in the `FrameCache` and tells every worker in its
next batch's `holds`). Tests: `pixfx-export/tests/bake.rs` (round trip equals live play at uneven steps),
`pixfx-canvas/tests/stage.rs` (a baked clip is shown with nothing drawn), `tests/frames.test.ts` (held frames
draw nothing, inline and pool).

## 7. SIMD (measured: no gain, not adopted)

The wasm golden test (25 effects × 3 seeds) takes the same CPU with and without `+simd128` (user 26.0 / 26.5 s vs
26.1 / 26.5 s, bytes equal): the draw code is scalar f64 per pixel with branches. The wasm keeps the default target.

## 8. Measure

`bun run perf` on the owner's machine before (main before task 1) and after; Chrome check of the demo.
