# pixfx-canvas2d

Play pixfx effects on a Canvas 2D context, among your own drawing. Effects run in the pixfx engine (Rust compiled to
WebAssembly) and are painted on worker threads. Each effect draws in two layers: its back layer goes under what you
draw between the two calls, its front layer over it. Colour is drawn normally and light is added (`"lighter"`).

## Install

The package is not on npm yet. Build the engine once, then link the package into your game:

```sh
cd pixfx-rs/packages/pixfx-web && bun install && bun run build:wasm   # wasm-pack → pixfx-web/wasm
cd ../pixfx-canvas2d && bun install
cd /path/to/your-game && bun add ../pixfx-rs/packages/pixfx-canvas2d
```

pixfx-web loads its wasm and its worker through `new URL(..., import.meta.url)`. With Vite, serve it as source:
`optimizeDeps: { exclude: ["pixfx-web"] }`. A page without a bundler loads `dist/pixfx-canvas2d.js` from
`bun run build`; the worker and the wasm sit beside it in `dist/assets/`.

## Use

```ts
import { createPixfx, loadPixfx } from "pixfx-canvas2d";

await loadPixfx(); // once, before createPixfx()
const ctx = canvas.getContext("2d");
if (!ctx) throw new Error("no 2d context");
const pixfx = createPixfx(ctx, { scale: 4 }); // one game pixel is 4×4 canvas pixels

pixfx.play("fire/campfire", 168, 160);
const hero = { x: 100, y: 142, active: true };
pixfx.play("fire/candle", 0, 0).follow(hero);

function frame(dt: number) {
  pixfx.update(dt);                 // effects play; frames still to paint go to the workers
  drawBackground(ctx);
  const [sx, sy] = pixfx.feel.shake; // the view's shake, in game pixels
  ctx.translate(sx * 4, sy * 4);
  pixfx.drawLayer("back");
  drawHero(ctx);
  pixfx.drawLayer("front");
}
```

`createPixfx(ctx, { scale? })`: `scale` is canvas pixels per game pixel, a whole number (default 1). Positions are
in game pixels, drawn in the context's current transform.

`pixfx.play(id, x, y, opts?)` places the effect's origin at `(x, y)` and returns a `PixfxEffect` (`stop()`,
`destroy()`, `follow(target, dx?, dy?)`, `setPosition(x, y)`; events `"complete"`, `"destroy"`, `"cue"`). Every option
is the engine's (`seed`, `flip`, `angle`, `light`, `palette`, `outline`, the effect's own); an unknown id or a bad
option throws and creates nothing. `drawLayer` draws the default depths (back 0, front 1); `draw()` draws every
depth, lowest first.

`pixfx.warp()`, after drawing everything a heavy blow's ring should bend, bends the canvas by the live ripples.
`pixfx.drawPicture(picture)` draws a picture Rust painted outside the effects: fighters (`new Sprites(w, h)`, then
`picture()`) or scene light (`new Lighting(w, h).paint(lights, glows)`). The package re-exports the engine's
`effects`, `meta`, `Fx`, `Body`, `FightFeel`, `lookColors`, `Sprites`, `Lighting`, `stepLights` and `putOutLight`,
usable once `loadPixfx()` resolved.

You run the loop. `update(dt)` returns the seconds the effects played: 0 in a hit-stop, less in slow motion.
`pixfx.feel` answers impacts. It has `shake`, `roll`, `flash`, `timeScale`, `addTrauma`, `freeze` and `slow`, and
emits `"cue"`; `feel.setEnabled(false)` leaves the answer to your game. `pixfx.stats()` counts effects, textures and
images. `pixfx.setDrawBudget(n)` draws at most `n` effect layers per frame (the rest keep their last frame and wait
their turn); 0, the default, draws every one that changed. `pixfx.destroy()` ends every effect.

## Demo and checks

```sh
bunx vite demo             # campfire, braziers, a candle following a walker; click for an explosion
bun run check              # tsc, eslint, bun test (real wasm)
bun run check:browser      # Chrome: fires show and animate, a paused loop holds them, shake, destroy frees
```
