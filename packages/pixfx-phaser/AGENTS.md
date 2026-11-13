# pixfx-phaser

Layer: **Shell** (Phaser 4, TypeScript). Read the root `AGENTS.md` ("The boundary" and "Layers and where code goes")
before changing anything here; this file only says what this package holds.

## Job

A Phaser 4 scene plugin: `scene.pixfx.play(id, x, y, opts)` with follow, stop, `complete` / `destroy` events,
layers, `scene.pixfx.feel` and `scene.pixfx.squash(sprite)`. It loads the engine through `pixfx-wasm`.

## A shell may only

Create and free Phaser textures and images, copy pixels Rust painted into them, place them where Rust says, apply
the Rust `Feel` to the camera, load wasm, run workers, and parse options at the boundary with zod.

## Stays here

`plugin.ts`, `effect.ts`, `gfx.ts`, `surface.ts` (textures and images); `feel.ts` (applies `Feel` to the camera);
`pool.ts`, `worker.ts`, `worker-core.ts`, `protocol.ts`, `backend.ts`, `inline.ts`, `load.ts`, `engine.ts`,
`clock.ts` (wasm, workers, transport); the zod parsing in `options.ts`.

## Known drift: decisions written in TypeScript

These leave for `pixfx-scene` and the engine behind `pixfx-wasm` (item 2 of "Out of place today" in the root
`AGENTS.md`). Do not add logic to them; a fix that needs logic goes into Rust.

- `pixels.ts` (`compositeColor`, `compositeLight`, `snap`): a copy of the Rust compositing.
- `layer.ts`, `layout.ts`: scene-wide layer compositing and placement.
- `lifecycle.ts`, `effects.ts`: effect state (playing, stopping, destroyed), follow, per-scene ticking.
- `runner.ts`: picks the frame from `floor(t × fps)`.
- `sprite-feel.ts`: velocity from distance moved.
- `options.ts` `PALETTE_PRESETS`: a copy of `palette::PRESETS`.
- `frames.ts`: its own frame cache (budget, drops, crops, sharing).

## Does not belong here

| If the code... | It goes to |
|---|---|
| decides placement, layering, timing, which frame shows, what to drop | `pixfx-scene` |
| a feel rule, a default, a preset, a palette | `pixfx-core` / `pixfx-draw`, read through `pixfx-wasm` |
| pixel maths | `pixfx-draw` |

The test: a Pixi shell would have to write the same line. If yes, it is not shell code.

## Rules

TypeScript strict; zod at every boundary; no `any`, no non-null assertions, no type assertions.

## Before you finish

```
bun run perf          # before the change, and again after; put both in the commit message
bun run build:wasm    # when Rust changed
bun run check         # tsc, eslint, bun test
bun run perf:check
```
