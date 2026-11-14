# pixfx-phaser

Layer: **Shell** (Phaser 4, TypeScript). Read the root `AGENTS.md` ("The boundary", "How a frame is made" and
"Layers and where code goes") before changing anything here; this file only says what this package holds.

## Job

A Phaser 4 scene plugin: `scene.pixfx.play(id, x, y, opts)` with follow, stop, `complete` / `destroy` events,
`scene.pixfx.feel` and `scene.pixfx.squash(sprite)`. It loads the engine through `pixfx-wasm` and follows the draw
list the wasm `Scene` hands it each frame: make, refill, show and free surfaces. It decides nothing.

## A shell may only

Create and free Phaser textures and images, copy pixels Rust painted into them, place them where Rust says, apply
the Rust `Feel` to the camera, load wasm, run workers, and parse options at the boundary with zod.

## Files

| File | Holds |
|---|---|
| `plugin.ts` | The scene plugin: Phaser events in, `Stage` calls out; the camera shake and flash. |
| `stage.ts` | One Phaser scene's wasm `Scene`: plays, follow targets read each frame, events, the show ops on images. |
| `effect.ts` | `PixfxEffect`, the handle a game holds; its events. |
| `game.ts` | The game-wide wasm `Frames`, the surfaces, and routing jobs to workers and painted frames back. |
| `surfaces.ts` | The draw-list op codes; makes, refills and frees surfaces on `GfxTexture`s. |
| `gfx.ts`, `phaser-gfx.ts` | The host objects: atlas pages on WebGL, canvas textures on the Canvas renderer, images. |
| `worker.ts`, `paint.ts`, `protocol.ts` | The painting worker, what it does with a message, and the messages (zod). |
| `load.ts`, `engine.ts` | Loading wasm and the workers; the engine's read-only data (effects, meta, presets). |
| `feel.ts`, `sprite-feel.ts` | The Rust `Feel` and `Squash`/`Blink` applied to the camera and to sprites. |
| `options.ts` | zod parsing of play options at the boundary; the engine checks the rest. |

## Traps

- Phaser's frame UVs assume a texture stored bottom row first. The atlas is made with `flipY` and filled with
  `flipY: true` at row `size - y - h`; filled any other way the regions show upside down or mixed.
- `Game.endFrame()` runs once per host frame after every scene drew (game `POST_RENDER`), never per scene.
- An effect's state lives in one worker for its life; its `ended` goes in the same message as its last job, and the
  worker paints the jobs before it lets go.

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
