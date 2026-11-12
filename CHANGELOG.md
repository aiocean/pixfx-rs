# Changelog

## 0.1.0

- `pixfx-core`: frame with colour and additive light, painter, `Effect`/`Instance`, `render_frames`,
  `composite`, `frame_hash`.
- `pixfx-effects`: `fire/candle`, `fire/campfire`, `fire/brazier`, matching pixelfx with 0 differing pixels.
  Frame-hash determinism test runs natively and in wasm.
- `pixfx-cli`: `pixfx parity`, `pixfx sheet`, `pixfx render --gif`.
- `pixfx-export`: sprite sheets (composite, colour, light PNG), beanfx-layout JSON, GIF.
- `pixfx-wasm`: JS bindings (`effects`, `meta`, `Fx`) and a browser demo.
- `pixfx-canvas`: a Canvas 2D renderer with its own loop (`Stage`) and a browser demo.
- `pixfx-gpui` and `examples/gpui-web`: live playback inside gpui on the web.
- `parity/LEDGER.md`: record of approved parity misses (empty).
