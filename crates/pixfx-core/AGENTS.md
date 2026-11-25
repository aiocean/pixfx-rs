# pixfx-core

Layer: **Engine**. Read the root `AGENTS.md` ("The boundary" and "Layers and where code goes") before changing
anything here; this file only says what this crate holds.

## Job

The effect model and game feel, painted with `pixfx-draw`:

- Effect model: `effect` (`Effect`, `Instance`, `Options`, `Meta`, `Placement`), `particles`, `sim`, `render`
  (`render_frames`, `edge_fade`), `player` (`Player`: one effect on its own canvas), `key` (`ClipKey`, `FrameId`),
  `frames` (`FrameStore`, `FRAME_BUDGET`), `state` (`Creature`), `light_color` (the colour an effect lights its
  surroundings in).
- Orders: `volley` (`Order` with the caster's body, `Aim`, `Launch`, `Tail`, and `Volley::aim`, which plans
  like pieces toward a point along a `pixfx_draw::path::Path`: converge, pierce, or orbit (round the point
  `orbit_turns` times on a ring, then in), each held first, flying on past the point, with a tail). An effect
  answers `Effect::order`; `pixfx-scene` flies what it returns and strikes it on bodies. `Effect::loop_from`
  lets a loop open once and then repeat only what follows.
- Motion: `trace` (`Trace`, `TracePoint` with its angle, `steps`) and `motion` (`at`, `velocity`, `speed`,
  `heading`, `turn`, `spin`, `acceleration`, `poses` for `pixfx_draw::sweep`): how a moving thing moves, read
  once for every effect. An effect asks for its trace with `Effect::traces` and takes it in `set_trace`.
- Sprite awareness: `actor` (`Actor`: the sprite an effect wraps, `opaque()` its pixels) and `silhouette`
  (`Silhouette`, `Area`: the body in the effect's cells, its edge, an outline ring, the cells round it by distance,
  a random cell of it). Every effect that reads its sprite goes through these, so a new query lands here once.
- Game feel: `cue` (`Cue`, `TimedCue`, `impact`), `feel` (`Feel`: shake, hit-stop, slow motion, flash, ripples),
  `squash` (`Squash`, `Blink`), `hit_flash`, `screen` (shake, ripple and flash passes).

## Belongs here

- A rule every game and every shell answers the same way: a new cue, a feel rule, a squash behaviour, a change to
  how an effect is stepped or rendered.
- A field on `Options` or `Placement`. `key.rs` destructures both, so the new field does not compile until it is
  encoded in `ClipKey`; encode it, do not silence it.

## Does not belong here

| If the code... | It goes to |
|---|---|
| paints pixels in a way an effect does not need to know about (a primitive, blend, palette) | `pixfx-draw` |
| is one effect | `pixfx-effects`, behind its own feature |
| is a fighter's body, pose or look | `pixfx-fighter` |
| decides which effects play, where, on which layer, in what order, or frees frames for a shell | `pixfx-scene` |
| is a `wasm-bindgen` type, a JS option parser, or reads a clock, a file or the DOM | `pixfx-wasm` or a shell |

## Dependencies

- Required: `pixfx-draw`, `libm`.
- Optional, behind `serde` / `ts`: `serde`, `tsify`, `wasm-bindgen`.
- Never `pixfx-effects`, `pixfx-scene`, `pixfx-wasm` or a shell: they sit above this crate.
- No re-exports of `pixfx-draw` items: callers import from `pixfx_draw` directly.

## Traps

- Pure and deterministic: no clock, no I/O, no host types. Time comes in as `dt`; check it with `valid_dt`.
  Randomness is seeded and passed in.
- A hit-stop is `dt = 0`: nothing may advance and no velocity may spike from dividing by it.
- Float maths through `libm` (`clippy.toml`); rounding through `pixfx_draw::math::js_round`.

## Character rig

The `body` feature provides poses, cameras, animation clips and mannequin painting. See [body instructions](src/body/AGENTS.md).

## Scene light

Scene light: the glow a spell, a blow or a lamp throws on the world. A `Light` is a value that ages (`step`), shows
as a `Glow` at any instant (`Light::glow`), and `draw` adds glows to a frame's light layer in hard-edged rings. How
a light swells, fades and rings is data (`Style`). Enable the `light` feature.

The caller keeps the live lights and passes time in. The game chooses which events cast light.
The ring blend matches Canvas 2D `lighter` with 8-bit channels. Preserve its rounding.

## Before you finish

```
cargo test -p pixfx-core --features body,light
cargo test -p pixfx-effects --test determinism -- --ignored   # when a change can move pixels: every effect's hashes
```
