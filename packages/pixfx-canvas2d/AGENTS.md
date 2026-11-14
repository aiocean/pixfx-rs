# pixfx-canvas2d

Layer: **Shell** (Canvas 2D, TypeScript). Read the root `AGENTS.md` ("The boundary", "How a frame is made" and
"Layers and where code goes") before changing anything here; this file only says what this package holds.

## Job

`createPixfx(ctx)`: pixfx on a Canvas 2D context, over `pixfx-web` (the JS runtime every JS shell shares). It
decides nothing.

| File | Holds |
|---|---|
| `pixfx.ts` | `Pixfx`: play options, `update(dt)` (stage update, draw list, end of frame), `drawLayer`, `draw`. |
| `canvas-gfx.ts` | The host objects: an `OffscreenCanvas` per surface (colour left, light right) filled with `putImageData`, and the images a scene shows, drawn by depth. |

## Traps

- An image keeps the holder it showed, not its key: a surface Rust frees at the end of the frame still draws this
  frame's pixels.
- `drawLayer` filters by the default depths (back 0, front 1); an effect given other depths shows only in `draw()`.
- `bun run build` is not library mode: library mode inlines the wasm into the module and the worker (twice, as
  base64). `base: "./"` keeps asset URLs relative to the module.
- `pixfx-web` is a `file:` dependency, copied into `node_modules`: `bun run check` runs `bun install` first so it
  tests the current one.

## Does not belong here

| If the code... | It goes to |
|---|---|
| would be the same line in the Phaser or Pixi shell | `pixfx-web` |
| decides placement, layering, timing, which frame shows, what to drop | `pixfx-scene` |
| a feel rule, a default, a preset, a palette | `pixfx-core` / `pixfx-draw`, read through `pixfx-wasm` |

## Rules

TypeScript strict; zod at every boundary; no `any`, no non-null assertions, no type assertions.

## Before you finish

```
(cd ../pixfx-web && bun run build:wasm)   # when Rust changed
bun run check
bun run check:browser
```
