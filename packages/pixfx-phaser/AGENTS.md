# pixfx-phaser

Layer: **Shell** (Phaser 4, TypeScript). Read the root `AGENTS.md` ("The boundary", "How a frame is made" and
"Layers and where code goes") before changing anything here; this file only says what this package holds.

## Job

A Phaser 4 scene plugin: `scene.pixfx.play(id, x, y, opts)` with follow, stop, `complete` / `destroy` events,
`scene.pixfx.feel` and `scene.pixfx.squash(sprite)`. It loads the engine through `pixfx-wasm` and follows the draw
list the wasm `Scene` hands it each frame: make, refill, show and free surfaces. It runs on `pixfx-web` (the JS runtime
every JS shell shares: wasm, workers, frames, surfaces, stage, effect, feel) and adds Phaser's host objects, its
events and its camera. It decides nothing.

## A shell may only

Create and free Phaser textures and images, copy pixels Rust painted into them, place them where Rust says, apply
the Rust `Feel` to the camera, load wasm, run workers, and parse options at the boundary with zod.

## Files

| File | Holds |
|---|---|
| `plugin.ts` | The scene plugin: Phaser events in, `Stage` (pixfx-web) calls out; the camera's view handed to the Rust scene each frame; the camera shake and flash. |
| `phaser-gfx.ts` | The host objects: atlas pages on WebGL, canvas textures on the Canvas renderer, images placed by scroll factor. |
| `sprite-feel.ts` | The Rust `Squash`/`Blink` applied to sprites. |
| `options.ts` | Phaser's own play options (`scrollFactor`, `depth`), split from the engine's by pixfx-web's `splitPlayOptions`. |

## Traps

- Phaser's frame UVs assume a texture stored bottom row first. The atlas is made with `flipY` and filled with
  `flipY: true` at row `size - y - h`; filled any other way the regions show upside down or mixed.
- `Game.endFrame()` runs once per host frame after every scene drew (game `POST_RENDER`), never per scene.
- The camera's `preRender` (follow, bounds) runs after the plugin draws, so `worldView` is a frame old then: the
  plugin works the view out from the camera's scroll, size, zoom and tilt (`cameraView`). Only a following camera
  still moves after that; an effect it brings into view shows a frame later.
- An effect's scroll factor goes to the Rust scene too (`setScrollFactor`): the view culls an effect where the
  camera shows it, so a HUD effect (`0`) is seen wherever the camera scrolls.
- `pixfx-web` is a `file:` dependency, copied into `node_modules`: `bun run check` runs `bun install` first so it
  tests the current one.

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
bun run build:wasm    # when Rust changed (builds pixfx-web's wasm)
bun run check         # tsc, eslint, bun test
bun run perf:check
```
