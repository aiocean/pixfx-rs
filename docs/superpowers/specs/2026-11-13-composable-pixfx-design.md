# Composable pixfx — design

Date: 2026-11-13 · Asked by the owner: "make it generic, smart, long-term and easy to customise; follow the Unix
and functional-programming principles, so the parts combine and talk to each other easily".

## Why

Building the site's duel on pixfx showed where the engine resists a game (findings of 2026-11-13):

1. A playing effect cannot be changed: a reflected bolt had to be stopped and replayed to turn round.
2. `pixfx-fighter` knows two characters: looks are picked by the names `"thunder"` and `"frost"`; status layers
   assume a standing body; tuning lives in ~45 hard-coded constants.
3. The JS boundary reads objects loosely: a missing or misspelt field silently becomes 0.
4. Three source-over blends exist (`Painter::blend_px`, `status::over`, `paint::over`), one of them off the
   canvas by up to 17 per channel; `js_max(0, -0)` is `-0` where JS gives `+0`.
5. Effect ids and options are strings checked at runtime; there are no types.
6. The rules duplicate the body's constants (`WALK`, `CHEST`, …) because the binding does not expose them.
7. Light that spills from effects onto the scene is the game's own code (`site/assets/duel-light.js`).

## Principles

The Unix and FP ideas, made concrete for this codebase:

- **Values everywhere.** Every input and output of every part is plain data: specs (tuning), looks (palettes),
  appearances (what to draw), marks, cues and events. One shape in Rust and in JS (serde), so any part can feed any
  other, a game can store, diff, log or replay them, and a test is "value in, value out".
- **Pure functions over those values, state passed explicitly.** `body.step(&spec, input) -> events`,
  `paint(&look, &appearance) -> sprite`, `feel.react(&spec, cues)`. No hidden globals, no clocks, randomness from
  the caller. The shells (canvas, wasm) are the only places with I/O.
- **One job per part.** body moves, feel answers blows on screen, paint draws a body, status draws what hangs on it,
  light spills light, blend mixes pixels, stage places effects. Parts share data types, never internals, so any
  one is usable alone (a game may use the body without the painter, the painter with its own physics).
- **Defaults are data, overrides are data.** Each part has `Spec::default()`; JS reads it (`Body.defaults()`)
  and passes back a partial override (`{gravity: 400}`); the rules read the same numbers instead of copying them.
- **Parse once, strictly, at the boundary.** Unknown or misspelt fields and wrong types are errors that name the
  field. Inside, the types are trusted.
- **Events out as data, pulled.** Parts return or queue events (`land`, `bump`, effect `end`); the host drains
  them. No callbacks into JS from the core: the order of effects stays deterministic.
- **Composition by the host, not by inheritance.** A game wires parts together with plain calls and data; no part
  owns another.

## Pieces

### pixfx-core
- `blend`: the one canvas-exact source-over (8-bit alpha, premultiplied rounding as Chrome's raster does), with
  `fill_rect(frame, x, y, w, h, color, alpha)`. paint, status and dust use it.
- `math`: `js_max`/`js_min` give JS's signed-zero results. Effect goldens and parity must stay as they are.
- `Instance::set(Placement { x, y, flip, angle, light })`: change where and how a playing effect is drawn without
  restarting it. (A palette is resolved at spawn; changing it means playing anew.)

### pixfx-canvas Stage
- `stage.set(handle, {x?, y?, flip?, angle?, light?})` (keeps `move` as is).
- `stage.events()` drains `{type: "end", handle}` for effects that finished or were removed.

### pixfx-fighter
- `BodySpec` (all body tuning, `Default` = today's numbers), `FeelSpec` likewise; body and feel take the spec.
- `Look` is data: any palette, parsed from JS; `thunder`/`frost` are presets.
- `Marks` is a list of mark values drawn in order; burning, shocked and frozen layers follow a `Silhouette` taken
  from the pose (standing, crouched, lying, airborne, tumbling).
- All values serde, with `#[serde(deny_unknown_fields)]`.

### pixfx-light (new)
- The scene light from `duel-light.js`, generic: lights are values (`{x, y, reach, color, strength, life}`), a
  pure `step` ages them and `draw` adds hard-edged rings into a frame's light layer.

### Boundary and types
- Every binding parses through serde (`serde-wasm-bindgen`) into the Rust types; errors name the field.
- TypeScript declarations are generated from the same Rust types, including a union of every effect id.

### The site
- `duel.js` reads the body's numbers from `Body.defaults()`; the duplicated constants go.
- The reflected bolt is played anew: it takes the reflector's palette, and a palette is fixed at spawn. `stage.set` is
  for changes that keep the effect's look (placement, flip, angle, light).
- Looks come from data (`look` presets), and `duel-light.js` becomes calls into pixfx-light.

## Verification

- Same fight: the 6-seed step trace stays identical (sha `c7661c827f40…`) and the 80 fights stay 104 to 118.
- Same pictures: effect goldens and parity unchanged; the fighter drawing, marks and dust unchanged against the
  pre-change engine in Chrome.
- Boundary: a misspelt field raises an error naming it (one test per binding family, not per field).
- Chrome on the built site: 60 fps, no errors; then merge to main and deploy.
