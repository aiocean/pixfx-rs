# Changelog

## Unreleased

- The site draws with `pixfx-canvas2d`: the landing duel, the effect pages, the gallery and the pointer fx paint on
  workers and load one engine (pixfx-web's wasm). Landing duel at 4x CPU throttling: 13-15 fps before, 58-60 after;
  at 6x: 11-15 before, 47-56 after.
- `pixfx-wasm`: the duel's bindings move here from `pixfx-canvas`, behind the `fighter` and `light` features (both
  default; +290 KB, a game builds without them): `Body`, `FightFeel`, `lookColors`, `Sprites` (now `new Sprites(w,
  h)` and `picture()`), and for light `stepLights(lights, dt)`, `putOutLight(light, within)` and `new Lighting(w,
  h).paint(lights, glows)`: the game keeps its own list of lights (`Lights` is gone). Pictures (`Picture`: the lit
  box, colour and light RGBA) are drawn by a shell. `scene.bend(src, w, h, unit)` bends a picture of the screen by
  the live ripples.
- `pixfx-canvas2d`: `drawPicture(picture)`, `warp()`; re-exports the engine's `effects`, `meta`, `Fx` and the fighter
  and light bindings. `pixfx-web`: `PixfxFeel.bend(src, w, h, unit)`.
- `pixfx-core`: a held frame counts at least one byte, so an unlit frame also goes past the budget (it was held
  forever, and which one stayed depended on hash order).
- `pixfx-canvas2d` (new): pixfx on a Canvas 2D context over `pixfx-web`, painting on workers. `createPixfx(ctx, {
  scale })`, `play(id, x, y, opts)`, and each frame `update(dt)` then `drawLayer("back")` / `drawLayer("front")` (or
  `draw()`) in the context's transform. `bun run build` writes `dist/` for pages without a bundler.
- `pixfx-web`: `Stage.live` is the Rust scene until `clear()`, then null; a feel built on it reads still after its
  shell is destroyed instead of throwing.
- `pixfx-pixi` (new): pixfx in a Pixi 8 application. `createPixfx(app, { container, camera })`, then `play(id, x, y,
  opts)` with the same effects, events and feel as Phaser; the shake moves and tilts a camera container, the flash
  covers the screen. On WebGL frames are regions of atlas pages filled with `texSubImage2D`.
- `pixfx-web` (new): the JS runtime every JS shell shares (wasm, painting workers, game-wide frames, a stage per
  host scene, effect and feel handles), host-neutral on eventemitter3.
- `pixfx-canvas`: a shared frame is held cut to the box that holds anything, so it takes the bytes the frame budget
  counts for it; what the stage shows is unchanged.
- `pixfx-phaser`: a pure shell over the wasm `Scene`. Placement, timing, which frame shows, fading, follow-stop,
  compositing and the frame cache come from Rust; workers simulate, paint, crop and convert, and transfer the bytes;
  on WebGL every surface is a region of a few atlas pages, filled with `texSubImage2D`. Removed: `createLayer` and
  `PixfxLayer` (no game needs layer groups; one atlas batch per layer costs less than re-merging a region whenever one
  effect changes), `PALETTE_PRESETS` (use `(await loadPixfx()).palettePresets()`), and the `textureBudget` load
  option. `loadBaked(url)` needs a scene with the plugin started. `pixfx-wasm`: `Frames.loadBaked(json, pages)`
  replaces `bakedFrames`. `pixfx-scene`: an effect has at most one frame in flight per layer with a painter
  elsewhere, and a frame that comes back late still shows once; a frame painted on the spot counts against the
  budget as its lit box (what shells upload), not its whole canvas.
- `pixfx-wasm`: the scene for JS shells (`src/scene.rs`). `new Frames(workers)` holds a game's shared frames;
  `frames.scene(scale)` makes a `Scene` per stage (`play`, `moveTo`, `set`, `stop`, `remove`, `update`, `events`,
  the feel, `setView`, `setDrawBudget`); `scene.draw(frames)` and `frames.endFrame()` return a draw list (an
  `Int32Array` of make, refill, show and free ops), each surface cropped to what it holds. With `new Frames(true)`
  frames are painted on workers: `frames.takeJobs()`, a worker's `Painting.paint(job)` paints, crops and turns the
  frame into RGBA, the worker transfers the bytes, and `scene.deliver(frames, job, shape)` takes back only their
  `Shape`, so pixels never pass through wasm on the main thread; they equal those painted on the spot. Painted on the
  spot (`new Frames(false)`), the bytes are at `frames.colorPtr(id)` and `lightPtr(id)` until the next call.
  `pixfx-scene`: `Frames::deliver` and `Scene::deliver_own` take a `Frame` or what the shell takes through
  `Receive<D>`. `palettePresets()` and `outlinePresets()` read the engine's presets.
  `Squash.step(dt, x, y, vx?, vy?)` scales a body for its velocity or, without one, for how far it moved, holding its
  scale through a hit-stop (`pixfx-core`: `squash::Moving`).
- `pixfx-scene`: painting leaves `Scene`. `Frames` holds the shared frames and a `Painter` for every scene of a game
  (`Frames::scene`), so a frame one scene painted shows in all of them; hosts end each frame with
  `Frames::end_frame`, and draw calls take the frames (`scene.draw(&mut frames, &mut shell)`). `InlinePainter`
  paints on the spot; a painter on another thread hands frames back with `Frames::deliver` and
  `Scene::deliver_own`, and an effect shows its last frame until then. `Scene::play` takes a `Clip`;
  `Scene::load`, `set_frame_budget` and `frame_bytes` are `Frames::deliver`, `set_budget` and `bytes`.
  `pixfx-core`: `Instance` is a `Clock` (time, stop, cues, fade, end) and a `Sim` (the effect's state, brought to a
  frame on demand); `Clip` is what an effect draws and how big. Every `frame_hash` is unchanged.
- `pixfx-scene`: `Scene::set_draw_budget(Some(n))` draws at most `n` layers per host frame (time-slicing); effects
  left over show their last frame and are drawn in a later host frame, the one waiting longest first, skipping to the
  frame their clock is on. `None` (the default) draws every layer whose frame changed, as before.
  `pixfx-gpui`: `FxScene::set_draw_budget`; `pixfx-canvas`: `Stage.setDrawBudget(layers)` (0 draws every one).
- `pixfx-scene` (new): `Scene` and the `Shell` contract move out of `pixfx-canvas` (`pixfx_canvas::stage`) so every
  shell shares them; `Shell` gains `free(surface)`, called for every surface the scene drops (past the frame budget,
  or when its effect ends), and `Scene::clear` frees all of them for a host shutting down.
- `pixfx-scene`: `Scene::draw_handle` shows one effect within a host frame and `Scene::end_frame` ends it, for a host
  that paints each effect on its own. `pixfx-gpui`: `FxScene` (`play`, `tick`, `stop`, `remove`, `paint`, `end_frame`)
  replaces `FxView`, `FxFrames` and `paint_fx`; it is a shell over `Scene`.
- `pixfx-draw` (new): frames, colour, painter, shapes, blending, palettes, outline rings and `math` move out of
  `pixfx-core`; import them from `pixfx_draw` (`pixfx_draw::Frame`, `pixfx_draw::color::Rgba`, ...). `pixfx-core`
  keeps effects, particles, cues, feel and the player. `pixfx-light` depends on `pixfx-draw` only.
- `pixfx-core`, `pixfx-wasm`, `pixfx-cli`, `pixfx-phaser`: every effect takes an `outline` (off, a preset, a hex colour
  or `{color, width, corners}`) and a `palette` by preset name (`fire`, `ice`, `poison`, `holy`, `shadow`, `arcane`,
  `blood`, `electric`, `nature`, `gold`); the CLI takes `--outline` and `--palette`.
- `pixfx-canvas`: the stage's own loop bends the view by the ripples alive; `Stage.warp()` does it for a host that
  owns the loop. `pixfx-core`: `screen::ripple_rgba` bends RGBA pixels at any scale.
- `pixfx-core`, `pixfx-wasm`: `HitFlash` fills a struck sprite for a few frames on play time, held through a hit-stop.
- `site/`: the hero's duel shows each fighter's skill bar on the dais floor: slots 1, 2 and 3 and the ultimate. The
  spells in a slot share one cooldown, which darkens the slot and sweeps back as it runs out; the ultimate's slot
  fills with rage. The icons are generated (OpenRouter `bytedance-seed/seedream-5-5`, with a frame of the duel as
  reference) and pixelated to 32 and 24 px atlases by `site/art/skill-icons.sh`.
- `pixfx-fighter`: the launched, tumble, fall and down poses of both looks are traced from generated art
  (`site/art/fighter-fall.prompt.txt`, Gemini 3 Pro filling the empty cells of a pose sheet) into rect tables in
  `src/traced/`, painted in the look's colours by role (`traced::pose`); these four poses are now still pictures.
- `pixfx-draw`: `trace` cuts a picture from its background, matches it to a palette, shrinks it to sprite pixels,
  merges the pixels into rects and diffs two images. `pixfx-cli`: `pixfx trace` turns a crop of art into a pose
  table, `pixfx diff` counts the pixels two images differ by and writes a mask, and `pixfx poses --blank` leaves
  chosen cells empty for an image model to fill. `pixfx-export`: `write_png_rgba`, `decode_png_rgba` and
  `pack_cells` are public, the one PNG encoder and decoder every file goes through.
- `site/`: the effect page picks a palette preset, an outline and (for impact effects) the feel, and copies the call.
- `pixfx-core`: the trauma shake follows the usual model on every axis: `Feel::roll(strength)` tilts the screen
  (radians, `FeelSpec::roll`) next to `shake`'s x and y, each axis is the most it can move times trauma squared times
  smooth Perlin noise (`math::perlin`, gradient noise that never comes to rest), `cue::impact` adds 0.3 to 0.6 trauma
  per hit, trauma drains 1 a second, and the default shake is 16 game pixels and 0.05 radians at full trauma (a heavy
  hit moves the screen 2 to 4 px, a light one 1 px; it was 1 px for both).
- `pixfx-wasm`, `pixfx-canvas`: `Feel.roll(strength)` and `Stage.roll(strength)`; the Canvas stage's own loop leaves
  roll out to keep the pixel grid. `pixfx-phaser`: `feel.roll`, and the plugin tilts the main camera by it for the
  render, keeping the game's own rotation.
- `pixfx-effects`: each effect is a cargo feature (`fire-candle`), each category a feature that turns on its effects
  (`fire`), and `all` (the default) every one. `pixfx-wasm` and `pixfx-canvas` build with
  `--no-default-features --features pixfx-effects/<effect or category>` to ship only the effects a game plays;
  `EffectId` then lists only those.
- Release builds favour play speed: fat LTO in one codegen unit (opt-level 3), and wasm-pack's `wasm-opt` pass at
  `-O3` for `pixfx-wasm` and `pixfx-canvas`. The 25-effect wasm is 235 KB gzip instead of 246 KB.
- `pixfx-canvas`: `Stage.move(handle, x, y)` moves a playing effect without restarting it, for effects that follow
  the pointer or a sprite.
- `site/`: landing page and docs, with an fx layer (`site/assets/fx.js`) that plays engine effects for the cursor,
  clicks, button presses and page changes as soon as each effect is in the registry. With fx on, pages change in
  place (`site/assets/nav.js`), so the old page burns open from the click point over the new one.
- `pixfx-core`: the simulation core the effects are built on, plus `Painter::clip_local`, `clip_box` and
  `dim_light`.
- `pixfx-effects`: 25 effects, listed in `pixfx_effects::EFFECTS`, each ported from pixelfx and checked by
  `pixfx parity`; the frame-hash determinism test covers all of them and is run on demand
  (`-- --ignored`).
- `pixfx-canvas`: `light_rgba` skips unlit pixels.
- `pixfx-core`: `Painter::disc`, `circle`, `ring` and `light`; `OptValue::Color` and `Options::color_or` for colour
  options; `Options::num_in`, `math::js_sign`, `math::or_one`; `color::step_at` is public.
- `pixfx-wasm`: an extra option may be a hex colour string (`{ stone: "#7a6a58" }`), read as `OptValue::Color`.
- `pixfx-effects`: the number options that size loops (`water-jet` and `laser-bolt` `length`, `small-explosion`
  `size`, `poison-cloud` `spread`) are held to a documented maximum so a huge value cannot hang an effect.
- `pixfx-core`: `Effect::canvas(&Options) -> Canvas` is the canvas an instance draws on (`Meta::canvas` unless the
  effect sizes it from its options); `render_frames`, `Rendered.origin`, the wasm `Player` (`Player::canvas`) and
  `Fx`, the canvas `Stage` and `pixfx-gpui` size and place every instance by it.
- `pixfx-effects`: `weather/fireflies` and `weather/snow` take `width` and `height` options (whole pixels, 1 to
  2048) that size their canvas, as densely filled as their own 320×180, which is still the default and renders
  as pixelfx's. `weather::lib` helpers take a `Canvas`; `sized`, `count` and `MAX_SIDE` are new.
- `pixfx-export`: a sheet's `pivot` is the origin on the canvas the options give (`Rendered.origin + margin`).
- `pixfx-cli`: `read_reference` takes the origin the reference was rendered at.
- `site/`: one fireflies instance the size of the landing scene lights it, so glows no longer stop in straight
  lines where 320-pixel tiles met.

## 0.1.0

- `pixfx-core`: frame with colour and additive light, painter, `Effect`/`Instance`, `render_frames`,
  `composite`, `frame_hash`.
- `pixfx-effects`: `fire/candle`, `fire/campfire`, `fire/brazier`, matching pixelfx with 0 differing pixels.
  Frame-hash determinism test runs natively and in wasm.
- `pixfx-cli`: `pixfx parity`, `pixfx sheet`, `pixfx render --gif`.
- `pixfx-export`: sprite sheets (composite, colour, light PNG), beanfx-layout JSON, GIF.
- `pixfx-wasm`: JS bindings (`effects`, `meta`, `Fx`) and a browser demo.
- `pixfx-canvas`: a Canvas 2D renderer with its own loop (`Stage`), `update` + `drawLayer("back"|"front")` for hosts that draw sprites between an effect's layers, and a browser demo.
- `pixfx-gpui` and `examples/gpui-web`: live playback inside gpui on the web.
- `pixfx-phaser`: Phaser 4 scene plugin playing effects as per-effect images or in scene-wide layers, with a demo.
- `parity/LEDGER.md`: record of approved parity misses (empty).
