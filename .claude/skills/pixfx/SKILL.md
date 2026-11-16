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
| Make files: sprite sheet, gif, baked clip, pose sheet, a new output | `pixfx-render` |
| Measure or improve speed | `pixfx-perf` |
| Read a picture as pixels, redraw a sprite | `pixfx-pixel` |
| Pose skeleton sheets for an image model | `pixfx-pose` |

## Where code goes

Rust decides, a shell draws. Dependencies point down this table only.

| Layer | Crates / packages | Holds |
|---|---|---|
| Shells | `packages/pixfx-canvas2d`, `pixfx-phaser`, `pixfx-pixi`, `crates/pixfx-gpui`, `crates/pixfx-export` | Host objects, the host loop, workers, files. No decisions. |
| JS runtime | `packages/pixfx-web` | wasm loading, painting workers, shared frames, a stage per host scene |
| JS binding | `crates/pixfx-wasm` | wasm-bindgen translation only |
| Scene | `crates/pixfx-scene` | what plays where, layers, which frame shows, shared frames |
| Engine | `pixfx-core`, `pixfx-effects`, `pixfx-light`, `pixfx-fighter` | effects, particles, cues, `Feel`, bodies. Pure: no clock, no I/O |
| Drawing | `crates/pixfx-draw` | frames (colour + additive light), painter, palette |
| Agent tools | `pixfx-pixel`, `pixfx-pose` | read pictures for an agent; never painted from |

- A new output (a file, a video, a report): logic in the library crate that owns it, plus a short script in that
  crate's `examples/` (shared helpers only in that crate's `examples/common/mod.rs`). Never a CLI or binary crate.
- The same rule in two shells moves down to the lowest layer both reach.
- Something thrown across the stage (an effect's pieces, at any distance): `pixfx_core::volley` plans it,
  `Scene::order` flies it and strikes it on the bodies the game declares; every shell gets it through `attack` and
  `setBody` / `FxScene::order` and `set_body`. Never a canvas sized to the distance.
- Veins, lightning, cracks, roots: a branching tree from `pixfx_draw::growth` (`dbm`, `colonize_from` off a `Tree::path` you set, `murray` fork angles, `radii` + `stroke` for tapered tubes; `shadow/blood_shield.rs` is the worked example)
- A streak or a trail: a brush swept along a path (`pixfx_draw::sweep`); how something moves: `pixfx_core::motion`
  read off its trace. Never a line or a velocity of an effect's own.
- Each crate's `AGENTS.md` says what it holds and what it does not; read it before editing that crate.

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
- Parity: every effect matches its source (pixelfx, or a vendored Sprite Fusion file) within 0.5% of pixels.

## Several sessions share this tree

Other agents often edit pixfx-rs at the same time. Leave their dirty files alone, and commit only your own changes
(through a temporary index), never with a plain `git commit -a`. A half-written crate of theirs can make every cargo
command fail with "failed to load manifest for workspace member": wait for it, or check your work in a copy of the
tree without that crate.
