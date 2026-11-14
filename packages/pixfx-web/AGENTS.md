# pixfx-web

Layer: **JS runtime** (TypeScript). Read the root `AGENTS.md` ("The boundary", "How a frame is made" and "Layers
and where code goes") before changing anything here; this file only says what this package holds.

## Job

What every JS shell (Phaser, Pixi, the next ones) shares, so a shell adds only its host objects and its loop:

| File | Holds |
|---|---|
| `load.ts`, `engine.ts` | Loading the wasm once and starting the painting workers; the engine's read-only data. |
| `game.ts` | The game-wide wasm `Frames`, the surfaces, jobs routed to workers (one per effect for its life), painted frames handed back. |
| `surfaces.ts` | The draw-list op codes; makes, refills and frees surfaces on the shell's `GfxTexture`s. |
| `stage.ts` | One host scene's wasm `Scene`: plays, follow targets read each update, events, the show ops on the shell's images. |
| `effect.ts`, `feel.ts` | `PixfxEffect` and `PixfxFeel`, the handles a game holds (eventemitter3). |
| `worker.ts`, `paint.ts`, `protocol.ts` | The painting worker, what it does with a message, and the messages (zod). |
| `options.ts` | Splitting play options into a shell's own fields and the engine's. |
| `gfx.ts` | The contract a shell implements: `GfxTextures` (room for a surface) and `GfxImages` (images showing one). |
| `tools/cdp.ts` | A Chrome DevTools Protocol client for the shells' browser checks. |

A shell's per-effect placement beyond depth is the type parameter `P` (`Stage<P>`, `PixfxEffect<P>`,
`GfxImage<P>`): Phaser's scroll factor, `null` where a shell has none.

## Does not belong here

| If the code... | It goes to |
|---|---|
| imports Phaser, Pixi or another host | that host's shell package |
| decides placement, layering, timing, which frame shows, what to drop | `pixfx-scene` |
| a feel rule, a default, a preset, a palette | `pixfx-core` / `pixfx-draw`, read through `pixfx-wasm` |

## Rules

TypeScript strict; zod at every boundary; no `any`, no non-null assertions, no type assertions.

## Before you finish

```
bun run build:wasm    # when Rust changed
bun run check         # tsc, eslint, bun test
```

Then `bun run check` in every shell package that depends on this one.
