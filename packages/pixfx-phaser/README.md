# pixfx-phaser

Play pixfx effects in Phaser 4 scenes. Effects run in the pixfx engine (Rust compiled to WebAssembly) and
show up as ordinary Phaser images: a back and a front pair, each a colour image and an additive light image,
so your sprites can stand between the two.

Requires Phaser `^4.2.1`.

## Install

The package is not on npm yet. Build the engine once, then link the package into your game:

```sh
cd pixfx-rs/packages/pixfx-phaser
bun install
bun run build:wasm          # wasm-pack → wasm/ (git-ignored), shipped beside the JS
cd /path/to/your-game
bun add ../pixfx-rs/packages/pixfx-phaser
```

The package exports its TypeScript source (`src/index.ts`); Vite compiles it with the game. The `.wasm` file
sits next to its JS glue and is loaded through `new URL(..., import.meta.url)`, so Vite bundles it with no
plugin.

A local package carries its own dev copy of Phaser in its `node_modules`. Tell Vite to use the game's copy,
or the bundle holds two Phasers (2.8 MB instead of 1.5 MB, measured):

```js
// vite.config.js of the game
export default { resolve: { dedupe: ["phaser"] } };
```

## Load the engine and register the plugin

```ts
import Phaser from "phaser";
import { PixfxPlugin, loadPixfx } from "pixfx-phaser";

await loadPixfx(); // once, before any scene calls play()

new Phaser.Game({
  pixelArt: true,
  zoom: 4,
  scene: [MyScene],
  plugins: { scene: [{ key: "pixfx", plugin: PixfxPlugin, mapping: "pixfx" }] },
});

class MyScene extends Phaser.Scene {
  declare pixfx: PixfxPlugin;
  create() {
    this.pixfx.play("fire/campfire", 150, 170);
  }
}
```

`play()` before `loadPixfx()` has resolved throws `pixfx: call await loadPixfx() before play()`.

### Worker threads

`loadPixfx()` starts worker threads that draw the effects, one per core the main thread leaves free (1 to 4),
each with its own copy of the engine. The pixels are exactly those the engine draws on the main thread; a new
frame shows one display frame after the scene step that asked for it, and `"complete"` fires when the worker
reports the effect finished. Each effect stays in one worker for its life. `play()` still validates the id and
options on the main thread and throws at once.

```ts
await loadPixfx({ workers: 2 }); // a fixed number of workers
await loadPixfx({ workers: 0 }); // draw on the main thread
```

Without `Worker` support effects draw on the main thread. Options apply to the first call; later calls return
the loaded engine. The worker is bundled by Vite from `new Worker(new URL(...), { type: "module" })`, with no
plugin.

## Play an effect

`scene.pixfx.play(id, x, y, opts?)` places the effect's origin at world `(x, y)` and returns a `PixfxEffect`.
One effect pixel is one world unit; camera zoom and `pixelArt` enlarge it, and positions snap to whole pixels.
An unknown id or a bad option throws and creates nothing.

Options (all optional):

| Option | Meaning |
|---|---|
| `seed` | Non-negative integer; same seed and same steps give the same pixels. |
| `flip`, `angle`, `palette` | Passed to the effect (see `meta(id)` in `pixfx-wasm`). |
| `light` | Glow strength, a number: `0` is no glow, default `1`. |
| `depth: {back, front}` | Depths of the back and front images. Default `{back: 0, front: 1}`. |
| `scrollFactor` | Camera scroll factor of the images: `1` world space (default), `0` screen space (HUD effects). |

Other numeric or array options are passed through to the effect.

`PixfxEffect`:

| Call | Effect |
|---|---|
| `stop()` | Let it end: loops fade out, one-shots are cut short. Does nothing once finished. |
| `destroy()` | Remove at once. |
| `follow(obj, dx?, dy?)` | Track a game object each frame. If `obj` is destroyed, the effect stops (fades) where `obj` last was. |
| `setDepths(back, front)` | Move the image pairs in the display list; put your sprite at a depth between them. |
| `setPosition(x, y)` | Move; stops following. |

Events: `"complete"` when the effect finishes on its own (after `stop()`, or at the end of a one-shot), and
`"destroy"` after `destroy()` (never both). When either fires, the effect's textures, images and engine
instance are already freed.

Effects advance on scene time: the scene `update` delta × `scene.time.timeScale`. A paused or sleeping scene
freezes them; `timeScale` 0.25 plays them at quarter speed. Scene shutdown (`restart`, `stop`) or destroy
frees every effect. `scene.pixfx.stats()` returns `{engines, textures, images}` that are still alive.

## Layer mode

Every per-effect object holds four textures. For many effects in one region, a layer shares four textures
between them:

```ts
const layer = this.pixfx.createLayer({
  bounds: { x: 290, y: 0, width: 110, height: 200 },
  depths: { back: 0, front: 1 }, // optional
  scrollFactor: 1,                // optional
});
layer.play("fire/brazier", 315, 180);
layer.play("fire/brazier", 370, 160);
```

`layer.play` returns a `PixfxEffect` with the same API and events. The layer owns the depths and scroll factor
(`setDepths` on a member does nothing). Members are drawn clipped to `bounds`, including their glow, so make
the bounds cover the light you want to see. `layer.destroy()` destroys every member and the layer's textures.

## Limitation: transparent render targets

Light images use Phaser's `BlendModes.ADD`. In Phaser 4.2.1's WebGL renderer that is `src·1 + dst·dstAlpha`:
over an opaque scene it adds exactly the light, but over a transparent render target (a transparent canvas, or
a transparent `RenderTexture`) it adds nothing. Draw effects over an opaque background.

## Demo

```sh
bun run build:wasm
bunx vite demo                   # dev server; open the printed URL
bunx vite build demo             # or build to demo/dist and serve that
```

`?renderer=webgl` (default) or `?renderer=canvas`. Keys: `P` pause/resume, `S` time scale 1 ↔ 0.25,
`X` stop the campfire, `D` destroy the walker the candle follows, `R` restart the scene.
`window.pixfxDemo` exposes `scene`, `game`, `engine`, `stats()` and the `events` log for checks.

## Develop

```sh
bun run check   # tsc, eslint, bun test
```
