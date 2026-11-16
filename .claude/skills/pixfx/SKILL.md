---
name: pixfx
description: Use when starting any work in pixfx-rs, the Rust pixel-effects engine under game-factory's games - before adding or changing an effect, putting effects in a game, rendering sheets or gifs, measuring speed, or reading pictures as pixels - and whenever unsure which crate, script or test a change belongs to.
---

# pixfx

pixfx is a pixel render engine in Rust: effects (fire, explosions, shields), game feel (shake, hit-stop, flash,
ripples, squash) and fighters, the same pixels natively, in wasm and on every shell. It ships as **source**: there
is no binary and no package to install. You work inside this repo, import the crates or JS packages, build the wasm
yourself, and run or edit scripts.

## Which skill

| You want to | Skill |
|---|---|
| Add, port or change an effect | `pixfx-effect` |
| An effect that reads the sprite it plays on (outline, glow round it, bleed from it) | `pixfx-sprite-aware` |
| A new effect from an idea: storyboard, sketch, questions, then the whole way to the library | `pixfx-new-effect` |
| Reference sheets from an image model: spec, template, draw, extract, compare | `pixfx-ref` |
| Play effects in a game (Phaser, Canvas 2D, Pixi, Rust/gpui) | `pixfx-play` |
| Make files: sprite sheet, gif, rendered clip, pose sheet, a new output | `pixfx-render` |
| Measure or improve speed | `pixfx-perf` |
| Read a picture as pixels, redraw a sprite | `pixfx-pixel` |
| Pose skeleton sheets for an image model | `pixfx-pose` |
| A character's poses, clips and directions as a mannequin sheet (hull, bones, part map, joints) | `pixfx-body` |

Two words that are never mixed up:

- **Bake** is data. A script in a crate's `examples/` (`bake_<name>.rs`) computes something once (a grown tree,
  cracks, a layout) and writes it as a Rust data file the effect reads (`<name>_<what>.rs`, "Baked by ...").
  Skill `pixfx-effect`, "Bake what nothing changes".
- **Render** or **export** is images. Sprite sheets, gifs, videos and rendered clips (`pixfx_export::render_clip`,
  `<name>.clip.json`, `loadClip`) are files made from an effect's frames. Skill `pixfx-render`.

## Where code goes

Rust decides, a shell draws. Dependencies point down this table only.

| Layer | Crates / packages | Holds |
|---|---|---|
| Shells | `packages/pixfx-canvas2d`, `pixfx-phaser`, `pixfx-pixi`, `crates/pixfx-gpui`, `crates/pixfx-export` | Host objects, the host loop, workers, files. No decisions. |
| JS runtime | `packages/pixfx-web` | wasm loading, painting workers, shared frames, a stage per host scene |
| JS binding | `crates/pixfx-wasm` | wasm-bindgen translation only |
| Scene | `crates/pixfx-scene` | what plays where, layers, which frame shows, shared frames |
| Engine | `pixfx-core`, `pixfx-effects`, `pixfx-light`, `pixfx-fighter`, `pixfx-body` | effects, particles, cues, `Feel`, bodies. Pure: no clock, no I/O |
| Drawing | `crates/pixfx-draw` | frames (colour + additive light), painter, palette, `Ruler` |
| Agent tools | `pixfx-pixel`, `pixfx-pose`, `pixfx-ref` | read pictures for an agent; never painted from |

- A new output (a file, a video, a report): logic in the library crate that owns it, plus a short script in that
  crate's `examples/` (shared helpers only in that crate's `examples/common/mod.rs`). Never a CLI or binary crate.
- The same rule in two shells moves down to the lowest layer both reach.
- Something thrown across the stage (an effect's pieces, at any distance): `pixfx_core::volley` plans it,
  `Scene::order` flies it and strikes it on the bodies the game declares; every shell gets it through `attack` and
  `setBody` / `FxScene::order` and `set_body`. Never a canvas sized to the distance.
- Veins, lightning, cracks, roots: a branching tree from `pixfx_draw::growth` (`dbm`, `colonize_from` off a `Tree::path` you set, `murray` fork angles, `radii` + `stroke` for tapered tubes; `shadow/blood_shield.rs` is the worked example)
- A way to follow (a loop before the target, a ring, a spiral; growth laid along it): `pixfx_draw::path::Path`,
  read by pixels along it. Never a curve of an effect's own.
- A streak or a trail: a brush swept along a path (`pixfx_draw::sweep`); how something moves: `pixfx_core::motion`
  read off its trace. Never a line or a velocity of an effect's own.
- Each crate's `AGENTS.md` says what it holds and what it does not; read it before editing that crate.

## One scale: the ruler

Every sprite, effect and reference sheet in a game is drawn to one ruler: its character, measured in pixels. One
pixel then means the same size on everything, so a thing drawn beside the character keeps the size it would have
beside a real one: a lens is as wide as the eye, so glasses round a 1-pixel eye are 2 pixels of lens. The ruler
is `pixfx_draw::RULER`, the duel's fighter, in the lowest layer so effects, fighters and tools all reach it;
`crates/pixfx-fighter/tests/ruler.rs` measures the painted fighter against it:

| Part | Pixels |
|---|---|
| Cell | 64 x 64, feet at (32, 60) |
| Standing, outline included | 40 rows x 20 columns (38 painted, 1 of outline above and below) |
| Head (face) | 7 x 7, about 5.4 heads to the body |
| Eye, open | 1 x 2 |
| Hand | 2 x 2 |
| Arm or leg | 3 thick |
| Blade | 16 |

- Rust paints at one game pixel per pixel and a shell scales the whole picture by a whole number, so every sprite
  and effect has the same pixel size. Never paint one thing at another pixel size (a "mixel"): no half-scale
  sprite, no 2x2 blocks standing for one pixel.
- Size a new thing against a body part, never in centimetres: name the part and the ratio (a lens as wide as the
  eye, a hat brim 1.5 heads, a spear 1.5 blades), take `Ruler::size(part, ratio)` (rounded, never under 1 pixel), and
  write the part and the ratio in a comment by the painter code. Pixel art enlarges small features (the eye is 1 of
  the face's 7 columns), and measuring against the drawn parts enlarges a new thing by the same amount.
- Anything under one pixel on the ruler is drawn at one pixel or left out, the same way everywhere.
- An effect's canvas is as big as the effect needs (a beam 256 wide, a screen 320 x 180); the things in it keep the
  ruler.
- Every effect keeps the ruler.
- Check it by eye: `scale_board` (skill `pixfx-render`) lays every effect's busiest frame on the fighter's sprite.
- The tools take the ruler: `pixfx-pose`'s default figure, `pixfx-ref`'s guide figure and the sizes its prompt
  asks for (`pixfx_ref::compare::art_ruler`), and `pixfx-pixel`'s `critique --standing` and
  `--against PART*RATIO`.
- A game with another character fills in its own `Ruler`, measured off its painted sprite, with a test like
  `tests/ruler.rs` that keeps the two equal.

## Tests: each crate tests itself

There is no repo-wide gate. Run only the tests of what you changed, as that crate's `AGENTS.md` lists under
"Before you finish":

```sh
cargo test -p <crate> --all-targets
cargo clippy -p <crate> --all-targets -- -D warnings
(cd packages/<package> && bun run check)
```

One effect builds and tests alone behind its feature (`pixfx-effect` skill). Never run `cargo test --workspace` as a
gate, and keep every test: a slow loop is fixed by running less, not by deleting tests.

## Rules the compiler and tests hold you to

- `unwrap`, `expect`, `panic` denied; `unsafe` forbidden. Handle the missing value (`let Some(x) = .. else { return }`).
- No platform float maths in engine code (`clippy.toml` bans `f64::sin`, `sqrt`, `round`...): use `libm::*` and
  `pixfx_draw::math::js_round` and friends. Randomness only from the seed.
- Determinism: same options and time give the same `frame_hash` natively and in wasm.
- Golden hashes: each effect's frames are pinned in `crates/pixfx-effects/tests/goldens/frame_hashes.json`.

## Several sessions share this tree

Other agents often edit pixfx-rs at the same time. Leave their dirty files alone, and commit only your own changes
(through a temporary index), never with a plain `git commit -a`. A half-written crate of theirs can make every cargo
command fail with "failed to load manifest for workspace member": wait for it, or check your work in a copy of the
tree without that crate.
