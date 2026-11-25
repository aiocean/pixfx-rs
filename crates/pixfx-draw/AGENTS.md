# pixfx-draw

Layer: **Drawing**, the lowest layer. Read the root `AGENTS.md` ("The boundary" and "Layers and where code goes")
before changing anything here; this file only says what this crate holds.

## Job

Pixels and nothing else: `Frame` with a colour layer and an additive light layer, the `painter`, `shapes`, `halo`
(a glow painted in solid steps round strokes, as pixel artists draw it), `blend` and `composite`, `frame_hash`,
`sweep` (a `Brush` of cells swept along poses, coloured by age: every streak, trail and smear; the brush is the
mover's cross-section: `column` for a thin mover, `edge` for a broad body's back edge, `from_column` for a turning
blade), `color` (`Rgba`,
`hex`), `palette` presets, `RgbaImage` (plain RGBA bytes: a sprite leaving the engine or a picture
coming in), the JS-number maths in `math` (`js_round`, `js_max`, `js_min`, `clamp`), and `ruler` (`Ruler`: a
character's measurements in pixels; `RULER`: the duel's fighter, the scale every sprite, effect and reference sheet
is drawn to, here so every crate reaches it; `Ruler::size` sizes a thing against a body part). Every other crate
paints through it. `growth` grows branching trees for veins, lightning, cracks and roots: `dbm` (dielectric breakdown)
and `colonize` (space colonization), `Tree::path` for a path you set, `murray` for fork angles, and `radii` with
`stroke` for tapered, shaded strokes. Every setting is a field. `path` (`Path`): a way from one place to another
made of lines, quadratic curves and turns round a centre (circle arcs, spirals), read by pixels along it (`at`,
`heading`, `points`): a volley's flight (`pixfx_core::volley`) and growth laid round a ring (`Tree::path`).

## Belongs here

- A new way to put pixels in a frame: a primitive, a blend, a palette. `Painter::sprite` stamps straight-alpha
  RGBA8 frames in local coordinates with nearest-neighbour sampling and ordered dithering.
- Sprites, text, nine-slice and UI layout when a real game needs them. That game decides the API; do not add one
  ahead of it.
- Number helpers that must give identical bits natively and in wasm.

## Does not belong here

| If the code... | It goes to |
|---|---|
| knows what an effect, a particle, an option or a cue is | `pixfx-core` / `pixfx-effects` |
| decides where something is placed, on which layer, or which frame shows | `pixfx-scene` |
| knows a host (`web-sys`, `js-sys`, gpui, files, a clock) | a shell |
| is a JS-facing type or option parser | `pixfx-wasm` |

## Dependencies

- Required: `libm` only. Nothing from this workspace: every other crate sits above this one.
- Optional, behind the `serde` / `ts` features only: `serde`, `tsify`, `wasm-bindgen`.
- No `std::time`, no I/O, no randomness that the caller does not pass in.

## Traps

- Float maths goes through `libm`, never `f64::sin` and friends; `clippy.toml` denies them so wasm and native give
  the same bits. Rounding is `math::js_round` (JS `Math.round`), not `f64::round`.
- The light word's top byte is unused (`composite_px` masks it). Never start reading it as alpha.
- Every effect's pixels go through this crate. Any change here can move every `frame_hash`.

## Before you finish

```
cargo test -p pixfx-draw
cargo test -p pixfx-effects --test determinism -- --ignored   # when a change can move pixels: every effect's hashes
```

A changed `frame_hash` without a stated reason is a failure, even if the result looks right.
