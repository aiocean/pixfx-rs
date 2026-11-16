# pixfx-rs

Read `README.md` for the crates, commands and hosts. This file says where the project is going, where each kind of
code belongs, and what is still out of place. The full design is
`docs/superpowers/specs/2026-11-14-render-engine-vision-design.md`.

## Vision

pixfx is the Rust pixel render engine under every game in game-factory. It draws sprites, UI and effects with one look
(colour and additive light, outline, palette), and every platform shows the same pixels. It is to our games what gpui
is to Zed: the games are the product, and the engine grows only what a real game needs.

- **Strong.** Effects look hand-made and land with weight: light, cues for shake, hit-stop, ripple, flash and sound,
  and bodies that react to a hit. Sprites and UI share the same look.
- **Convenient.** A game draws with a few calls and plain-data options, from TypeScript or from Rust.
- **Fast.** Many sprites and effects at once on a mid-range phone at 60 fps. Play speed comes before download size: a
  download happens once, every frame pays for slow code, so release builds keep `opt-level = 3` with fat LTO
  (`Cargo.toml` `[profile.release]`), and a game cuts size by building only the effects it plays. "Fast" is a
  measurement: a change that claims it shows `bun run perf` (`packages/pixfx-phaser/perf/`) or a gpui measurement
  before and after.
- **Everywhere, identical.** Written once in Rust; the same frames natively, in WebAssembly and on every shell.
  `frame_hash` and the parity gate prove it.
- **Source, not a package.** A user clones this repo and works in it with an agent. The agent imports the crates and
  the JS packages, builds the wasm itself (`bun run build:wasm`), and runs or edits the scripts in each crate's
  `examples/` to make files. There is no command-line tool: an output a game needs is a script over the library,
  and its logic lives in the library so every script and game can call it.

## The boundary: Rust decides, the shell draws

Rust owns every decision: effects, sprites, UI, game feel, timing, which frame is current, what is drawn where, on
which layer and in what order. Each tick Rust hands a shell a list of surfaces to make, refill and show. A shell is a
thin presenter: it follows that list and decides nothing.

The contract is the `Shell` trait in `crates/pixfx-scene`, driven by `Scene`:

- `make(frame)`: a new surface holding a frame;
- `refill(surface, frame)`: put a new frame in a surface;
- `show(pass, surface)`: draw a surface at `pass.rect`, colour normally, then light additively.
- `free(surface)`: let go of a surface the scene dropped (past the frame budget, or its effect ended).

`Scene` decides on the thread that runs it and keeps only each effect's clock. Painting a frame is a `Painter`'s job,
on the spot or on another thread, and the frames are held in a game-wide `Frames` that every `Scene` of the game
shares (two canvases, or two Phaser scenes, paint a frame once).

A shell may do only these: create and free host objects (textures, images, elements), copy pixels Rust painted into
them, place them where `Pass` says, run the host's loop and hand Rust the elapsed time, move work to workers, cache
surfaces, and turn host input into calls. Anything that changes what the player sees (placement, timing, which frame,
compositing, feel rules, palettes, option defaults) is a decision and lives in Rust.

Every shell is equal. None is the main one:

| Shell | Where | Status |
|---|---|---|
| Canvas 2D | `packages/pixfx-canvas2d` | shipped |
| Phaser 4 | `packages/pixfx-phaser` | shipped |
| gpui | `crates/pixfx-gpui` | shipped (own workspace, needs the gpui fork) |
| Files (PNG sheets, beanfx JSON, GIF, baked clips) | `crates/pixfx-export` and its `examples/` scripts | shipped |
| Pixi 8 | `packages/pixfx-pixi` | shipped |
| CSS and DOM | `packages/pixfx-dom` | possible |
| WebGL | a shell of its own | possible |

A new shell is a new crate or package that implements the contract. It needs no change in the engine; if it does,
the engine was missing something every shell needs.

Same pixels on every shell: Rust paints every surface's pixels, so the content is identical everywhere. A shell keeps
the composition identical by honouring whole-pixel placement, nearest-neighbour scaling and additive light (CSS:
`image-rendering: pixelated` and `mix-blend-mode: plus-lighter`).

Games are made two ways, on the same engine:

| | JS game | gpui game |
|---|---|---|
| Example | a Capacitor game like terminal-tails | a native mobile app in Rust |
| Game code | TypeScript | Rust |
| Reaches the engine through | `pixfx-wasm` and a JS shell (Canvas, Phaser, Pixi) | `pixfx-gpui` |
| Platforms | web, and iOS and Android through Capacitor | what the gpui fork (`tools/zed-gpui`) supports: iOS and web, no Android yet |

A new engine feature ships for both kinds of game.

## How a frame is made, and where each cost goes

Three parts, each on the thread that suits it:

| Part | Holds | Runs |
|---|---|---|
| Decide (`Scene`) | clocks, placement, which frame shows, cues, events | the host's main thread; never touches pixels |
| Paint (`Painter` into a `Frame`) | simulating an effect up to a frame, drawing it, cropping it, turning it into RGBA | workers for every JS shell (Canvas, Phaser, Pixi), native threads for gpui (`FxScene::with_threads`) when a window has many effects, on the spot for files, tests and hosts without workers |
| Show (`Shell`) | uploading pixels into host objects and placing them | the host's main thread |

The main thread has 16 ms a frame and runs the game too; simulation and painting are the cost that grows with
the number of effects, so they leave it:

- A frame identical effects share is painted once for the whole game (`Frames` is shared by every `Scene`), and an
  effect's state advances only when a frame of it is painted, so effects off screen cost nothing.
- A worker does all the pixel work and transfers the buffer. Pixels reach the main thread once, to go to the GPU:
  they never go back through wasm there; Rust gets only what it decides with (size, crop, which halves hold pixels).
- WebGL shells (Phaser, Pixi) show frames from one texture atlas, so every effect of a layer goes out in one batch.
  The engine has no layer groups: merging a region again whenever one effect in it changes costs more than the atlas.
- Canvas 2D fills a small canvas per frame with the bytes a worker transferred (`putImageData`) and draws from it; it
  needs no atlas. Handing it `ImageBitmap`s a worker made would save only that copy: 28 ms/s on the landing duel at
  6× CPU throttling.
- A new output (a terminal in ASCII, an OLED screen, the DOM) is a new shell; deciding and painting stay as they are.
  Embedded targets would also need `no_std` crates, room for frames, and a way round `f64` maths, which parity keeps.

## Layers and where code goes

Dependencies point one way, top to bottom. A lower layer never depends on a higher one.

| Layer | Crate (target) | Holds | Rules |
|---|---|---|---|
| Shells | `pixfx-canvas2d`, `pixfx-phaser`, `pixfx-pixi`, `pixfx-gpui`, `pixfx-export` | Host objects, the host loop, workers, surface caches | No decisions. Thin enough that a second shell for the same host would be the same code. |
| JS runtime | `packages/pixfx-web` (TypeScript) | Loading the wasm, painting workers, the game-wide frames and their surfaces, a stage per host scene following the draw list, effect handles, the feel's JS face | Host-neutral: no Phaser, no Pixi. A JS shell adds only its host objects and its loop. |
| JS binding | `pixfx-wasm` | `wasm-bindgen` types and option parsing for JS | Translates only. Every JS shell loads the engine through it, by way of `pixfx-web`. |
| Game | `pixfx-duel` | The landing duel's rules: fighters, skills, the rotating bar, statuses, shots, rounds, the seats that decide for each corner (the brain from `pixfx-mind`, or the first mind), and the view the page reads | Deterministic from a seed: no clock, no I/O, no host types. Every decision of the duel is here; the page plays the commands and events it returns. |
| Scene | `pixfx-scene` (`Scene`, `Frames`, `Painter`, `Shell`, `Pass`, `Rect`, `StageEvent`) | What plays, where, on which layer, which frame, the game-wide shared frames and who paints them, events | Host-neutral: no `wasm-bindgen`, no `js-sys`. gpui and every JS shell use the same one. |
| Engine | `pixfx-core` (effect model, game feel), `pixfx-effects`, `pixfx-light`, `pixfx-fighter`, `pixfx-body` | Effects, particles, cues, `Feel`, `Squash`, acts, bodies; the rig every pose, direction and clip is made with, and its mannequin | Pure and deterministic: no clock, no I/O, no host types. |
| Drawing | `pixfx-draw` (`frame`, `painter`, `shapes`, `halo`, `blend`, `color`, `palette`, `math`) | Frames with colour and light, painting, painted glow, compositing | Pure. `libm` is the one required dependency. Sprites, text and UI layout join here when a game needs them. |

Where a new piece of code goes:

- A new effect: `pixfx-effects`, behind its own cargo feature (for example `fire-candle`) listed in its category
  feature (`fire`) and in `all` in `crates/pixfx-effects/Cargo.toml`, with `#[cfg(feature = "...")]` on its `mod`,
  `pub use` and `EFFECTS` entry; `crates/pixfx-effects/tests/features.rs` fails otherwise. A new feel rule or body
  behaviour: `pixfx-core` or `pixfx-fighter`.
- A pose, a camera, an animation clip or the mannequin painted from them (hull, bones, part map): `pixfx-body`.
- A new way to paint pixels (sprite, text, nine-slice, UI box): the drawing layer, driven by the first game that
  needs it; that game decides its API.
- A tool an agent uses to read a picture as pixels (an image model's drawing, a customer's sprite): `pixfx-pixel`,
  a pure function per step in `pixfx_pixel::tool`, run by a script per step in its `examples/`. It feeds the agent, never the game: the agent writes painter code from what it
  reads, and nothing in the engine paints from its output.
- Getting reference art for a new effect from an image model (template sheet, prompt, the call, the effect's pixels
  cut back out as colour and light): `pixfx-ref`, a script per step in its `examples/`. Like `pixfx-pixel` it feeds the
  agent: the effect is written by hand in `pixfx-effects` from what it shows.
- Placement, layering, lifetimes, follow, fading after stop, which frame to show: `Scene`.
- A JS-facing type or option: `pixfx-wasm`, as a translation of a Rust type.
- A new host or output: a new shell.
- A new file to make (a sheet, a video, a report): its logic in the library crate that owns it, and a short script
  in that crate's `examples/` that reads arguments and writes files. Scripts share nothing but an
  `examples/common/mod.rs` of their own crate.
- The same rule written in two shells: move it down to the lowest layer that every user of it can reach.
- Data that the engine owns (presets, effect metadata, option specs) is read from Rust at run time, never copied into
  a shell.
- Speed work on showing (caching surfaces, batching, workers, GPU upload) goes into shells. Speed work on painting and
  simulation goes into the lower layers and keeps `frame_hash` unchanged.

## Out of place today

Each item is a refactor. Each lands with every test green, the parity gate passing and `frame_hash` unchanged; a
change to a JS shell also shows `bun run perf` before and after.

Nothing is out of place now. Code found on the wrong side of a boundary is listed here, one numbered item each,
until its refactor lands.

## Rules

- Parity: every port in `pixfx_effects::EFFECTS` passes the parity gate against its source (`cargo run -p pixfx-export --example parity -- <id>`): pixelfx,
  or the Sprite Fusion file vendored at `tools/spritefusion/<id>.ts` for an effect pixelfx does not have. An effect
  made here (`feedback/outline`) has no source; its golden hashes pin it.
- Determinism: the same scene, options and time give the same `frame_hash` on every target.
- Lints: clippy `unwrap_used`, `expect_used` and `panic` are denied; `unsafe_code` is forbidden.
- Tests live in the crate or package they test, and there is no repo-wide gate: a change runs the tests of what it
  changed (each crate's and package's "Before you finish"), and a speed claim shows that crate's own measurement
  before and after (`bench` for effects, `bun run perf` for Phaser, the gpui measurements). One effect builds,
  tests and benches alone behind its feature (`crates/pixfx-effects/AGENTS.md`). Keep every test; a slow loop is
  fixed by running less of it, not by deleting it.