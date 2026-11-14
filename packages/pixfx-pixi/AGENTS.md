# pixfx-pixi

Layer: **Shell** (Pixi 8, TypeScript). Read the root `AGENTS.md` ("The boundary", "How a frame is made" and "Layers
and where code goes") before changing anything here; this file only says what this package holds.

## Job

`createPixfx(app)`: pixfx in a Pixi application, over `pixfx-web` (the JS runtime every JS shell shares). It
decides nothing.

| File | Holds |
|---|---|
| `pixfx.ts` | `Pixfx`: the ticker step and the after-render, play options, the shake on a camera container, the flash. |
| `pixi-gfx.ts` | The host objects: atlas pages on WebGL (`texSubImage2D` through Pixi's own binding), a buffer texture per surface otherwise, sprites. |

## Traps

- The atlas binds a page through `renderer.texture.bindSource` and puts `UNPACK_PREMULTIPLY_ALPHA_WEBGL` back as it
  found it: Pixi caches both, and a direct `gl.bindTexture` or a changed flag would desync it.
- A region's `Texture` is resized in place (`frame`, `orig`, `update()`), never replaced: a sprite may still show it.
- `pixfx-web` is a `file:` dependency, copied into `node_modules`: `bun run check` runs `bun install` first so it
  tests the current one.

## Does not belong here

| If the code... | It goes to |
|---|---|
| would be the same line in the Phaser shell | `pixfx-web` |
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
