# Changelog

## Unreleased

- `pixfx-core`: the simulation core the effects are built on, plus `Painter::clip_local`, `clip_box` and
  `dim_light`.
- `pixfx-effects`: 25 effects, listed in `pixfx_effects::EFFECTS`, each ported from pixelfx and checked by
  `pixfx parity`; the frame-hash determinism test covers all of them and is run on demand
  (`-- --ignored`).
- `pixfx-canvas`: `light_rgba` skips unlit pixels.

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
