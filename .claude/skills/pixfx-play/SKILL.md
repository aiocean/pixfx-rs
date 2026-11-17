---
name: pixfx-play
description: Use when putting pixfx effects into a game - a Phaser 4, Canvas 2D or Pixi 8 game in TypeScript, or a Rust game on gpui - including building the wasm, linking the package, Vite setup, following a sprite, playing on a character, answering shake and hit-stop cues, rendered frames, or when play() throws or nothing shows.
---

# Play pixfx effects in a game

The engine's effects read the sprite they play on or the way it moved (`explosions/body-burst`, `feedback/outline`,
`feedback/power-up`, `projectiles/trail`). A fire, an explosion or a shield is sprite art the game plays itself.

A JS game reaches the engine through the wasm (`crates/pixfx-wasm`), the runtime `packages/pixfx-web`, and one shell
package. A Rust game uses `crates/pixfx-gpui`. Nothing is on npm or crates.io: you build and link from this repo.
Each shell's `README.md` has the full API; this is the path through it.

## JS: build, link, configure

```sh
cd pixfx-rs/packages/pixfx-web && bun install && bun run build:wasm      # wasm-pack → pixfx-web/wasm
cd ../pixfx-phaser && bun install                                         # or pixfx-canvas2d, pixfx-pixi
cd /path/to/game && bun add /abs/path/to/pixfx-rs/packages/pixfx-phaser   # absolute, or relative from the game
```

- Build only the effects the game plays (a smaller `.wasm`):
  `bun run build:wasm --no-default-features --features pixfx-effects/feedback-outline,pixfx-effects/explosions`.
  Feature = the id with `/` as `-`, or a whole category. `--no-default-features` also drops `fighter` and `light`
  (fighters, scene light): add `,fighter,light` when the game uses them, and always for **pixfx-canvas2d**, whose
  `index.ts` re-exports them (without them its build fails: `"Body" is not exported`). The wasm's `EffectId` type
  then lists only those effects, but `play` takes a `string`: an id left out throws at run time, not at compile time.
  Rebuild the wasm whenever Rust changed.
- `vite.config`: `optimizeDeps: { exclude: ["pixfx-web"] }` (it loads its wasm and worker by
  `new URL(..., import.meta.url)`), and for Phaser or Pixi `resolve: { dedupe: ["phaser"] }` / `["pixi.js"]`, or the
  bundle holds two copies.
- `await loadPixfx()` once before anything plays; `play()` before it throws. It starts the painting workers.

## The three shells

```ts
// Phaser 4: a scene plugin
import { PixfxPlugin, loadPixfx } from "pixfx-phaser";
await loadPixfx();
new Phaser.Game({ pixelArt: true, scene: [Play],
  plugins: { scene: [{ key: "pixfx", plugin: PixfxPlugin, mapping: "pixfx" }] } });
// in Play.create(): this.pixfx.play("feedback/power-up", 150, 170).wrap(this.pixfx.actorOf(hero));

// Canvas 2D: you run the loop and draw between the layers
import { createPixfx, loadPixfx } from "pixfx-canvas2d";
await loadPixfx();
const pixfx = createPixfx(ctx, { scale: 4 });
function frame(dt: number) {
  pixfx.update(dt);
  const [sx, sy] = pixfx.feel.shake; ctx.translate(sx * 4, sy * 4);
  pixfx.drawLayer("back"); drawHero(ctx); pixfx.drawLayer("front");
}

// Pixi 8: sprites in a container sorted by zIndex
import { createPixfx, loadPixfx } from "pixfx-pixi";
await loadPixfx();
const pixfx = createPixfx(app, { container: fx, camera: world });
```

Same calls on all three: `play(id, x, y, opts?)` → effect with `stop()` (loops fade, one-shots cut), `destroy()`,
`follow(target, dx?, dy?)` (a target is `{x, y, active}`: Phaser objects are; a Pixi `Sprite` needs a small
adapter object), `setPosition`, `setDepths(back, front)` (Phaser, Pixi), `wrap(...)`; events
`"complete"`, `"destroy"`, `"cue"`. Options: `seed`, `flip`, `angle`, `light`, `palette`, the effect's own (see its
`meta(id)`), plus the shell's `depth` / `scrollFactor`. An unknown id or bad option throws and creates nothing.

- Sprite between the layers: put it at a depth between the effect's back and front.
- Play on a character (`feedback/power-up` grips the sprite's outline every frame): Phaser and Pixi
  `effect.wrap(pixfx.actorOf(sprite))`; Canvas 2D `effect.wrap(imageActor(() => ({ image, cut, x, y, flipX })))`.
- A world bigger than the screen (Canvas 2D, Pixi): `pixfx.setView(x, y, w, h)` so off-screen effects cost no
  painting. Phaser reads the main camera itself.
- Many effects at once: `setDrawBudget(n)`.
- Throw at a foe at any distance (an effect tagged `volley`; none ships today): play it where it gathers
  (at the caster, `follow` keeps it there); declare every body a piece may hit with `pixfx.setBody(id, x, y, w, h)`
  (world pixels, each frame it moves; `clearBody(id)` when gone), then `effect.attack(x, y, "converge" | "pierce" | "orbit",
  casterId)` (`"orbit"`: round the point on a ring first, then in): the point gives the direction, the pieces fly on past it and burst on the first body they meet (the
  caster's spared). Each strike emits `"strike"` (`{index, x, y, body}`, `body` the id struck): the game applies
  damage there. `effect.burst()` bursts a piece where it is. `attack` returns false for an effect that takes no
  orders or was ordered already; the effect completes after its last piece.
- A trail (`projectiles/trail`): `effect.trail("projectiles/trail", opts)` lays one under any moving effect; or play
  it on its own and move it with a sprite (`setPosition` each frame): it draws the way it went, behind the
  character, and ends after `stop()`. `turn: 1` turns it with its mover's angle.

## Game feel: the game must answer it

Impact effects (`explosions/body-burst`, `feedback/power-up`) fire `shake`, `hitstop`, `ripple` and `flash` cues (the cue type also has `sound`, which no
shipped effect fires yet: load audio under the cue's `name` before playing it); `pixfx.feel` answers shake and flash on
the camera and freezes effects in a hit-stop. **The game's own world does not freeze by itself.** Multiply every
motion the game owns by `feel.timeScale` (0 in a hit-stop, a fraction in slow motion, else 1):

```ts
player.setVelocityX(speed * this.pixfx.feel.timeScale);
this.anims.globalTimeScale = this.pixfx.feel.timeScale;   // Phaser sprite animations
this.tweens.timeScale = this.pixfx.feel.timeScale;        // Phaser tweens
this.pixfx.feel.on("cue", (c: WorldCue) => {                 // WorldCue from the shell package; c is untyped otherwise
  if (c.cue.type === "sound" && this.cache.audio.exists(c.cue.name)) this.sound.play(c.cue.name);
});
```

The game's own hits: `feel.addTrauma(k)`, `feel.freeze(secs)`, `feel.slow(secs, speed)`. Player settings:
`setShakeStrength`, `setFlashStrength`. `feel: 0` on an effect silences its cues; `feel.setEnabled(false)` leaves
everything to the game. Phaser sprites squash, stretch and blink with `this.pixfx.squash(sprite)` (`land`, `blink`).

## Rendered frames (no painting at run time)

`cargo run -p pixfx-export --example render_clip -- projectiles/trail --out public/fx` (see `pixfx-render`), then in the game
`await loadClip("fx/projectiles_trail.clip.json")`. Effects with the same options show the rendered frames; others paint.

## Rust game on gpui

`crates/pixfx-gpui` is its own workspace and needs the gpui fork in game-factory's `tools/zed-gpui`, beside `pixfx-rs`. Path dependencies on
`pixfx-gpui`, `pixfx-core`, `pixfx-effects` (with only the features you play). One `FxScene` per window:

```rust
let mut fx = FxScene::default();                // or FxScene::with_threads(n)? for many effects
let power_up = pixfx_effects::effect("feedback/power-up").ok_or_else(|| anyhow!("not built"))?;
let h = fx.play(power_up, Options { x: 0.0, y: 0.0, ..Options::default() })?;
// each window frame:
fx.tick(dt)?;
fx.paint(h, Layer::Back, origin, 4, window)?;   // your sprite in between
fx.paint(h, Layer::Front, origin, 4, window)?;
fx.end_frame(window)?;                          // once, after every effect painted
```

With threads, an effect's first frame shows one window frame late. `set_actor(h, Some(actor))` each frame plays on
a character. An effect that takes orders: `fx.order(h, x, y, Aim::Converge)?` (`x`/`y` from its origin), paint each
of `fx.launched(h)` where you paint `h` (they outlive it), and read `StageEvent::Strike` from `fx.events()`.

## When it goes wrong

| Symptom | Cause |
|---|---|
| `call await loadPixfx() before play()` | `loadPixfx` not awaited first |
| wasm or worker 404 / MIME error under Vite | `optimizeDeps.exclude ["pixfx-web"]` missing |
| unknown effect id after building with features | that effect's feature not in the `build:wasm` list |
| `"Body" is not exported` (Canvas 2D) | trimmed build without `,fighter,light` |
| light shows nothing | drawn over a transparent target; additive light needs an opaque background |
| effects freeze on a hit, the hero keeps sliding | the game's motion not multiplied by `feel.timeScale` |
| package changes not picked up | `pixfx-web` is a `file:` copy: `bun install` in the shell package again |

Shell code decides nothing: a placement, timing or feel rule a game needs goes into Rust (`pixfx-scene`,
`pixfx-core`), never into the game's copy of a shell.
