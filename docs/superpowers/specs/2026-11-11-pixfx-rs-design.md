# pixfx-rs — design

Date: 2026-11-11 · Status: draft for owner approval

## Intent

Two open-source projects make pixel-art effects as code:

- **pixelfx** (github.com/patch-gg/pixelfx, JS, MIT): a library of 244 effects, each a small module that draws
  into a frame with a colour layer and an additive light layer, played live in a browser or exported. Every
  character effect is *acted*: wind-up, hit-stop, target recoil. Core is ~3.7k lines (`src/core/*.js`).
- **beanfx** (github.com/baenv/beanfx, Python, MIT): a Claude Code plugin that *designs* new effects from words
  or a reference image (knob questions with ASCII previews, takes to pick from), draws them on bundled game
  casters, synthesises sound, and exports sprite sheets with Aseprite JSON.

pixfx-rs takes the best of both into one Rust codebase. **An effect is written once, as Rust, and produces the
same pixels two ways:**

1. **Rendered** natively to sprite sheet + JSON + preview, the way beanfx does.
2. **Played live** on the web through wasm, inside the owner's game, which is a **gpui** (Zed's UI framework)
   app compiled to wasm.

Why Rust: one language for both targets, type-checked effects, and determinism that both originals lack across
runtimes (pixelfx is JS-only live; beanfx is Python-only offline and needs a separate JS player).

## Goals (v1)

1. `pixfx-core`: the engine — frame (colour + light layers), painter, shapes, particles/sims, colour ramps,
   seeded RNG, acting (poses, hits, hit-stop, shake, target), chains, casters/rigs, composite.
2. All **244 pixelfx effects** ported, each matching pixelfx's behaviour (see *Verification*).
3. **Casters + acting**: beanfx's four bundled casters (`nghich_tien_kiem`, `_kiem_topdown`, `_truong`,
   `_truong_topdown`: sheets + `rig.json`) and pixelfx's stock actor/target, both usable by any effect.
4. **Native render/export** (CLI): sprite sheet PNG (combined, plus separate colour and light sheets), JSON in
   the beanfx layout (Aseprite "array" + `meta.beanfx`, see beanfx `docs/output-json.md`), gif, strip, a
   self-contained HTML preview, and "takes" sheets (variants side by side to pick from).
5. **Live wasm engine** usable from a gpui-on-wasm game via a `pixfx-gpui` adapter.
6. **Sound**: beanfx 0.3's synthesis (hit/charge/whoosh/tail cues from the effect's act and states; retro/soft/real
   voices; metal, thunder, fire, ice, wind, arcane), exported as WAV natively and playable live on the web.
7. **Claude Code skill** `/pixfx <description | image>`: research the subject, ask knob questions with ASCII
   previews, write a Rust effect module, render takes, look, refine, export. Knob set follows beanfx's
   `knobs/` (size, view, loop, timing, role, density, placement, clarity, style, palette, glow, shape, caster,
   combo, aura, states, sound).

## Non-goals

- A JS/TS API or npm package. The web consumer is a Rust gpui game; no JS boundary is designed for.
- Adapters for Bevy, macroquad, Phaser, Pixi. The core stays engine-agnostic so they can be added later.
- Runtime-loaded effects (scripts/data). Effects are compiled Rust; the web plays what was built in.
- beanfx's WIP `/char` and `/scene` (SDF/Blender) tools.
- pixelfx's "publish on Patch" flow and the Patch Pixel font (not MIT).

## Anti-goals (fails even if it "works")

- Native render and wasm playback of the same effect, seed and time **differ by any pixel**.
- A ported effect that looks visibly different from pixelfx's (motion, colours, timing).
- The core depends on gpui, a window system, the filesystem or a clock — it must stay pure.
- Credits dropped: the NOTICE must keep Patch, Hugo Duprez / Sprite Fusion, Hollow Crown, and bien (beanfx).
- Escape hatches used to get the compiler quiet (see *Constraints*).

## Constraints

- **Language rules** (owner's `CLAUDE.md`): clippy `unwrap_used`, `expect_used`, `panic` = deny;
  `#![forbid(unsafe_code)]` in every crate we own; no `clone()`/`Arc<Mutex>` added only to satisfy the borrow
  checker.
- **gpui on web** (verified 2026-11-11, zed-industries/zed `crates/gpui_web`): exists and is active (last commit
  2026-10-29), renders through `gpui_wgpu`, needs **nightly** Rust (`parking_lot` nightly features) and
  multithreaded wasm (`wasm_thread`). It is **not published** (`publish.workspace`), so `pixfx-gpui` and the game
  depend on gpui by git revision, pinned. gpui draws pixel buffers via `ImageSource::Render(Arc<RenderImage>)`
  (`crates/gpui/src/elements/img.rs:42`).
- **License**: MIT. `NOTICE.md` carries the upstream credits verbatim from pixelfx's `NOTICE.md` plus beanfx.
- **gpui cannot blend additively today** (verified 2026-11-11): image sprites use one colour target that is
  alpha blending, straight or premultiplied (`crates/gpui_wgpu/src/wgpu_renderer.rs:744-755`); the fragment
  shader multiplies RGB by alpha (`shaders.wgsl:391-394`, `blend_color`), so the premultiplied "alpha 0 adds"
  trick does not work; `PolychromeSprite` and `Window::paint_image` carry no blend mode. The image sampler is
  linear (`wgpu_renderer.rs:1264-1265`).
- **Machine**: keep the owner's Mac light — no resident heavy processes; Node is needed only for the pixelfx
  reference renders used by parity tests.

## Decisions the owner made

- Effects are **Rust code** (compiled into both native and wasm), not scripts or data.
- First use: **the same effect, both ways equally** (sheet export and live play) — so the shared, deterministic
  core comes first.
- Web host: a **Rust game on gpui**, compiled to wasm.
- v1 includes **all four**: the 244 effects, casters + acting, the skill, sound.
- Location and name: **`pixfx-rs`**, a standalone repo; the game depends on it as crates.
- **Acting in the live game: both modes.** (a) *Self-contained*: the gpui element draws the actor/caster and
  target and applies its own freeze and shake (pixelfx `createAct` model) — for previews, galleries, tools.
  (b) *Event timeline*: the element draws only the effect's back/front layers and emits events (pose name at t,
  hit with freeze/shake/push/react) that the game applies to its own characters, clock and camera (beanfx
  `porting.md` model). One `Act` description drives both.
- **Glow is truly additive over the game scene** in live play — not baked into alpha.
- **No game exists yet.** `examples/gpui-web` is the integration target; this repo picks and pins the gpui
  revision, and the future game pins the same one.
- **beanfx's 20 example drafts are ported too** (`drafts/*` in beanfx), as first-class effects alongside the 244.

## Design calls (mine — owner may override)

### Workspace layout

```
pixfx-rs/
  crates/
    pixfx-core     engine, pure: no I/O, no clock, no gpui; builds for wasm32 and native
    pixfx-effects  the effect library (244 ported + new), one module per effect, grouped by category
    pixfx-sfx      sound synthesis, pure: effect JSON/act -> PCM samples
    pixfx-export   native only: PNG/gif/strip, sheet packing, beanfx JSON, HTML preview, takes, WAV
    pixfx-cli      `pixfx render|sheet|takes|list|check|parity` over the above
    pixfx-gpui     adapter: a gpui element/entity that ticks an effect and paints its frame
  examples/gpui-web  a minimal gpui wasm app playing effects (the integration proof)
  skill/pixfx      the Claude Code skill (SKILL.md, knobs/, research notes)
  casters/         beanfx's four bundled casters
  NOTICE.md
```

Functional core, imperative shell: `pixfx-core`, `pixfx-effects`, `pixfx-sfx` are pure; files, windows and audio
devices live only in `pixfx-export`, `pixfx-cli`, `pixfx-gpui`.

### The effect contract

Mirrors pixelfx's module shape (`meta`, `act`, optional `create`/`update`, `drawBack`/`drawFront`) as a Rust
trait plus a static `Meta`:

- `Meta`: name, desc, tags, size, origin, duration, loop, fps, bg, actor, default options (typed per effect,
  palette ramps resampled to the default length as pixelfx's `resolveOptions` does).
- `Act` (optional): target, pose keyframes, hits (stop, shake, push, skin, …), dust — pixelfx `act.js` model.
- Drawing is a pure function of `(t, options, seed)` where possible; effects with simulations step state at a
  **fixed 1/120 s** (pixelfx steps at ≥120 Hz from the caller's dt; we fix the step so results do not depend on
  the host's frame rate). Rendering frame *n* of an export = advancing the same fixed steps the live engine does.

### Determinism (native == wasm, exactly)

- All transcendental math (`sin`, `cos`, `atan2`, `pow`, `exp`, `sqrt`, `hypot`) in core/effects/sfx goes through
  the **`libm`** crate, never `std` float methods, so both targets run the same software implementation.
  Enforced by a clippy `disallowed_methods` list.
- One seeded RNG/hash in core (port of pixelfx `Rng`/`rand` hashing); no OS randomness anywhere in the core.
- No `HashMap` iteration order in draw paths.
- f64 throughout the core (pixelfx uses JS doubles).

### Layers and compositing

Same model as pixelfx and beanfx: `color` (RGBA, opaque/dithered/translucent) and `light` (additive glow, summed
falloff `(1-d)^2 * k`), composited on top; over transparency light becomes colour with alpha = brightest channel
(beanfx `pfx.py` rule). Exports write the composite sheet and the separate colour and light sheets.

### Live engine in gpui

`pixfx-gpui` owns a frame buffer per effect instance, ticks it from gpui's frame callback, converts to
`RenderImage` and paints it. Pixel-crisp scaling is done on the CPU (integer upscale to whole device pixels)
because gpui samples images linearly; if the gpui change below adds a nearest sampler, use that instead.

**Additive light (top risk, first spike).** The colour layer paints as a normal alpha image; the light layer
paints as an image with an *additive* blend mode. That requires a gpui change: a blend mode on
`PolychromeSprite` / `paint_image`, a second sprite pipeline with additive `BlendState`, and the same in the
WebGL shader path (`shaders_webgl.wgsl`). Plan: spike it first in a pinned fork of zed's gpui, prove it in
`examples/gpui-web` (native and wasm), then offer it upstream as a PR; until merged, pixfx-gpui and the game pin
the fork's revision. If the spike fails, stop and bring it back to the owner before porting effects onto that path. Effects draw `back` and `front` layers so the game
can sandwich its character between them (pixelfx's `draw(frame, "back"|"front")`). Sound on the web goes through
the browser's audio via `web-sys` in the adapter or example, not in the pure crates.

### Act in both modes

`Act` is data (poses keyed by name and time, hits, dust, target). The self-contained mode plays it with the
ported pixelfx actor/target and beanfx casters; the event mode turns the same data into a time-ordered event
stream for the game. The four beanfx casters are usable in both (as the drawn character, or as reference
animations the game may adopt). Pose names follow beanfx (`idle`, `windup`, `cast`, `release`, …).

### beanfx kit and drafts

The skill's knobs need beanfx's kit, ported into core/effects alongside pixelfx's: `facing` (8/16 heading
rows), `derive` (points and lines taken from the drawing), `ref` (tracing a reference image, for
`/pixfx <image>`), `caster` (silhouette mock-ups), `fxkit`/`fxlib` (ramps, shade maps, strokes, noise, impact
frames), `light` (glow levels, flashes, flicker, bands). The export covers beanfx's full JSON, including
`parts` (cast/travel/impact), `move`/`portal`, `caster.chain`, `pages`, `sound`.

### Verification

- **Native vs wasm**: a test renders a fixed set of effects (every category) at fixed seeds and times natively and
  under wasm (e.g. `wasm-bindgen-test` in Node) and compares frame hashes — **must be equal**.
- **Port parity with pixelfx**: `pixfx parity <id>` renders pixelfx's reference with its own Node renderer
  (`tools/render.mjs`) and compares frames. JS `Math.sin` and `libm` can differ in the last bits, so the gate is a
  tolerance (proposed: ≤0.5% of pixels differ, and no differing pixel off by more than one palette step), plus a
  side-by-side gif for a human look. The reference is pixelfx's bare effect render (`renderSheet`, no actor), so
  porting does not wait on the acted render being pixel-exact; acted renders are compared by eye. Chaotic sims
  (random rolls inside steps) can drift from a one-bit `Math.sin` difference: an effect that fails the numbers
  passes only by a recorded human approval of its side-by-side gif, listed in a parity ledger. The palette-step
  rule applies to the colour layer; the light layer uses a per-channel tolerance.
- **beanfx drafts**: compared by eye against beanfx's own Python renders (same seed).
- **Golden images** for core primitives and the four casters.
- **Skill**: an end-to-end run produces a new effect that compiles, renders, passes `pixfx check` (cell does not
  touch its edge, seeds stable — beanfx's checks), and plays in `examples/gpui-web`.

## Left to the builder

- Exact crate APIs, module names, how options are typed (struct per effect vs. a typed options map).
- PNG/gif/audio crate choices (prefer small, pure-Rust, wasm-friendly).
- How effects are registered (macro, inventory, or a generated list) and whether categories are cargo features to
  keep the wasm bundle small.
- Order and batching of the 244 ports (they are independent once core is stable; parallel batches by category).
- HTML preview implementation (may reuse beanfx's `preview.html` behaviour, rewritten).
- Sound DSP details, as long as beanfx 0.3's feature list is covered.

## Calls I made from the review (owner may override)

- Sound is **opt-in per effect** (beanfx model: needs element, states, hits). The 244 pixelfx ports ship
  without sound; the skill's `sound` knob adds it.
- Effects register through a **public trait plus cross-crate registration**, so effects the skill writes can
  live in `pixfx-effects` or in the game's own crate.
- Parity rules as in *Verification* (bare render is the reference; human-approved ledger for chaotic sims).
