# Shared frames: fixed ticks, a frame cache, baked sheets, wasm SIMD

Date: 2026-11-13 · Asked by the owner: use what Rust gives us that pixelfx (JS) cannot, so the game spends less CPU on
effects. Chosen: fixed-step simulations, a frame cache shared by identical effects, baked sprite sheets, SIMD.

## Facts this rests on

Measured natively (release, Apple M4) with `render_frames` over every effect at its default options and glow margin:

| | |
|---|---|
| Draw cost per frame | 0.04 ms (coin-spin) to 5.9 ms (snow); campfire 0.97 ms, brazier 1.08 ms |
| Frames per clip | 15 (laser-bolt) to 360 (snow, fireflies); campfire 90 |
| All 25 clips as colour + light RGBA | 3669 MB full canvas, 550 MB cropped to each frame's lit box |
| Campfire clip | 46 MB full, 10.8 MB cropped |

From the code:

- Every looping effect (15) is a pure function of `t % duration` (no `update`); every simulating effect (9) is a
  one-shot. So every effect is a **finite sequence of frames**: `round(duration × fps)` for a loop, one more for a
  one-shot.
- `render_frames` already steps exactly as a fixed tick: frame i is drawn after i calls of `advance(1 / fps)`, each
  4 sub-steps of 1/120 s at fps 30 (`render.rs:47-51`). It is the pixelfx parity reference (design spec,
  *Verification*).
- Live play steps `dt / ceil(dt × 120)` from the host's dt (`effect.rs` `advance`), so a simulation's frame depends
  on the machine's frame times, and the worker pool's merged steps differ from inline (architecture review #6).
- Live hosts redraw only when `floor(t × fps)` changes (`runner.ts:36`), but draw at the current `t`, not at the
  frame's time.
- A frame's pixels depend only on effect id, engine options (seed, flip, angle, light, palette, own options) and
  margin. Position is the host's (`packages/pixfx-phaser/src/options.ts:22-31`).
- Each Phaser effect uploads four textures of its full canvas on every new frame (`surface.ts` `OwnSurface.show`);
  fifty identical campfires upload the same pixels fifty times.

## Design

### 1. A clip is the one definition of an effect's frames (pixfx-core)

`Clip` = the frames of one effect with one set of options and margin, computed lazily and kept:
`Clip::frame(k) -> &Frame` steps its own instance forward tick by tick (never backwards) and stores each frame.
`render_frames` becomes "every frame of a clip", so the parity reference and goldens do not change.

Frame index k of a live instance at time t is `floor(t × fps)`; a loop shows clip frame `k mod n`, a one-shot clip
frame `min(k, n − 1)`. Live play is then byte-equal to the render for **any** sequence of dt, which:

- makes every effect cacheable and bakeable (simulations included);
- removes the worker pool's inline-vs-late difference (review #6);
- keeps pixelfx parity: the bare render is unchanged, and pixelfx's own live play was never the reference.

Cues keep firing on continuous time; only drawing is quantised to the clip's frames.

### 2. Fade-out on stop is a pass over a frame

Today `stop()` fades by passing `fade` into the painter, so a stopped effect's frame is not a clip frame. The fade
becomes a pure pass applied on top of a clip frame: a colour pixel stays where `fade > bayer(x, y)`, light is scaled
by `fade`. Same 4×4 ordered dither and 0.3 s as the painter's fade; opaque pixels fade exactly as before, dithered
pixels and saturated light differ slightly (owner's choice: one code path, stopped effects stay on shared frames).

### 3. Phaser shares clips (frame cache)

A clip is keyed by `(id, engine options, margin)`. The plugin keeps one clip per key: each frame becomes four
textures, uploaded once, shown by every effect with that key. Per effect the plugin keeps only its time, stop
state and cues. Memory has a budget (default 64 MB of texture); past it, least-recently-shown clip frames are
dropped and recomputed on demand. Clips are computed in one worker each (a clip's simulation must step in order),
picked by least load as today.

### 4. Baked sheets (pixfx-cli + Phaser)

`pixfx bake <id>[:<options json>] … --out <dir>` writes, per clip, an atlas PNG of every frame cropped to its lit
box (back colour, back light, front colour, front light) and a JSON of each frame's rect and offset.
`scene.pixfx.loadBaked(key, url)` in the game's `preload` fills the clip cache from it; effects whose key is baked
never touch wasm to draw. Clips that are not baked fill the cache at runtime.

### 5. SIMD

Build the wasm with `-C target-feature=+simd128` (a `.cargo/config.toml` for `wasm32-unknown-unknown`). Integer
paths give the same bytes; the native-vs-wasm frame-hash test is the gate.

## Verification

- Property test: for random dt sequences, a live instance's drawn frame equals `render_frames` frame k (loops and
  one-shots, every effect).
- Goldens and `pixfx parity` unchanged.
- Phaser: fifty campfires share one clip (`stats().textures` stays at one clip's count); byte-equal frames inline
  vs pool with a late worker.
- `bun run perf` before and after, on the owner's machine (this agent's session has no display); Chrome check of
  the demo at 60 fps with no console errors.
