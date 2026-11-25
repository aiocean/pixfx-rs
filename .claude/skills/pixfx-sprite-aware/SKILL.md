---
name: pixfx-sprite-aware
description: Write a pixfx effect that knows the sprite it plays on (a character, a sword, a hat) instead of a layer pasted over it, with pixfx_core::Silhouette and Actor. Use when an effect should hug a sprite's outline, ring it, glow round it, light its edge, tint its body, take its colours, spawn particles on it or bleed from it ("the aura should follow the character", "outline the selected unit", "sparks off the sword's edge", "make the hat glow"), or when an effect looks like a lifeless overlay.
---

# pixfx-sprite-aware

In `tools/pixfx-rs`. An effect that **wraps its actor** is handed the sprite's current frame each time it changes,
reads it into cells of its own space, and draws from that: on the body, along its edge, out from it. The same effect
follows every frame of a walk cycle, and works on any sprite the game hands it.

Effects that read an actor or trace can adapt to moving sprites. Working examples to copy from:

| Effect | File | Reads |
|---|---|---|
| `feedback/outline` | `crates/pixfx-effects/src/feedback/outline.rs` | `ring`: an outline round the sprite. The smallest one: start here. |
| `feedback/power-up` | `crates/pixfx-effects/src/feedback/power_up.rs` | `around` (aura by distance), `pick` (motes on the body), `cells` and `Actor::opaque` (the body flashing in its own brightness), a stand-in body when there is no sprite. |
| `explosions/body-burst` | `crates/pixfx-effects/src/explosions/body_burst.rs` | `Actor::opaque`: the sprite's own pixels broken into chunks that fly and settle; the game removes the sprite. |
| `projectiles/trail` | `crates/pixfx-effects/src/projectiles/trail.rs` | its motion (`Trace`), and on a character its back edge (`Brush::edge`). |

## The API (`pixfx_core`)

| Call | Gives | For |
|---|---|---|
| `Silhouette::of(&actor, area)` | The cells of `area` (an `Area {x, y, w, h}` of effect pixels) under the sprite's opaque pixels | Every query below |
| `Silhouette::from_fn(area, \|x, y\| ...)` | A body from shapes, tested at cell centres | A stand-in when the game gave no sprite |
| `is_empty()`, `cells()`, `covers(x, y)` | The body | Drawing on it |
| `bounds()`, `centroid()` | Its box, its middle | Placing a glow, a label, a burst |
| `edge()` | Covered cells beside an uncovered one | Rim light, sparks or drips off the body |
| `ring(width, corners)` | Cells of an outline `width` px wide, square (`corners`) or rounded | Outlines, selection rings |
| `around(max)` | Cells off the body within `max` px, with their distance | Auras, halos, glows fading out |
| `distance(x, y)` | One cell's distance from the body | Shading by nearness |
| `pick(&mut rng, tries)` | A random covered cell, the same from the same seed | Spawning particles on the body |
| `Actor::opaque()` | Each opaque sprite pixel: its cell and `Rgba` colour | Tinting the body, taking its colours, bursting from its pixels |
| `actor.x()`, `y()`, `w()`, `h()` | The sprite's box in the effect's space | Sizing the `Area` |

The effect's space has its origin where the game plays it (a character's feet, a sword's hilt). The sprite arrives
already placed there, scaled and flipped as the game shows it; the scene mirrors it when the effect is flipped.

## Writing one

1. **The file.** `crates/pixfx-effects/src/<category>/<name>.rs`, `mod` and `pub use` in the category's `mod.rs`,
   the feature `<category>-<name> = []` in `crates/pixfx-effects/Cargo.toml` (listed in its category feature), and
   its `EFFECTS` entry in `src/lib.rs`, each under `#[cfg(feature = "<category>-<name>")]`.
2. **Say it wraps its actor.** `fn wraps_actor(&self) -> bool { true }` on the `Effect`. Its frames become its
   instance's own (drawn again when the sprite changes), never shared with another instance.
3. **Read the sprite once per sprite frame.** In `EffectState::set_actor`, build the `Silhouette` (or what you draw
   from it, as outline keeps its ring cells) and store it in the state. `set_actor(None)` means no sprite: draw a
   stand-in (`from_fn`) or nothing. Never read the sprite in `draw`: `draw` runs every frame, `set_actor` only when
   the sprite changes.
4. **Size the area.** The sprite's box plus the room the effect draws in round it (`width` for a ring, the reach
   for an aura). The canvas (`Effect::canvas`) must hold that area round the origin: a fixed size, or `w`/`h`
   options as outline has.
5. **Draw from it** in `draw`: `p.px(x, y, colour)` per cell, on `Layer::Back` for what goes behind the sprite (a
   ring, an aura) and `Layer::Front` for what goes over it (a tint, sparks in front).
6. **Keep the sprite's pixel size.** What the effect adds (an outline, a mote, a spark, a drip) is in the sprite's
   own pixels: one effect pixel is one sprite pixel, so a 1-pixel outline stays 1 pixel and a mote is the size of a
   body part it is measured against (skill `pixfx`, "One scale: the ruler"; a mote no bigger than the eye). Never
   draw blocks standing for one pixel. Sizes that follow the sprite (an aura's reach) come from the silhouette, not
   from a fixed number for one character.
7. **Animate with time, not by re-reading.** Keep the cells; vary colour, reach or alpha with `t` (see power-up's
   aura reach and noise).

Determinism holds as long as randomness comes from the effect's seeded `Rng` (`pick` takes it) and nothing iterates a
`HashMap`.

## How a game hands it the sprite

```ts
const fx = scene.pixfx.play("feedback/outline", hero.x, hero.y, { width: 2, palette: ["#ffffff"] });
fx.follow(hero).wrap(scene.pixfx.actorOf(hero));      // Phaser; Pixi: pixfx.actorOf(sprite)
// Canvas 2D, for a sprite the game draws itself:
fx.wrap(imageActor(() => ({ image: sheet, cut: { x, y, w, h }, x: left, y: top })));
```

gpui: `FxScene::set_actor(fx, Some(actor))` with an `Actor` placed in effect pixels.

## Checking it

- Look at it on a real sprite first, as a gif: `cargo run -p pixfx-export --example gif -- <id> --actor a.png,b.png
  --actor-every 6` plays it on those sprite frames (feet, the middle of each PNG's bottom row, on the origin) and
  writes `target/out/<name>.gif`. The gif holds what the effect draws, not the sprite.
- Then in a game: play it in a shell's demo (`packages/pixfx-canvas2d/demo/main.ts` plays power-up on a hero whose
  pose changes) and watch it follow each frame.
- Its golden hashes (`crates/pixfx-effects/tests/goldens/frame_hashes.json`) pin its frames once you bless them.
