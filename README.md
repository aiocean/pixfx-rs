# pixfx-rs

Pixel-art effects in Rust. An effect is written once and produces the same pixels natively and in WebAssembly.

It has two modes:

- **Render.** Run an effect natively and write sprite sheets (composite, colour and light PNGs), a
  beanfx-layout JSON, or a GIF.
- **Play live.** Run the same effect in a browser, through plain JS bindings, a self-driving Canvas 2D
  renderer, or inside gpui.

Effects are ports of [pixelfx](https://github.com/patch-gg/pixelfx) (JS), or of
[Sprite Fusion's pixel art VFX](https://www.spritefusion.com/pixel-art-vfx) (TypeScript) where pixelfx has no such
effect, and use the [beanfx](NOTICE.md) sheet layout. Every effect in `pixfx_effects::EFFECTS` is a port that passes
the parity gate against its source.

## How you use it

pixfx ships as source. There is no binary to install and no package to download: you clone this repo and work in it
with your agent, which reads the code, imports it and runs it.

The agent's guides are the skills in `.claude/skills/`: `pixfx` (start here: which skill, which crate, which
test), `pixfx-effect`, `pixfx-play`, `pixfx-render`, `pixfx-perf`, `pixfx-pixel` and `pixfx-ref`.

- **A Rust game** (gpui, a native tool) depends on the crates by path and calls them.
- **A JS game** builds the wasm itself (`bun run build:wasm` in `packages/pixfx-web`) and plays through a JS shell:
  `pixfx-canvas2d`, `pixfx-phaser` or `pixfx-pixi`.
- **Files** (sprite sheets, gifs, rendered clips, pose sheets) come from the scripts below. Each is a cargo example,
  a short `main` over the library: run it as it is, or change it or copy it when its flags do not cover what you
  need.

| Script | Makes |
|---|---|
| `cargo run -p pixfx-export --example sprite_sheet -- <id>` | Composite, colour and light PNGs and the beanfx-layout JSON |
| `cargo run -p pixfx-export --example gif -- <id>` | A looping gif |
| `cargo run -p pixfx-export --example mp4 -- <id>` | A video (needs ffmpeg) |
| `cargo run -p pixfx-export --example render_clip -- <id>` | A rendered clip hosts play without drawing (`loadClip`) |
| `cargo run -p pixfx-export --example parity -- <id>` | The parity gate for one effect |
| `cargo run --release -p pixfx-effects --example bench` | What a frame of effects costs natively |
| `cargo run -p pixfx-fighter --example poses -- <look>` | Every pose of a fighter look on one sheet |
| `cargo run -p pixfx-body --example clip_sheet -- <clip> [--camera side\|iso\|topdown]` | A catalog clip (walk, run, crouch...) as a mannequin sheet, a row per direction: composite, hull, bones and part map PNGs, a GIF and the frames' joints as JSON (skill `.claude/skills/pixfx-body/SKILL.md`); `--example catalog` writes every clip |
| `cargo run -p pixfx-pixel --example <step> -- ...` | The agent's pixel steps: `clean`, `read`, `draw`, `view`, `critique`, `sheet`, `cut`, `parts`, `diff` (skill `.claude/skills/pixfx-pixel/SKILL.md`) |
| `cargo run -p pixfx-ref --example <step> -- ...` | A new effect's reference from an image model: `template` (sheet and prompt from a spec), `generate` (the OpenRouter call), `extract` (colour and light layers per frame, a review page), `gallery` (skill `.claude/skills/pixfx-ref/SKILL.md`) |

Each script's first lines say what it takes and writes.

## Crates

| Crate | What it is |
|---|---|
| `crates/pixfx-draw` | Drawing, the lowest layer: frame with colour and additive light, painter, shapes, blending, `composite`, `frame_hash`, palette presets, the JS-number maths (`math`). Knows nothing of effects or hosts. |
| `crates/pixfx-core` | The engine on top of `pixfx-draw`: `Effect`/`Instance`, particles, `render_frames`, `Player`, the frame key and store; game feel: `Feel` (shake, hit-stop, slow motion, flash, ripples), `cue::impact`, `Squash`/`Blink`; sprite awareness: `Actor`, `Silhouette` (skill `.claude/skills/pixfx-sprite-aware/SKILL.md`). |
| `crates/pixfx-scene` | What a stage shows and where, for every shell: `Scene` (effects playing, placement, layers, the view outside which nothing is drawn, the answer to their cues, events), `Frames` (the frames every scene of a game shares, and the `Painter` that paints them, on the spot or on another thread) and the `Shell` contract (`make`, `refill`, `show`, `free`). No browser or gpui types. |
| `crates/pixfx-body` | The rig: a figure on the ruler (`Body`) posed by joint angles (`Pose`), seen by any camera (`Camera::side`, `iso`, `topdown`, 4 or 8 directions), animated by clips with easing, root motion and planted feet (`Clip`, `catalog`), and painted as a mannequin: a hull of shapes per body part, the bones inside it, a part map (`sprite`, `clip_sheet`). |
| `crates/pixfx-effects` | The effects, listed in `pixfx_effects::EFFECTS` and looked up with `pixfx_effects::effect(id)`. |
| `crates/pixfx-pixel` | Pixels for agents, not shipped in a game: a picture (an image model's, an artist's, a customer's sprite) cleaned to its pixel grid, read as text and drawn back, viewed, critiqued, split into parts; blank sheets for an image model (`pixfx_pixel::tool`, a script per step in `examples/`, skill `.claude/skills/pixfx-pixel/SKILL.md`). |
| `crates/pixfx-ref` | Effect references from an image model, not shipped in a game: a spec of an effect's look and moments becomes a labelled template sheet and a prompt, the OpenRouter request is built and read, and the sheet the model drew is cut back into frames as colour and light layers with a review page (a script per step in `examples/`, the xianxia shields as worked specs, skill `.claude/skills/pixfx-ref/SKILL.md`). |
| `crates/pixfx-export` | Sprite sheets (composite, colour, light PNG), beanfx-layout JSON, GIF, rendered clips, and the parity check (`parity`); the scripts that write them in `examples/`. |
| `games/arena/crates/pixfx-duel` | The landing duel's rules, deterministic from a seed: fighters, skills, the rotating bar, statuses, shots, rounds, the brain or the first mind in each corner, and the view the page reads. The site plays it through the wasm `Duel` (`site/assets/duel.js`). |
| `crates/pixfx-wasm` | The engine for JS: `effects()`, `meta(id)`, `new Fx(id, opts)`, `update(dt)`, `draw(layer)`, `color()`, `light()`, `rgba()`, `takeCues()`; `new Feel(spec?)`, `new Squash(spec?)`, `new Blink(spec?)`. Demo in `crates/pixfx-wasm/demo/`. |
| `packages/pixfx-phaser` | Phaser 4 scene plugin: `scene.pixfx.play(id, x, y, opts)` with follow, stop, `complete`/`destroy` events, scene time, game feel (`scene.pixfx.feel`) and `scene.pixfx.squash(sprite)`. See its [README](packages/pixfx-phaser/README.md); demo in `packages/pixfx-phaser/demo/`. |
| `packages/pixfx-canvas2d` | Canvas 2D shell over `pixfx-web`: `createPixfx(ctx, { scale }).play(id, x, y, opts)`, then each frame `update(dt)` and `drawLayer("back")` / `drawLayer("front")` among your own drawing, `warp()` for a heavy blow's ring, `drawPicture()` for fighters and scene light Rust painted; workers paint. See its [README](packages/pixfx-canvas2d/README.md); demo in `packages/pixfx-canvas2d/demo/`. |
| `packages/pixfx-pixi` | Pixi 8 shell: `createPixfx(app).play(id, x, y, opts)` with the same effects, events and feel as Phaser. See its [README](packages/pixfx-pixi/README.md); demo in `packages/pixfx-pixi/demo/`. |
| `packages/pixfx-web` | The JS runtime both JS shells run on: the wasm, the painting workers, the game-wide frames, a stage per host scene. |
| `crates/pixfx-gpui` | The gpui shell: `FxScene` plays effects through a `pixfx-scene` `Scene`, paints each at its view's origin and drops images from the atlas at the end of the frame. Frames are painted on the spot (`FxScene::default()`) or on native threads (`FxScene::with_threads(n)`), which turn them into images too. Needs the gpui fork (see [gpui](#gpui)). |
| `examples/gpui-web` | A gpui web app that plays the candle and campfire. Needs the gpui fork. |

Toolchain: `rust-toolchain.toml` pins the nightly and the wasm32 target. The commands below assume you are in
the repo root.

Optional: cargo's intermediate build files can go in one shared directory with
`export CARGO_BUILD_BUILD_DIR=$PWD/target/build-dir`. Nothing needs it.

The root workspace holds every crate except `crates/pixfx-gpui`, so a fresh clone builds and tests without
anything else. `crates/pixfx-gpui` and `examples/gpui-web` are separate workspaces that need a local gpui
fork; see [gpui](#gpui).

## Game feel

Effects say what an impact should do to the screen, and every host answers the same way.

- **Cues.** An effect fires timed cues on the frame it means them: `shake {trauma}`, `hitstop {dur}`,
  `ripple {x, y, r0, r1, dur, amp}`, `flash {k, dur}`, `sound {name, gain, pitch}`. The impact effects
  (explosions, dirt, plasma, implosion, parry, icicle fall, poof, feather burst) fire them by weight through
  `cue::impact`: each hit adds 0.3 (a light poof) to 0.6 (an explosion) trauma, and an explosion also stops,
  ripples and flashes. Their `feel`
  option (0 to 2, default 1) scales the cues, and `feel: 0` silences them. pixelfx fires none of these, so parity
  is unchanged.
- **`Feel`** (core) turns cues and the game's own impacts (`add_trauma`, `freeze`, `slow`) into one answer:
  `advance(dt)` returns the seconds that play (none in a hit-stop, a fraction in slow motion), `shake(strength)`
  gives a whole-pixel offset and `roll(strength)` a tilt in radians (strength is the player's setting, 0 holds
  still), and `flash()` gives 0 to 1. Trauma (0 to 1) drains 1 a second; each axis is the most it can move (16 px,
  0.05 rad) times trauma squared times smooth Perlin noise, so one light hit moves the screen a pixel and a heavy
  one 2 to 4. Hit-stops do not add up: the longest wins, capped at 13/60 s.
- **Hosts.** The Canvas `Stage` answers cues itself: its own loop shakes and flashes the view, and `update(dt)`
  returns the played seconds for the host's own world. `setFeel(false)` leaves the answer to the game, and cues
  still arrive in `events()`. `playThroughHitstop(handle, true)` lets one effect, a hit spark, play on in real time while
  a hit-stop freezes the rest (slow motion, which starts after the stop, slows it with the rest; its own hit-stops do not renew the stop). In Phaser, `scene.pixfx.feel` shakes and tilts the main camera, flashes, and freezes the
  scene's effects; the game reads `feel.timeScale`. Any other JS host uses `Fx.takeCues()` with a `Feel`.
- **Sprites.** `Squash` stretches a body along its velocity when it moves fast (up to 1.3 long at 220 px/s and
  faster), flattens it to 0.7 tall as it lands and eases back to itself within 0.15 s. The other axis takes the
  inverse, so the area stays the same, and the shape pivots at the feet. `shape(vx, vy)` gives the full shape as
  `[a, b, c, d]` for a canvas `transform` about the feet, leaning on a diagonal flight. `scale(vx, vy)` gives the
  nearest upright `[sx, sy]` for hosts that cannot skew. `Blink` blinks a body while it cannot be hurt. In Phaser,
  use `scene.pixfx.squash(sprite)`. The fighter paints with the full shape.

## Effects that know their sprite

An effect can play on a sprite: a character, a sword, a hat. The game hands it the sprite's current frame every
update (`effect.wrap(this.pixfx.actorOf(sprite))` in Phaser, `pixfx.actorOf(sprite)` in Pixi, `imageActor(...)` on a
Canvas 2D context, `FxScene::set_actor` in gpui), and an effect that wraps its actor (`Effect::wraps_actor`) reads it
and follows each frame. `feedback/power-up` grips the body with its aura; `feedback/outline` rings it.

To write one, read the sprite through `pixfx_core::Silhouette` (see `.claude/skills/pixfx-sprite-aware/SKILL.md`):

| Call | Gives |
|---|---|
| `Silhouette::of(&actor, area)` | The cells of an `Area` (a box of effect pixels) the sprite's opaque pixels cover. `Silhouette::from_fn` makes a stand-in body from shapes. |
| `cells()`, `covers(x, y)`, `bounds()`, `centroid()` | The body, a test, its box, its middle. |
| `edge()` | Cells on the body's edge: for rim light or sparks off it. |
| `ring(width, corners)` | Cells of an outline round it, square or rounded. |
| `around(max)`, `distance(x, y)` | Cells off the body with their distance in pixels: for an aura, a glow, a halo. |
| `pick(&mut rng, tries)` | A random cell of the body, the same from the same seed. |
| `Actor::opaque()` | The sprite's pixels with their colour: to tint the body or take its colours. |

### `feedback/outline`

A ring round the sprite it wraps, drawn behind it, so it reads over busy ground or marks what is picked:
`play("feedback/outline", x, y, { width: 2, palette: ["#ffffff"] }).wrap(actorOf(hero))`. `width` is 1 to 3,
`corners: 1` fills the corners (square ring), 0 leaves them cut (rounded, the default); the colour is the
palette's first, by default the engine's dark outline (`#1a1422`). `w` and `h` (default 128) size the box round where
it plays that the sprite and its ring must fit in. It draws nothing until it has a sprite.

Some effects draw an outline as part of their art (the explosions' rocks and clods, the spinning coin); their
`palette` and colour options recolour it.

## Palettes

Every effect takes a `palette`: a list of hex colours from dark to light, or one preset's name. A palette of
another length is resampled to the effect's own, so any ramp fits any effect.

| Preset | Ramp |
|---|---|
| `"fire"` | Soot red through orange to pale yellow. |
| `"ice"` | Deep blue through cyan to white. |
| `"poison"` | Dark moss through acid green to pale lime. |
| `"holy"` | Dark bronze through gold to white. |
| `"shadow"` | Near black through violet to pale lilac. |
| `"arcane"` | Indigo through blue violet to near white. |
| `"blood"` | Near black through crimson to pale pink. |
| `"electric"` | Night blue through cyan to white. |
| `"nature"` | Dark leaf through green to pale lime. |
| `"gold"` | Dark brown through gold to white. |

```js
stage.play("explosions/small-explosion", x, y, { palette: "poison" });
```

```sh
cargo run -p pixfx-export --example gif -- fire/campfire --palette ice
```

In Phaser, `(await loadPixfx()).palettePresets()` lists the names.

## Ripples and hit flash

- **Ripples.** A heavy impact's `ripple` cue pushes a ring out from where it lands, bending what is behind it. The
  Canvas `Stage`'s own loop bends the view each frame. A host that owns the loop calls `stage.warp()` once it has
  drawn everything the ring should bend. Any other host reads `ripples()` and bends RGBA pixels with
  `screen::ripple_rgba` (core).
- **Hit flash.** `HitFlash` (core, and `new HitFlash()` in JS) fills a struck sprite with one colour for a few
  frames: `start(secs?)` (5 frames at 60 a second when left out), `advance(dt)` with play time, then draw the
  sprite filled while `active()`, or blend the fill by `amount()` (1 down to 0). It runs on play time, so a
  hit-stop holds the struck sprite lit through the freeze.

## Render a sheet

```sh
cargo run -p pixfx-export --example sprite_sheet -- fire/campfire --out out
cargo run -p pixfx-export --example gif -- fire/campfire --scale 4 --out out
```

`sprite_sheet` writes the composite, colour and light PNGs and the beanfx-layout JSON into `out/`; `gif` writes a
looping GIF. For a seed or an effect's own option, change the `Options` in the script.

## What a frame costs

```sh
cargo run --release -p pixfx-effects --example bench -- --ids fire/candle,fire/campfire --n 50 --secs 10 --hz 60
```

The bench plays `--n` instances of each effect (default: every effect the build has, 50 each) on a `--hz` display for `--secs`
seconds, drawing an instance only when its clock reaches a new frame, as hosts do. Built with one effect's feature
(`--no-default-features --features fire-candle`), it measures that effect alone. It prints the milliseconds per
display frame spent updating every clock, then per effect its frame size (canvas plus glow margin), how many times it
drew, and the milliseconds spent drawing and turning the lit box into RGBA. Run it on an idle machine; the numbers are
native, and wasm in a browser is slower. `bun run perf` in `packages/pixfx-phaser` measures the browser.

## Parity against pixelfx

Parity compares every pixel of every frame with pixelfx's own render. It needs Node and a pixelfx checkout.

```sh
git clone https://github.com/patch-gg/pixelfx /tmp/pixelfx
git -C /tmp/pixelfx checkout 044e12a
cargo run -p pixfx-export --example parity -- fire/candle --pixelfx /tmp/pixelfx --out target/parity
```

The ports match pixelfx commit `044e12a` (0.3.1); a later pixelfx may have changed an effect. Only `water/water-jet`
differs from pixelfx, in 0.01% of its colour pixels, because libm `pow` and V8 `Math.pow` can disagree in the last bit;
the other 24 effects match pixel for pixel.

It prints `colour x% differ, light y% differ` and `PASS` or `FAIL` (gate: both at most 0.5%), and writes a
side-by-side `target/parity/<id>.side.gif`. Without `--pixelfx` it uses `$PIXELFX`, then `./pixelfx`. Exit
code 0 is PASS, 1 is FAIL, 2 means the comparison could not run. Effects that ship below the gate are listed in
`parity/LEDGER.md`; it is empty.

An effect ported from Sprite Fusion has its source vendored verbatim at `tools/spritefusion/<id>.ts`, and the parity
script checks it against that file instead (through `tools/spritefusion-ref.mjs`, which needs Node 22.6 or later to run
TypeScript; no pixelfx checkout). The source runs on the port's canvas, fps and seed, sub-stepped at 120 Hz as
pixelfx steps its effects. `arcane/hex-shield` and `feedback/power-up` match their sources pixel for pixel.

An effect that wraps its character (`feedback/power-up`) is checked on one as well: `--actor idle.png,cast.png`
stands each PNG with its feet on the effect's origin and shows them in turn, `--actor-every N` render frames each
(default 6). Both the port and the source see the same sprite on the same frame.

## Determinism test

Frame hashes of each effect are checked against goldens, natively and in wasm, each effect against its own entry
in `crates/pixfx-effects/tests/goldens/frame_hashes.json`. Every effect at once is heavy, so the test is ignored by
default; build one effect alone and it checks that one in a fraction of a second.

```sh
cargo test -p pixfx-effects --no-default-features --features fire-candle --test determinism -- --ignored
cargo test -p pixfx-effects --test determinism -- --ignored        # every effect
wasm-pack test --node crates/pixfx-effects --test determinism -- --include-ignored
```

## Tests

Each crate and package tests itself, and there is no repo-wide gate: run the tests of what you changed, as its
`AGENTS.md` lists under "Before you finish" (`cargo test -p <crate>`, `bun run check` in a package). Working on one
effect, build it alone, so nothing else compiles or runs:

```sh
cargo test -p pixfx-effects --no-default-features --features fire-candle --all-targets
```

## Ship only the effects you play

Each effect is a cargo feature of `pixfx-effects`, named after its id with the `/` as `-` (`fire/candle` is
`fire-candle`); each category (`fire`, `explosions`, ...) turns on all of its effects, and `all` turns on every one.
`pixfx-wasm` builds `all` by default. A game that plays a few effects builds only those, so
effects added to the library later cost it nothing:

```sh
wasm-pack build crates/pixfx-wasm --target web -- --no-default-features \
  --features pixfx-effects/fire-candle,pixfx-effects/explosions
```

The `EffectId` type in the generated `.d.ts` lists only the effects built in, and `new Fx(id)` with any other id
is an error. `crates/pixfx-effects/tests/features.rs` fails if an effect is added without its feature.

## Web demos

Each demo needs `wasm-pack`; the gpui demo needs `trunk`.

### Wasm engine (JS drives)

```sh
wasm-pack build crates/pixfx-wasm --target web --out-dir demo/pkg
cd crates/pixfx-wasm/demo && python3 -m http.server 8765 --bind 127.0.0.1
```

Open http://127.0.0.1:8765/.

### Canvas 2D

```sh
cd packages/pixfx-web && bun install && bun run build:wasm
cd ../pixfx-canvas2d && bun install && bunx vite demo
```

### gpui

`crates/pixfx-gpui` and `examples/gpui-web` need the local gpui fork checked out next to this repo at
`../zed-gpui`, branch `pixfx/additive-sprites`. Without it, neither builds; everything else does.

Check the gpui crate:

```sh
cargo clippy --manifest-path crates/pixfx-gpui/Cargo.toml -- -D warnings
```

Run the demo:

```sh
cd examples/gpui-web
trunk serve --release
```

Open http://127.0.0.1:8090/ and click the page once: gpui throttles a page that does not have focus, so the
effects run slowly until the page is focused. Add `?backend=webgl` or `?backend=webgpu` to force a renderer.
The first build takes about 5 minutes; later builds take seconds.

## Site

The landing page and docs are static HTML in `site/`, live at https://pixfx.pages.dev.

```sh
site/build.sh --serve        # build site/dist (pages + the pixfx-canvas2d bundle), serve on http://127.0.0.1:8770/
wrangler pages deploy site/dist --project-name pixfx --branch main
```

## Credits

See [NOTICE.md](NOTICE.md): pixelfx (Patch), Hugo Duprez / Sprite Fusion, Hollow Crown, beanfx (bien).

## License

MIT, see [LICENSE](LICENSE).
