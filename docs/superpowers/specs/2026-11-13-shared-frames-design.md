# Shared frames: fixed ticks, a frame cache, baked sheets, wasm SIMD

Date: 2026-11-13 · Asked by the owner: use what Rust gives us that pixelfx (JS) cannot, so the game spends less CPU on
effects. Chosen: fixed-step simulations, a frame cache shared by identical effects, baked sprite sheets; SIMD measured.

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

### 1. Live play shows rendered frames (pixfx-core)

An `Instance` keeps the host's clock `t` and shows frame `floor(t × fps)`, wrapped round a loop and held at a
one-shot's last frame. Its state takes ticks of `1 / fps` (respawning when a loop comes round), the same ticks
`render_frames` takes to frame k. Live play is then byte-equal to the render for **any** sequence of dt, which:

- makes every effect cacheable and bakeable (simulations included);
- removes the worker pool's inline-vs-late difference (review #6);
- keeps pixelfx parity: the bare render is unchanged, and pixelfx's own live play was never the reference.

Cues keep firing on the continuous clock; only drawing is quantised to frames.

### 2. A frame is a pure function of (effect, options, frame, fade)

The engine draws every frame, the painter's fade included (pixelfx `p.fade`, 0.3 s after `stop()`). The fade is
defined once, in the painter. Running effects have fade 1, so their frames are shared; a stopped effect's fade
frames are its own and it draws them with its own engine instance, exactly as pixelfx does.

### 3. Every host shares frames by one key (Rust), stored in its own form

Today the canvas `Stage` draws and blits every effect on every display frame (`stage.rs` `draw_layer`), 2 to 4
times the effect's 30 fps, and nothing is shared; Phaser uploads four textures per effect per new frame.

- **The key, once, in Rust (pixfx-core).** `ClipKey` is a canonical byte encoding of `(effect id, options, margin,
  placement)`; a shown frame is `(ClipKey, k)`. The encoder destructures `Options` and `Placement`, so a new field
  does not compile until it is encoded. The key travels to JS and into bake files as a string.
- **Player in pixfx-core.** `Player` (one effect on its own canvas) moves from pixfx-wasm to pixfx-core so every
  host plays through it (architecture review #3). `Player::shown(place)` gives the frame's identity, or none while
  the effect fades after `stop()` (a fade frame is its own).
- **One rule for every host:** redraw only when the shown frame changes; if another effect already holds that
  frame, show the held one instead of drawing.
- **Canvas (pixfx-canvas):** per frame, one `OffscreenCanvas` per layer, drawn once and `drawImage`d by every effect
  showing it.
- **Phaser:** effects with one key live in one worker lane; the first effect to reach a missing frame draws it and
  sends it once, cropped to its lit box; per frame four textures, shown by every effect on it.
- **gpui (pixfx-gpui):** per frame one image per layer, painted by every view showing it.
- **Memory:** each host holds frames under a byte budget (default 64 MB) and drops the least recently shown first;
  a dropped frame is drawn again by the next effect that reaches it.

### 4. Baked sheets (pixfx-cli, canvas, Phaser)

`pixfx bake <id>[:<options json>] … --out <dir>` writes, per key, an atlas PNG of every frame cropped to its lit
box (back colour, back light, front colour, front light) and a JSON of the key and each frame's rect and offset.
`scene.pixfx.loadBaked(url)` (Phaser) and `stage.loadBaked(url)` (canvas) fill the hosts' frame stores from it;
effects whose frames are baked never touch wasm to draw. Frames that are not baked fill the stores at runtime.

### 5. SIMD

Measured, not adopted: the wasm golden test takes the same CPU with and without `-C target-feature=+simd128` (user
26.0 / 26.5 s vs 26.1 / 26.5 s, frames byte-equal). The draw code is scalar f64 per pixel with branches, which the
compiler does not vectorise, so the flag would only raise the browser floor (Safari 16.4).

## Verification

- Property test: for random dt sequences, a live instance's drawn frame equals `render_frames` frame k (loops and
  one-shots, every effect).
- Goldens and `pixfx parity` unchanged.
- Key: equal inputs give equal keys; every field of `Options` and `Placement` changes the key.
- Each host (canvas, Phaser, gpui): fifty effects with one key draw each frame once; a host redraws only on a new
  frame; a dropped frame comes back byte-equal; a stopped effect fades on its own frames. Phaser: byte-equal frames
  inline vs pool with a late worker.
- `bun run perf` before and after, on the owner's machine (this agent's session has no display); Chrome check of
  the demo at 60 fps with no console errors.
