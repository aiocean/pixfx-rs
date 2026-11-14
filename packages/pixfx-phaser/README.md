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

That builds every effect into the `.wasm`. A game that plays a few builds only those, and pays for no other:

```sh
bun run build:wasm --no-default-features --features pixfx-effects/fire-candle,pixfx-effects/explosions
```

Name an effect by its id with the `/` as `-` (`fire/candle` is `pixfx-effects/fire-candle`), or a whole category
(`pixfx-effects/explosions`). `EffectId` then lists only those effects, so TypeScript flags a `play` of one left out.

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

`loadPixfx()` starts worker threads that paint the effects, one per core the main thread leaves free (1 to 4),
each with its own copy of the engine. The main thread only decides (clocks, placement, which frame shows) and
uploads; a worker simulates, paints, crops and turns a frame into RGBA, and transfers the bytes. The pixels are
exactly those the engine paints on the main thread. A new frame shows once its worker sends it back; until then the
effect shows its last one, and each effect has at most one frame out at a time, so a slow phone drops frames
instead of falling behind. Each effect stays in one worker for its life. `play()` validates the id and options on
the main thread and throws at once; `"complete"` and cues come from the main thread's clock.

```ts
await loadPixfx({ workers: 2 }); // a fixed number of workers
await loadPixfx({ workers: 0 }); // paint on the main thread
await loadPixfx({ frameBudget: 32 * 1024 * 1024 }); // bytes of shared frames held (default 64 MiB)
```

Without `Worker` support effects paint on the main thread. Options apply to the first call; later calls return
the loaded engine. The worker is bundled by Vite from `new Worker(new URL(...), { type: "module" })`, with no
plugin.

### How frames are shown

An effect plays its rendered frames: what it shows depends only on the effect, its options and the frame its
clock is on. Effects with the same id and options (seed, flip, angle, light, palette, outline, their own
options) show the same frames, so the game paints each frame once and every such effect, in every scene, shows it.
The engine (`pixfx-scene`, through `pixfx-wasm`) decides all of this; the plugin follows its draw list:

- Each frame is cropped to the lit box of its colour and light, and held in one surface, uploaded once. Nothing is
  painted or uploaded while the frame an effect shows stays the same.
- On WebGL every surface is a region of a few 4096² atlas pages, so the images go out in few batches; on Phaser's
  Canvas renderer each surface is a canvas texture.
- The held frames are game-wide (they outlive a scene) and stay under `frameBudget` bytes: the least recently shown
  go first, never one shown now. A frame dropped is painted again, byte for byte, by the next effect that reaches it.
- A stopped effect's fade-out is its own frames, painted only when its clock reaches a new one.

`stats()` returns `{engines, textures, images}`: effects playing in the scene, the game's surface regions, and the
scene's images.

### Baked frames

`pixfx bake <id> [--palette WORD] [--outline WORD] --out public/fx` writes `<name>.bake.json` and its PNG pages:
every frame of the effect as the game would paint it, cropped. Load it once a scene with the plugin has started:

```ts
await loadPixfx();
// in a scene's create():
await loadBaked("fx/fire_campfire.bake.json"); // pages are fetched from the same folder
```

Effects playing that effect with the same options then show the baked frames and paint nothing; other options
paint live. Baked frames are held like painted ones, under `frameBudget`: past it they go and are painted again if
shown.

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
| `outline` | Outline round the effect: `true`, `false`, `"white"`, `"#ffcc00"`, `{color, width, corners}` (see the repo README's [Outlines](../../README.md#outlines)). Left out, the effect looks as designed. |

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

## Game feel

Effects fire cues on the frame they mean them: `shake`, `hitstop`, `ripple`, `flash` and `sound`. The impact
effects (explosions, dirt, plasma, implosion, parry, icicle fall, poof, feather burst) fire them by how heavy the
hit is. Their `feel` option scales the cues (0 to 2, default 1), and `feel: 0` silences them.

`scene.pixfx.feel` (a `PixfxFeel`) answers the cues for the scene:

- a hit-stop freezes every effect in the scene, and `feel.timeScale` (0 in a hit-stop, a fraction in slow
  motion, else 1) lets the game freeze its own world with them;
- the shake moves and tilts the main camera's view for the render only, so its scroll, follow and rotation are
  untouched. Each hit adds 0.3 (light) to 0.6 (heavy) trauma, which drains 1 a second; the camera moves up to
  16 game pixels and tilts up to 0.05 radians times trauma squared, along smooth Perlin noise;
- the flash lifts the view toward white.

```ts
this.pixfx.feel.on("cue", (c: WorldCue) => {
  if (c.cue.type === "sound") this.sound.play(c.cue.name);
});
player.setVelocityX(speed * this.pixfx.feel.timeScale);
```

| Call | Effect |
|---|---|
| `addTrauma(k)` | The game's own impact shakes the camera (`k` 0 to 1 added to the trauma). |
| `freeze(secs)` | The game's own hit-stop. Freezes do not add up: the longest wins. |
| `slow(secs, speed)` | Slow motion at `speed` (0 to 1) for `secs`, after any freeze. |
| `setShakeStrength(k)` / `setFlashStrength(k)` | Player settings: 1 as designed, 0 off (still camera, no flash). |
| `setEnabled(false)` | Leave the answer to the game. Effects play at scene time and `"cue"` events still fire. |
| `shake`, `roll`, `flash`, `trauma`, `frozen`, `timeScale` | This frame's answer, read-only. |

`"cue"` fires on the feel and on the effect (`effect.on("cue", ...)`) with `{t, pan, cue}`. A ripple's `x`/`y`
and a sound's `x` are world positions. Ripples are data only; the plugin does not draw them. Cues fire
on the frame their clock reaches, with or without worker threads.

### Squash, stretch and blink for sprites

```ts
const hero = this.physics.add.sprite(100, 200, "hero").setOrigin(0.5, 1); // pivot at the feet
const look = this.pixfx.squash(hero);
// on landing: look.land(Math.min(1, fallSpeed / 400));
// on taking a hit: look.blink(1.2);
```

Each scene update it stretches the sprite along its velocity when it moves fast, using the arcade body's velocity
when there is one and otherwise how far the sprite moved. It reaches 1.3 times its length at 220 px/s, so it is
tall for a jump or fall and wide for a dash. A Phaser sprite cannot skew, so the stretch stays upright, and a
flight at exactly 45° has none. On `land(strength)` it flattens the sprite to 0.7 tall and eases back within
0.15 s. The other axis takes the inverse, so the area stays the same. It blinks the sprite for `blink(secs)`. It scales over the sprite's scale and alpha at creation; call
`setBase(scaleX, scaleY, alpha)` when the game changes them. In a hit-stop it holds its shape. The collision body
keeps its size. It ends when the sprite is destroyed, on `destroy()` (the sprite returns to its base), or when the
scene shuts down.

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
