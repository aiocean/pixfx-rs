# pixfx-pixi

Play pixfx effects in a Pixi 8 application. Effects run in the pixfx engine (Rust compiled to WebAssembly), are
painted on worker threads, and show up as Pixi sprites: a back and a front pair, each a colour sprite and an
additive light sprite, so your own sprites can stand between the two.

Requires `pixi.js` `^8.22.0`.

## Install

The package is not on npm yet. Build the engine once, then link the package into your game:

```sh
cd pixfx-rs/packages/pixfx-web && bun install && bun run build:wasm   # wasm-pack → pixfx-web/wasm
cd ../pixfx-pixi && bun install
cd /path/to/your-game && bun add ../pixfx-rs/packages/pixfx-pixi
```

pixfx-web (the runtime every pixfx JS shell shares) loads its wasm and its worker through
`new URL(..., import.meta.url)`. Tell Vite to serve it as source, not pre-bundled:

```js
// vite.config.js of the game
export default { optimizeDeps: { exclude: ["pixfx-web"] } };
```

## Use

```ts
import { Application, Container } from "pixi.js";
import { createPixfx, loadPixfx } from "pixfx-pixi";

await loadPixfx(); // once, before createPixfx()
const app = new Application();
await app.init({ width: 1344, height: 720, background: "#17131c", roundPixels: true });

const world = app.stage.addChild(new Container());
world.scale.set(4);                          // one effect pixel is 4 screen pixels, nearest-neighbour
const fx = world.addChild(new Container());  // the effects' sprites go here, sorted by zIndex
const pixfx = createPixfx(app, { container: fx, camera: world });

pixfx.play("fire/campfire", 168, 160);
const hero = { x: 100, y: 142, active: true };  // anything with x, y and active
pixfx.play("fire/candle", 0, 0).follow(hero);
pixfx.feel.on("cue", (c) => { if (c.cue.type === "sound") playSound(c.cue.name); });
```

`createPixfx(app, { container?, camera? })`:

| Option | Meaning |
|---|---|
| `container` | Where the effects' sprites go; set to sort children by `zIndex`. Default: a new container on `app.stage`. |
| `camera` | What the feel's shake moves and tilts (about the screen's centre), put back after each render. Default: `app.stage`. |

`pixfx.play(id, x, y, opts?)` places the effect's origin at `(x, y)` in `container` and returns a `PixfxEffect`.
Options: `depth: {back, front}` sets the back and front sprites' `zIndex` (default `{back: 0, front: 1}`); every
other option (`seed`, `flip`, `angle`, `light`, `palette`, the effect's own) is the engine's, as in
`pixfx-phaser`. An unknown id or a bad option throws and creates nothing.

`PixfxEffect`: `stop()`, `destroy()`, `follow(target, dx?, dy?)`, `setDepths(back, front)`, `setPosition(x, y)`,
`wrap(source)`; events `"complete"`, `"destroy"` and `"cue"`, as in `pixfx-phaser`.

`effect.wrap(pixfx.actorOf(sprite))` plays the effect on a character: an effect that wraps its actor
(`feedback/power-up`) grips the sprite's outline and follows every texture it shows, at its scale (a negative x
scale mirrors it). Its rotation, and the scale of containers between it and `container`, are not read.

Effects advance on `app.ticker` (after the game's own ticker callbacks, before the render); `ticker.speed` scales
them and `ticker.stop()` freezes them. `pixfx.feel` is the same `PixfxFeel` as Phaser's: the shake moves `camera`,
the flash covers the screen, `feel.timeScale` freezes the game's own world in a hit-stop. `pixfx.stats()` counts
effects, textures and sprites; `pixfx.destroy()` ends every effect and frees its sprites. `pixfx.setDrawBudget(n)`
draws at most `n` effect layers per frame (the rest keep their last frame and wait their turn); 0, the default, draws
every one that changed.
When your world is larger than the screen, `pixfx.setView(x, y, w, h)` says which rectangle of it (game pixels) your
camera shows: effects outside it play on undrawn and cost no painting. `pixfx.clearView()` draws them all again.

On WebGL every frame is a region of a few 4096² atlas pages, filled straight from the bytes the workers painted;
on WebGPU each frame is a buffer texture.

## Demo and checks

```sh
bunx vite demo             # campfire, braziers, a candle following a walker; click for an explosion with shake
bun run check              # tsc, eslint, bun test (real Pixi objects, real wasm)
bun run check:browser      # Chrome: fires show and animate, ticker stop freezes, shake comes back, destroy frees
```
