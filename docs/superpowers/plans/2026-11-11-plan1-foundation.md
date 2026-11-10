# pixfx-rs Plan 1 — Foundation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Prove the whole pipeline on a few real effects: one Rust effect renders to a sprite sheet natively, plays
live in a gpui wasm app with truly additive glow, and both give identical pixels.

**Architecture:** A pure `pixfx-core` (frame with colour + light layers, painter, sims, fixed-step effect
instances) ported from pixelfx `src/core`; effects in `pixfx-effects`; native export and CLI on top; a
`pixfx-gpui` adapter that paints the colour layer normally and the light layer with a new additive sprite blend
mode added to a pinned fork of gpui.

**Tech Stack:** Rust (pinned nightly, needed by gpui on wasm), `libm`, gpui + gpui_web + gpui_wgpu (zed fork),
trunk, wasm-bindgen-test, Node ≥18 (pixelfx reference renders only).

**Spec:** `docs/superpowers/specs/2026-11-11-pixfx-rs-design.md`

**This plan's slice of the spec:** additive-blend spike (top risk), core, determinism, parity tooling, three
pixelfx fire effects (`fire/_lib.js`, `fire/candle`, `fire/campfire`, `fire/brazier`), sheet + beanfx JSON export,
gpui live playback. **Later plans** (written after this one lands): (2) the remaining pixelfx effects in batches,
with acting (both modes); (3) beanfx kit, casters, 20 drafts, full JSON; (4) sound; (5) the `/pixfx` skill.

**Reference sources:** pixelfx at `/tmp/fxref/pixelfx` (commit `044e12a`), beanfx at `/tmp/fxref/beanfx`
(commit `923deee`), zed at commit `b1962f7cd8569ecade486c40320e3cff2d4b9d22` (2026-11-10). If `/tmp/fxref` is
gone, re-clone: `git clone https://github.com/patch-gg/pixelfx` / `baenv/beanfx` and check out those commits.

## Global Constraints

- Every crate we own: `#![forbid(unsafe_code)]`; workspace clippy lints `unwrap_used`, `expect_used`, `panic` =
  `deny`; no `clone()`/`Arc<Mutex>` added only to satisfy the borrow checker.
- In `pixfx-core` and `pixfx-effects`: transcendental float math only through `libm` (`sin cos tan atan2 pow
  exp ln sqrt hypot floor ceil round`); `std` float methods for these are listed in `clippy.toml`
  `disallowed-methods`. f64 throughout. No `HashMap` iteration in draw paths. No I/O, clock, OS randomness.
- Simulations step at a fixed `1/120 s`; `update(dt)` runs `max(1, ceil(dt*120 - 1e-9))` sub-steps of `dt/n`
  (pixelfx `src/core/fx.js` `createFx.update`).
- Light composite rule: over transparency, light becomes colour with alpha = its brightest channel (beanfx
  `kit/pfx.py`).
- `NOTICE.md` carries pixelfx's `NOTICE.md` credits (Hugo Duprez / Sprite Fusion, Hollow Crown, Patch) verbatim
  plus "beanfx, Copyright (c) 2026 bien, MIT". Ported files that pixelfx marks "adapted from Sprite Fusion" keep
  that header comment.
- License MIT. Toolchain pinned in `rust-toolchain.toml`: `channel = "nightly-2026-09-07"`, targets
  `wasm32-unknown-unknown`, components `rust-src rustfmt clippy`.
- Repo-wide check before every commit: `cargo fmt --check && cargo clippy --workspace --all-targets -- -D
  warnings && cargo test --workspace`.
- Publishing the gpui fork to GitHub is outward-facing: **owner approval required**; this plan uses local paths.

## Review Focus

1. **Huge `dt` in live play** (tab was in background for 30 s): `update(30.0)` would run 3600 sub-steps per call.
   Expected: the instance clamps one `update` to at most `0.25 s` of sim time (`MAX_DT`), no hang. Test in Task 5.
2. **Effect placed partly off-screen / negative coordinates**: painting outside the frame must clip, never index
   out of bounds. Test in Task 3.
3. **Non-loop effect after its duration**: `done()` is true, drawing yields an empty frame; a loop effect wraps
   `t % duration`. Test in Task 5.
4. **Light sums past 1.0** where many glows overlap: composite saturates at 255, no wrap. Test in Task 3.
5. **Window resize / device-pixel-ratio change in gpui** while an effect plays: the integer upscale recomputes;
   the effect keeps its world position. Checked by hand in Task 10 (step listed).

---

### Task 1: gpui additive sprite blend (spike, WebGPU + WebGL)

**Files (in the fork, not in pixfx-rs):**
- Create fork: `zed-gpui/` — partial clone of `https://github.com/zed-industries/zed` at `b1962f7`,
  branch `pixfx/additive-sprites`; sparse-checkout `crates/gpui* crates/scheduler crates/http_client
  crates/util* crates/collections crates/refineable crates/sum_tree crates/media` and whatever else
  `cargo metadata` reports missing (add until `cargo check -p gpui` resolves).
- Modify: `crates/gpui/src/scene.rs` (PolychromeSprite ~749, batching ~429-448)
- Modify: `crates/gpui/src/window.rs` (`paint_image` ~5305)
- Modify: `crates/gpui_wgpu/src/wgpu_renderer.rs` (pipelines ~744-760, batch draw ~1627)
- Modify: `crates/gpui_wgpu/src/shaders.wgsl`, `shaders_webgl.wgsl` only if the webgl path needs its own entry
- Create: `crates/gpui_web/examples/hello_web/additive_test.rs` + a `[[bin]] additive-test` entry

**Interfaces:**
- Produces: `pub enum SpriteBlend { Normal, Additive }` (re-exported from `gpui`), field
  `PolychromeSprite::blend: SpriteBlend`, and
  `Window::paint_image_blended(&mut self, bounds: Bounds<Pixels>, image_bounds: Bounds<Pixels>, corner_radii: Corners<Pixels>, data: Arc<RenderImage>, frame_index: usize, grayscale: bool, blend: SpriteBlend) -> Result<()>`.
  `paint_image` keeps its signature and calls `paint_image_blended(..., SpriteBlend::Normal)`.
- Additive semantics: `dst.rgb = dst.rgb + src.rgb * opacity`, `dst.a` unchanged
  (`BlendState { color: {src: One, dst: One, op: Add}, alpha: {src: Zero, dst: One, op: Add} }`), shader returns
  `rgb` un-multiplied by `src.a` for additive sprites.

- [ ] **Step 1: Write the failing test** in `crates/gpui/src/scene.rs` tests: two sprites on the same texture,
  same order, blends `Normal` then `Additive` → `scene.batches()` yields **2** `PolychromeSprites` batches, each
  carrying its blend; two sprites both `Additive` on the same texture → **1** batch.
- [ ] **Step 2:** `cargo test -p gpui scene::` → FAIL (no `SpriteBlend`).
- [ ] **Step 3: Implement** `SpriteBlend`, the field, batch split on `(texture_id, blend)`, `PrimitiveBatch::PolychromeSprites { texture_id, blend, range }`, `paint_image_blended`.
- [ ] **Step 4:** `cargo test -p gpui scene::` → PASS.
- [ ] **Step 5: Renderer.** Add an `additive_poly_sprites` pipeline in `wgpu_renderer.rs` (same shader module,
  fragment entry `fs_poly_sprite_additive` returning `vec4(color.rgb * opacity_factor, 0.0)`), select it for
  `PrimitiveBatch::PolychromeSprites { blend: Additive, .. }`. Apply the same to the WebGL shader source if it is a
  separate file.
- [ ] **Step 6: Visual proof.** `additive_test.rs`: a mid-grey `#404040` background, a red `#ff0000` square
  painted normally, and a 64×64 `RenderImage` of solid `#004000` (alpha 255) painted with
  `SpriteBlend::Additive` half over the red square. Install trunk if missing: `cargo install trunk --locked`.
  Run `trunk serve` in `hello_web`, open `/additive-test` with the ego-browser skill, screenshot with WebGPU and with
  WebGL2 (force via the gallery's fallback flag or a browser without WebGPU).
  Expected pixels: over grey `#404040` → `#408040`; over red → `#ff4000`; red square elsewhere `#ff0000`.
  Read the pixels from the screenshot (not by eye) and record them in the commit message.
- [ ] **Step 7: Gate.** If Step 6 does not give those pixels on both backends after one focused debugging pass
  (superpowers:systematic-debugging), **stop the plan** and report `BLOCKED: additive blend in gpui` with what
  failed — the owner decides the next path.
- [ ] **Step 8: Commit** in the fork: `gpui: additive blend mode for image sprites (WebGPU, WebGL)`.

### Task 2: gpui additive blend on Metal (native macOS)

**Files:** Modify `crates/gpui_apple/src/metal_renderer.rs`, `crates/gpui_apple/src/shaders.metal`; reuse
`crates/gpui/examples/` with a native `additive_test` example (same scene as Task 1 Step 6).

**Interfaces:** Consumes `SpriteBlend` / `PrimitiveBatch::PolychromeSprites { blend, .. }` from Task 1.

- [ ] **Step 1:** Add a Metal pipeline state with blending `rgb: One + One`, `alpha: Zero + One` and a fragment
  function `polychrome_sprite_additive_fragment`; select it per batch.
- [ ] **Step 2:** `cargo run -p gpui --example additive_test`, screenshot the window (`screencapture -l <id>`),
  read the same three pixels as Task 1 Step 6 → `#408040`, `#ff4000`, `#ff0000`.
- [ ] **Step 3: Commit** in the fork: `gpui: additive blend mode for image sprites (Metal)`.

### Task 3: Workspace, frame, colour, painter, light

**Files:**
- Create: `pixfx-rs/Cargo.toml` (workspace: `crates/*`, `examples/gpui-web` excluded from default members),
  `rust-toolchain.toml`, `clippy.toml`, `.gitignore`, `LICENSE` (MIT, "Copyright (c) 2026 pixfx-rs
  contributors"), `NOTICE.md`
- Create: `crates/pixfx-core/src/{lib.rs, math.rs, color.rs, frame.rs, painter.rs}` — ports of pixelfx
  `src/core/math.js`, `color.js`, `fx.js` (frame + composite parts), `painter.js`
- Test: `crates/pixfx-core/tests/{painter.rs, composite.rs}`

**Interfaces (produces):**
- `pub struct Rgba(pub u8, pub u8, pub u8, pub u8)`; `pub fn hex(s: &str) -> Result<Rgba, ColorError>`; ramps:
  `pub fn fit(ramp: &[Rgba], n: usize) -> Vec<Rgba>`, `pub fn mix(a: Rgba, b: Rgba, t: f64) -> Rgba` (match
  pixelfx `color.js` rounding exactly).
- `pub struct Frame { pub w: u32, pub h: u32, pub ox: i32, pub oy: i32, pub color: Vec<u32>, pub light: Vec<u32> }`
  with pixelfx's packing (colour ABGR, light xBGR in `u32`); `Frame::new(w, h, ox, oy)`, `Frame::clear()`.
- `pub fn composite(frame: &Frame, out: &mut [u8], bg: Option<&[u8]>)` → RGBA8, rule from Global Constraints.
- `pub struct Painter<'a>` with pixelfx `Painter` method set in snake_case (`place(x, y, flip, angle)`, `px`,
  `dot`, `rect`, `ellipse`, `poly`, `tri`, `line`, `glow`, `glow_line`, `glow_ring`, `add`, …) — port every
  method `fire/_lib.js` and the three fire effects call; others come with later plans.
- `pub struct Rng`, `pub fn rand(a: f64, b: f64) -> f64` — exact ports of pixelfx `math.js` hashing.

- [ ] **Step 1: Write failing tests:** `glow_peak_and_falloff` (glow at (8,8) r=4 k=0.5 white on a 16×16 frame →
  light at centre pixel has each channel `round(255*0.5*(1-d)^2)` for that pixel's `d`; pixel at distance ≥4 is 0);
  `paint_clips_offscreen` (rect from (-10,-10) size 30×30 on 8×8 frame → no panic, all 64 colour pixels set);
  `light_saturates` (20 overlapping glows k=0.5 → composite channel = 255); `composite_light_over_transparent`
  (colour empty, light (100,50,0) → RGBA (100,50,0,100)); `hex_parses_short_and_long` (`#f80` == `#ff8800ff`).
- [ ] **Step 2:** `cargo test -p pixfx-core` → FAIL (types missing).
- [ ] **Step 3: Implement** the ports; check each against the JS source line by line.
- [ ] **Step 4:** `cargo test -p pixfx-core` → PASS; repo-wide check passes.
- [ ] **Step 5: Commit** `core: frame, colour, painter and light`.

### Task 4: Simulations used by the fire effects

**Files:** Create `crates/pixfx-core/src/{sim.rs, particles.rs, shapes.rs}` — ports of the parts of pixelfx
`sim.js`, `particles.js`, `shapes.js` that `fire/_lib.js`, `candle`, `campfire`, `brazier` use (keep the "adapted
from Sprite Fusion" header on `sim.rs`). Test: `crates/pixfx-core/tests/sim.rs`.

**Interfaces (produces):** the Rust equivalents of each imported JS symbol, same names in snake_case /
CamelCase types (`Particles`, `Bits`, `Lights`, `Blazes`, `GRAVITY`, `FIRE_RAMP`, `SMOKE_RAMP`, `flare`, …
only those used).

- [ ] **Step 1: Write failing test** `particles_are_seeded`: two `Particles` systems with the same seed, stepped
  240 times at 1/120 → identical state; a different seed → different state.
- [ ] **Step 2:** run → FAIL. **Step 3:** implement. **Step 4:** run → PASS, repo-wide check passes.
- [ ] **Step 5: Commit** `core: simulations for fire effects`.

### Task 5: Effect trait, instances, frame rendering

**Files:** Create `crates/pixfx-core/src/{effect.rs, render.rs}`; Test `crates/pixfx-core/tests/instance.rs`.

**Interfaces (produces):**
```rust
pub struct Meta { pub id: &'static str, pub name: &'static str, pub desc: &'static str,
  pub tags: &'static [&'static str], pub size: (u32, u32), pub origin: (i32, i32),
  pub duration: f64, pub looping: bool, pub fps: u32, pub screen: bool }
pub enum Layer { Back, Front, Both }
pub trait Effect: Send + Sync {
    fn meta(&self) -> &Meta;
    fn spawn(&self, opts: &Options, seed: u64) -> Box<dyn EffectState>;
}
pub trait EffectState: Send {
    fn update(&mut self, dt: f64) {}                       // one fixed sub-step
    fn draw(&self, p: &mut Painter, t: f64, layer: Layer); // pure in (state, t)
}
pub struct Instance { /* effect, state, t, stop_at, opts */ }
impl Instance {
    pub fn new(effect: &'static dyn Effect, opts: Options) -> Self;
    pub fn update(&mut self, dt: f64);   // clamps dt to MAX_DT = 0.25, then sub-steps per Global Constraints
    pub fn draw(&self, frame: &mut Frame, layer: Layer);
    pub fn stop(&mut self);              // fades over 0.3 s (pixelfx FADE)
    pub fn done(&self) -> bool;
    pub fn t(&self) -> f64;
}
pub struct Options { pub x: f64, pub y: f64, pub flip: bool, pub angle: f64, pub seed: Option<u64>,
  pub light: f64, pub palette: Option<Vec<Rgba>>, pub extra: BTreeMap<String, OptValue> }
pub fn render_frames(effect: &'static dyn Effect, opts: &Options, margin: u32) -> Rendered; // port of fxlib.mjs renderFrames, bg "none", no actor, edge fade when margin == 0 and !screen
pub struct Rendered { pub w: u32, pub h: u32, pub fps: u32, pub frames: Vec<Frame> }
```
Effect registration: `inventory`-style `pixfx_core::register!(EFFECT)` collected by
`pub fn effects() -> impl Iterator<Item = &'static dyn Effect>` and `pub fn effect(id: &str) -> Option<&'static dyn Effect>`
(works across crates, spec call).

- [ ] **Step 1: Write failing tests** with a tiny test effect (a dot moving 1 px per sub-step):
  `update_substeps` (`update(1/30)` → 4 sub-steps); `huge_dt_is_clamped` (`update(30.0)` → 30 sub-steps, returns
  fast); `non_loop_done_after_duration`; `loop_wraps_time`; `render_frame_count` (non-loop duration 1.0 fps 30 →
  31 frames; loop → 30, matching pixelfx `n`).
- [ ] **Step 2:** run → FAIL. **Step 3:** implement. **Step 4:** run → PASS, repo-wide check.
- [ ] **Step 5: Commit** `core: effect trait, instances, frame rendering`.

### Task 6: Parity tooling against pixelfx

**Files:**
- Create: `tools/pixelfx-ref.mjs` — Node script: `node tools/pixelfx-ref.mjs <id> --pixelfx DIR --out DIR`;
  imports pixelfx `tools/lib/fxlib.mjs`, calls `renderFrames(fx, { bg: "none", actor: false })`, writes
  `<id>.color.bin` / `<id>.light.bin` (raw `Uint32Array` per frame, concatenated) and `<id>.json`
  `{ w, h, n, fps }`. Same idea as beanfx `tools/layers.mjs`.
- Create: `crates/pixfx-cli/src/{main.rs, parity.rs}`, `crates/pixfx-effects/src/lib.rs` (empty registry)
- Test: `crates/pixfx-cli/tests/parity.rs`

**Interfaces (produces):** `pixfx parity <id> [--pixelfx DIR] [--out DIR]` → prints
`<id>: colour <pct>% differ, light <pct>% differ → PASS|FAIL` and writes `<out>/<id>.side.gif` (pixelfx left,
pixfx right, 4× scale). Pure function `pub fn compare(a: &Rendered, b: &Rendered) -> ParityReport` with
`colour_diff_frac`, `light_diff_frac` (light pixel differs when any channel differs by > 2). Gate: both ≤ 0.005. (The spec's "no colour pixel off by more than one palette step" rule is not enforced in Plan 1; the side-by-side gif covers it by eye. Plan 2 adds it once more palettes are ported.)
Parity ledger: `parity/LEDGER.md`, one line per effect that passes only by human approval
(`<id> · <date> · colour x% light y% · approved by owner`).

- [ ] **Step 1: Write failing tests** for `compare`: identical → 0/0 PASS; one pixel of 100 differs in colour →
  0.01 FAIL; light off by 2 → not counted; off by 3 → counted.
- [ ] **Step 2:** run → FAIL. **Step 3:** implement `compare`, the CLI subcommand, the Node script.
- [ ] **Step 4:** run → PASS; `node tools/pixelfx-ref.mjs fire/candle --pixelfx /tmp/fxref/pixelfx --out
  target/ref` writes 120 frames (`duration 4 × fps 30`, loop) — check `n` in the JSON.
- [ ] **Step 5: Commit** `cli: parity against pixelfx reference renders`.

### Task 7: Port the fire effects

**Files:** Create `crates/pixfx-effects/src/fire/{mod.rs, lib.rs, candle.rs, campfire.rs, brazier.rs}` — ports of
pixelfx `src/fx/fire/_lib.js`, `candle.js`, `campfire.js`, `brazier.js`, keeping their headers and option
defaults (palettes, seeds) exactly.

**Interfaces:** consumes Tasks 3-5; produces registered effects with ids `fire/candle`, `fire/campfire`,
`fire/brazier`.

- [ ] **Step 1: Write failing test** `crates/pixfx-effects/tests/fire.rs`: `effect("fire/candle")` exists,
  `meta().size == (56, 76)`, `origin == (28, 64)`, `duration == 4.0`, `looping`, `fps == 30`.
- [ ] **Step 2:** run → FAIL. **Step 3:** port the four files.
- [ ] **Step 4:** `cargo test -p pixfx-effects` → PASS; then for each id run `cargo run -p pixfx-cli -- parity
  fire/<id>` → **PASS** expected. A FAIL: debug (systematic-debugging) until PASS or until the cause is shown to be
  JS-vs-libm float drift (prove it by locating the first differing sub-step value), then add a ledger line marked
  `pending owner approval` and report it — never mark it approved yourself.
- [ ] **Step 5: Commit** `effects: fire candle, campfire, brazier`.

### Task 8: Native export — sheets, beanfx JSON, gif

**Files:** Create `crates/pixfx-export/src/{lib.rs, sheet.rs, json.rs, gif.rs}`; add `pixfx sheet` and `pixfx
render` to `pixfx-cli`. Test `crates/pixfx-export/tests/sheet.rs`.

**Interfaces (produces):**
`pub fn write_sheet(r: &Rendered, meta: &Meta, out_dir: &Path, name: &str) -> Result<SheetFiles, ExportError>` →
writes `<name>.png` (composite), `<name>_color.png`, `<name>_light.png` (RGB, additive), `<name>.json` in the
beanfx layout (`docs/output-json.md` in beanfx): Aseprite array `frames[]` with `duration = round(1000/fps)` ms,
`meta.image`, `meta.size`, and `meta.beanfx` always-present fields `name fps frames rows(=1) frameSize columns
pivot loop end sheet` plus `color light blend_light("add") light_gain(1.0)`. `columns = min(frames, 8)`;
`pivot = origin + margin`; `end = "loop"` if looping else `"remove"`.
`pixfx render <id> --gif [--scale N]` writes `<id>.gif`.

- [ ] **Step 1: Write failing test** `candle_sheet_layout`: candle → JSON has 120 frames, `columns 8`,
  `size {w: 448, h: 1140}` (8×56, 15×76), `pivot [28, 64]`, `end "loop"`, PNG dimensions match `size`.
- [ ] **Step 2:** run → FAIL. **Step 3:** implement (pick small pure-Rust crates for PNG/gif).
- [ ] **Step 4:** run → PASS; `cargo run -p pixfx-cli -- sheet fire/campfire --out target/out` and open the PNG
  with Read to look at it.
- [ ] **Step 5: Commit** `export: sprite sheets, beanfx JSON, gif`.

### Task 9: Native == wasm determinism

**Files:** Create `crates/pixfx-effects/tests/determinism.rs` and `crates/pixfx-effects/tests/goldens/frame_hashes.json`; modify `crates/pixfx-core/src/frame.rs` (add `frame_hash`).

**Interfaces:** `pub fn frame_hash(f: &Frame) -> u64` (FNV-1a 64 over `color` then `light` words, little-endian)
in `pixfx-core`.

- [ ] **Step 1: Write the test** `frame_hashes_match_goldens`: for each fire effect, seeds `[1, 2, 7]`, frames
  `[0, 1, n/2, n-1]` from `render_frames` → hashes equal `goldens/frame_hashes.json` (loaded with
  `include_str!`). Mark it `#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]` alongside `#[test]`.
- [ ] **Step 2:** generate goldens natively once with `PIXFX_BLESS=1 cargo test -p pixfx-effects determinism`
  (the test writes the file when that env var is set and the target is native), commit them.
- [ ] **Step 3:** `cargo test -p pixfx-effects determinism` → PASS natively.
- [ ] **Step 4:** `wasm-pack test --node crates/pixfx-effects -- --test determinism` → PASS (same goldens).
- [ ] **Step 5: Commit** `test: native and wasm frames hash identically`.

### Task 10: gpui adapter and web example

**Files:**
- Create: `crates/pixfx-gpui/src/lib.rs` — depends on the fork by path (`../../../zed-gpui/crates/gpui`)
- Create: `examples/gpui-web/{Cargo.toml, main.rs, index.html, Trunk.toml, .cargo/config.toml}` — copy the build
  setup of zed `crates/gpui_web/examples/hello_web` (`.cargo/config.toml` atomics/shared-memory flags, `build-std`,
  COOP/COEP headers in `Trunk.toml`, `wasm_thread` rev `0cf96c7708dfb97ccf3da50347e25edcf75d6937`)

**Interfaces (produces):**
```rust
pub struct FxView { /* Instance, colour/light buffers, scale */ }
impl FxView {
    pub fn new(effect: &'static dyn Effect, opts: Options) -> Self;
    pub fn tick(&mut self, dt: f64);
}
pub fn paint_fx(view: &FxView, layer: Layer, origin: Point<Pixels>, scale: u32, window: &mut Window) -> Result<()>;
// paints colour with SpriteBlend::Normal, light with SpriteBlend::Additive, both as RenderImage
// upscaled on the CPU by integer `scale` (nearest), so pixels stay crisp.
```
Ticks from gpui's per-frame callback (`window.request_animation_frame()` / `on_next_frame`, whichever the pinned
gpui exposes) using real elapsed time.

- [ ] **Step 1: Write failing test** `upscale_nearest`: a 2×1 frame scaled 3× → 6×3 buffer, each source pixel a
  solid 3×3 block.
- [ ] **Step 2:** run → FAIL. **Step 3:** implement adapter + example: a dark ground scene, candle and campfire
  side by side at scale 4, campfire's glow brightening the ground behind it.
- [ ] **Step 4:** `cargo test -p pixfx-gpui` → PASS; `trunk serve` in `examples/gpui-web`, open with ego-browser,
  screenshot: flames animate (two screenshots 0.5 s apart differ), the ground under the campfire is brighter than
  the ground far from it (read pixel values), no console errors. Resize the window and change zoom: effects stay
  crisp and in place.
- [ ] **Step 5: Commit** `gpui: live playback with additive light, web example`.

### Task 11: Close-out

- [ ] **Step 1:** README.md: what pixfx-rs is, how to render a sheet, how to run the web example, credits →
  `NOTICE.md`. CHANGELOG `0.1.0`.
- [ ] **Step 2:** repo-wide check + Task 9 wasm test + Task 7 parity for all three effects, outputs pasted in the
  final report; ledger state reported.
- [ ] **Step 3: Commit** `docs: README, changelog 0.1.0`. Do not push; report to the owner, and ask before
  publishing the gpui fork or the repo.
