# Changelog

## Unreleased

- `pixfx-core`, `pixfx-wasm`, `pixfx-cli`, `pixfx-phaser`: every effect takes an `outline` (off, a preset, a hex colour
  or `{color, width, corners}`) and a `palette` by preset name (`fire`, `ice`, `poison`, `holy`, `shadow`, `arcane`,
  `blood`, `electric`, `nature`, `gold`); the CLI takes `--outline` and `--palette`.
- `pixfx-canvas`: the stage's own loop bends the view by the ripples alive; `Stage.warp()` does it for a host that
  owns the loop. `pixfx-core`: `screen::ripple_rgba` bends RGBA pixels at any scale.
- `pixfx-core`, `pixfx-wasm`: `HitFlash` fills a struck sprite for a few frames on play time, held through a hit-stop.
- `site/`: the hero's duel shows each fighter's skill bar on the dais floor: slots 1, 2 and 3 and the ultimate. The
  spells in a slot share one cooldown, which darkens the slot and sweeps back as it runs out; the ultimate's slot
  fills with rage.
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
