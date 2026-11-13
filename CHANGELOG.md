# Changelog

## Unreleased

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
