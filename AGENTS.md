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

## The boundary: Rust decides, the shell draws

Rust owns every decision: effects, sprites, UI, game feel, timing, which frame is current, what is drawn where, on
which layer and in what order. Each tick Rust hands a shell a list of surfaces to make, refill and show. A shell is a
thin presenter: it follows that list and decides nothing.

The contract is the `Shell` trait (today in `crates/pixfx-canvas/src/stage.rs`), driven by `Scene`:

- `make(frame)`: a new surface holding a frame;
- `refill(surface, frame)`: put a new frame in a surface;
- `show(pass, surface)`: draw a surface at `pass.rect`, colour normally, then light additively.

A shell may do only these: create and free host objects (textures, images, elements), copy pixels Rust painted into
them, place them where `Pass` says, run the host's loop and hand Rust the elapsed time, move work to workers, cache
surfaces, and turn host input into calls. Anything that changes what the player sees (placement, timing, which frame,
compositing, feel rules, palettes, option defaults) is a decision and lives in Rust.

Every shell is equal. None is the main one:

| Shell | Where | Status |
|---|---|---|
| Canvas 2D | `crates/pixfx-canvas` | shipped |
| Phaser 4 | `packages/pixfx-phaser` | shipped |
| gpui | `crates/pixfx-gpui` | shipped (own workspace, needs the gpui fork) |
| Files (PNG sheets, beanfx JSON, GIF) | `crates/pixfx-export`, `crates/pixfx-cli` | shipped |
| Pixi | `packages/pixfx-pixi` | next, for games like color-connect |
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

## Layers and where code goes

Dependencies point one way, top to bottom. A lower layer never depends on a higher one.

| Layer | Crate (target) | Holds | Rules |
|---|---|---|---|
| Shells | `pixfx-canvas`, `pixfx-phaser`, `pixfx-pixi`, `pixfx-gpui`, `pixfx-export`, `pixfx-cli` | Host objects, the host loop, workers, surface caches | No decisions. Thin enough that a second shell for the same host would be the same code. |
| JS binding | `pixfx-wasm` | `wasm-bindgen` types and option parsing for JS | Translates only. Every JS shell loads the engine through it. |
| Scene | `pixfx-scene` (today `Scene`, `Shell`, `Pass`, `Rect` in `pixfx-canvas/src/stage.rs`) | What plays, where, on which layer, which frame, the shared frame store, events | Host-neutral: no `wasm-bindgen`, no `js-sys`. gpui and every JS shell use the same one. |
| Engine | `pixfx-core` (effect model, game feel), `pixfx-effects`, `pixfx-light`, `pixfx-fighter` | Effects, particles, cues, `Feel`, `Squash`, acts, bodies | Pure and deterministic: no clock, no I/O, no host types. |
| Drawing | `pixfx-draw` (`frame`, `painter`, `shapes`, `blend`, `color`, `palette`, `outline`, `math`) | Frames with colour and light, painting, compositing | Pure. `libm` is the one required dependency. Sprites, text and UI layout join here when a game needs them. |

Where a new piece of code goes:

- A new effect: `pixfx-effects`, behind its own cargo feature (for example `fire-candle`) listed in its category
  feature (`fire`) and in `all` in `crates/pixfx-effects/Cargo.toml`, with `#[cfg(feature = "...")]` on its `mod`,
  `pub use` and `EFFECTS` entry; `crates/pixfx-effects/tests/features.rs` fails otherwise. A new feel rule or body
  behaviour: `pixfx-core` or `pixfx-fighter`.
- A new way to paint pixels (sprite, text, nine-slice, UI box): the drawing layer, driven by the first game that
  needs it; that game decides its API.
- Placement, layering, lifetimes, follow, fading after stop, which frame to show: `Scene`.
- A JS-facing type or option: `pixfx-wasm`, as a translation of a Rust type.
- A new host or output: a new shell.
- The same rule written in two shells: move it down to the lowest layer that every user of it can reach.
- Data that the engine owns (presets, effect metadata, option specs) is read from Rust at run time, never copied into
  a shell.
- Speed work on showing (caching surfaces, batching, workers, GPU upload) goes into shells. Speed work on painting and
  simulation goes into the lower layers and keeps `frame_hash` unchanged.

## Out of place today

Each item is a refactor. Each lands with every test green, the parity gate passing and `frame_hash` unchanged; a
change to a JS shell also shows `bun run perf` before and after.

1. **`Scene` and `Shell` live in `pixfx-canvas`, a wasm crate.** Move them into `pixfx-scene`. `pixfx-canvas` keeps
   only its Canvas 2D shell and JS surface, and `pixfx-gpui` drops its own layout and uses `Scene`.
2. **`packages/pixfx-phaser` makes decisions in TypeScript.**
   - `pixels.ts` (`compositeColor`, `compositeLight`, `snap`) copies the Rust compositing;
   - `layer.ts` and `layout.ts` place and composite scene-wide layers;
   - `lifecycle.ts` and `effects.ts` keep effect state (playing, stopping, destroyed), follow and per-scene ticking;
   - `runner.ts` picks the frame to show from `floor(t × fps)`;
   - `sprite-feel.ts` works out velocity from distance moved;
   - `options.ts` copies the palette presets (`PALETTE_PRESETS`);
   - `frames.ts` keeps its own frame cache (budget, what to drop, crops, sharing), which `Scene`'s frame store also
     decides.

   These move into `Scene` and the engine behind `pixfx-wasm`; the package keeps Phaser textures and images, the
   camera, wasm loading, workers and boundary parsing.
3. **No Pixi shell.** Add `packages/pixfx-pixi` once item 2 has left the Phaser package a pure shell, so the Pixi
   shell is written against the same contract and stays as thin.

## Rules

- Parity: every effect in `pixfx_effects::EFFECTS` passes the parity gate against pixelfx (`pixfx parity`).
- Determinism: the same scene, options and time give the same `frame_hash` on every target.
- Lints: clippy `unwrap_used`, `expect_used` and `panic` are denied; `unsafe_code` is forbidden.
