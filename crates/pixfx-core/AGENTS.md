# pixfx-core

Layer: **Engine**. Read the root `AGENTS.md` ("The boundary" and "Layers and where code goes") before changing
anything here; this file only says what this crate holds.

## Job

The effect model and game feel, painted with `pixfx-draw`:

- Effect model: `effect` (`Effect`, `Instance`, `Options`, `Meta`, `Placement`), `particles`, `sim`, `render`
  (`render_frames`, `edge_fade`), `player` (`Player`: one effect on its own canvas), `key` (`ClipKey`, `FrameId`),
  `frames` (`FrameStore`, `FRAME_BUDGET`), `state` (`Creature`).
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
| paints pixels in a way an effect does not need to know about (a primitive, blend, palette, outline) | `pixfx-draw` |
| is one effect | `pixfx-effects`, behind its own feature |
| is a fighter's body, pose or look | `pixfx-fighter` |
| is scene light (glows thrown on the world) | `pixfx-light` |
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
- Effects are ports of pixelfx with JS number semantics. "Simplifying" maths here breaks parity.

## Before you finish

```
cargo test -p pixfx-core
cargo test --workspace                                        # golden hashes of every effect
cargo test -p pixfx-effects --test determinism -- --ignored
```
