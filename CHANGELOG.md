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
