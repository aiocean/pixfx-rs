# pixfx-rs

Pixel-art effects in Rust. An effect is written once and produces the same pixels natively and in WebAssembly.

It has two modes:

- **Render.** Run an effect natively and write sprite sheets (composite, colour and light PNGs), a
  beanfx-layout JSON, or a GIF.
- **Play live.** Run the same effect in a browser, through plain JS bindings, a self-driving Canvas 2D
  renderer, or inside gpui.

Effects are ports of [pixelfx](https://github.com/patch-gg/pixelfx) (JS) and use the
[beanfx](NOTICE.md) sheet layout. Every effect in `pixfx_effects::EFFECTS` is a port that passes the parity gate against pixelfx.

## Crates

| Crate | What it is |
|---|---|
| `crates/pixfx-draw` | Drawing, the lowest layer: frame with colour and additive light, painter, shapes, blending, `composite`, `frame_hash`, palette presets, outline rings, the JS-number maths (`math`). Knows nothing of effects or hosts. |
| `crates/pixfx-core` | The engine on top of `pixfx-draw`: `Effect`/`Instance`, particles, `render_frames`, `Player`, the frame key and store; game feel: `Feel` (shake, hit-stop, slow motion, flash, ripples), `cue::impact`, `Squash`/`Blink`. |
| `crates/pixfx-scene` | What a stage shows and where, for every shell: `Scene` (effects playing, placement, layers, shared frames, the view outside which nothing is drawn, the answer to their cues, events) and the `Shell` contract (`make`, `refill`, `show`, `free`). No browser or gpui types. |
| `crates/pixfx-effects` | The effects, listed in `pixfx_effects::EFFECTS` and looked up with `pixfx_effects::effect(id)`. |
| `crates/pixfx-cli` | The `pixfx` binary: `parity`, `sheet`, `render`, `bake`, `bench`. |
| `crates/pixfx-export` | Sprite sheets (composite, colour, light PNG), beanfx-layout JSON, GIF. |
| `crates/pixfx-wasm` | The engine for JS: `effects()`, `meta(id)`, `new Fx(id, opts)`, `update(dt)`, `draw(layer)`, `color()`, `light()`, `rgba()`, `takeCues()`; `new Feel(spec?)`, `new Squash(spec?)`, `new Blink(spec?)`. Demo in `crates/pixfx-wasm/demo/`. |
| `crates/pixfx-canvas` | A renderer with its own loop on a Canvas 2D: `new Stage(canvas, scale)`, `play`, `move`, `stop`, `remove`, `start`, `pause`, `setScale`, `background`, `tick` (which draw only what the canvas shows); for a host-owned loop `update(dt)` then `drawLayer("back")` / `drawLayer("front")` around its own sprites, with `setView(x, y, w, h)` / `clearView()` to draw only what its camera sees; game feel: `shake(strength)`, `roll(strength)`, `flash()`, `frozen()`, `addTrauma`, `freeze`, `slow`, `setFeel(on)`, cue events. Demo in `crates/pixfx-canvas/demo/` (`wide.html`: 100 fires panned past with `?view=on` or `off`). |
| `packages/pixfx-phaser` | Phaser 4 scene plugin: `scene.pixfx.play(id, x, y, opts)` with follow, stop, `complete`/`destroy` events, scene time, scene-wide layers, game feel (`scene.pixfx.feel`) and `scene.pixfx.squash(sprite)`. See its [README](packages/pixfx-phaser/README.md); demo in `packages/pixfx-phaser/demo/`. |
| `crates/pixfx-gpui` | The gpui shell: `FxScene` plays effects through a `pixfx-scene` `Scene`, paints each at its view's origin and drops images from the atlas at the end of the frame. Needs the gpui fork (see [gpui](#gpui)). |
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
  a hit-stop freezes the rest (slow motion still slows it). In Phaser, `scene.pixfx.feel` shakes and tilts the main camera, flashes, and freezes the
  scene's effects; the game reads `feel.timeScale`. Any other JS host uses `Fx.takeCues()` with a `Feel`.
- **Sprites.** `Squash` stretches a body along its velocity when it moves fast (up to 1.3 long at 220 px/s and
  faster), flattens it to 0.7 tall as it lands and eases back to itself within 0.15 s. The other axis takes the
  inverse, so the area stays the same, and the shape pivots at the feet. `shape(vx, vy)` gives the full shape as
  `[a, b, c, d]` for a canvas `transform` about the feet, leaning on a diagonal flight. `scale(vx, vy)` gives the
  nearest upright `[sx, sy]` for hosts that cannot skew. `Blink` blinks a body while it cannot be hurt. In Phaser,
  use `scene.pixfx.squash(sprite)`. The fighter paints with the full shape.

## Outlines

Every effect takes an `outline` option. Left out, the effect looks as designed. A few effects draw an outline of
their own: the explosions' rocks and clods, and the spinning coin. The option recolours that outline or turns it
off. Any other effect gets a ring the engine draws round its shapes. Smoke, haze and lone motes, which the painter
draws as a scatter of single pixels, get no ring.

| `outline` | Result |
|---|---|
| left out | As designed. |
| `false`, `"off"` | No outline, not even the effect's own. |
| `true`, `"dark"` | The engine's dark 1 px outline (`#1a1422`). |
| `"black"`, `"white"` | A 1 px outline in that colour; white reads over dark scenes. |
| `"thick"` | Dark, 2 px, square corners. |
| `"#ffcc00"` | A 1 px outline in any hex colour. |
| `{color, width, corners}` | `color` is a preset name or a hex colour, `width` is 1 to 3, `corners` fills them (square) or leaves them cut (rounded). Each defaults to the dark 1 px ring. |

The engine's ring goes in the colour layer, so the effect's glow still shows over it. Elsewhere, an effect's
darkest palette step does the work of an edge (the candle's wax, the icicle, the hit stars), and the effect's
`palette` and colour options recolour it. pixelfx has no outline option, so effects left without one match it.

```sh
cargo run -p pixfx-cli -- render fire/candle --gif --outline white
```

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
cargo run -p pixfx-cli -- render fire/campfire --gif --palette ice
```

In Phaser, `PALETTE_PRESETS` lists the names.

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
cargo run -p pixfx-cli -- sheet fire/campfire --out out
cargo run -p pixfx-cli -- render fire/campfire --gif --scale 4 --out out
```

`sheet` writes the composite, colour and light PNGs and the beanfx-layout JSON into `out/`. `render --gif`
writes a looping GIF.

## What a frame costs

```sh
cargo run --release -p pixfx-cli -- bench --ids fire/candle,fire/campfire --n 50 --secs 10 --hz 60
```

`bench` plays `--n` instances of each effect (default: the three fires, 50 each) on a `--hz` display for `--secs`
seconds, drawing an instance only when its clock reaches a new frame, as hosts do. It prints the milliseconds per
display frame spent updating every clock, then per effect its frame size (canvas plus glow margin), how many times it
drew, and the milliseconds spent drawing and turning the lit box into RGBA. Run it on an idle machine; the numbers are
native, and wasm in a browser is slower. `bun run perf` in `packages/pixfx-phaser` measures the browser.

## Parity against pixelfx

Parity compares every pixel of every frame with pixelfx's own render. It needs Node and a pixelfx checkout.

```sh
git clone https://github.com/patch-gg/pixelfx /tmp/pixelfx
git -C /tmp/pixelfx checkout 044e12a
cargo run -p pixfx-cli -- parity fire/candle --pixelfx /tmp/pixelfx --out target/parity
```

The ports match pixelfx commit `044e12a` (0.3.1); a later pixelfx may have changed an effect. Only `water/water-jet`
differs from pixelfx, in 0.01% of its colour pixels, because libm `pow` and V8 `Math.pow` can disagree in the last bit;
the other 24 effects match pixel for pixel.

It prints `colour x% differ, light y% differ` and `PASS` or `FAIL` (gate: both at most 0.5%), and writes a
side-by-side `target/parity/<id>.side.gif`. Without `--pixelfx` it uses `$PIXELFX`, then `./pixelfx`. Exit
code 0 is PASS, 1 is FAIL, 2 means the comparison could not run. Effects that ship below the gate are listed in
`parity/LEDGER.md`; it is empty.

## Determinism test

Frame hashes of each effect are checked against goldens, natively and in wasm. The test is heavy, so it is
ignored by default; run it before merging an effect.

```sh
cargo test -p pixfx-effects --test determinism -- --ignored
wasm-pack test --node crates/pixfx-effects --test determinism -- --include-ignored
```

## Whole-repo check

```sh
cargo fmt --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace
(cd packages/pixfx-phaser && bun run check)
bun test site/tests
```

## Ship only the effects you play

Each effect is a cargo feature of `pixfx-effects`, named after its id with the `/` as `-` (`fire/candle` is
`fire-candle`); each category (`fire`, `explosions`, ...) turns on all of its effects, and `all` turns on every one.
`pixfx-wasm` and `pixfx-canvas` build `all` by default. A game that plays a few effects builds only those, so
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

### Canvas renderer

```sh
wasm-pack build crates/pixfx-canvas --target web --out-dir demo/pkg
cd crates/pixfx-canvas/demo && python3 -m http.server 8766 --bind 127.0.0.1
```

Open http://127.0.0.1:8766/.

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
site/build.sh --serve        # build site/dist (pages + pixfx-canvas wasm), serve on http://127.0.0.1:8770/
wrangler pages deploy site/dist --project-name pixfx --branch main
```

## Credits

See [NOTICE.md](NOTICE.md): pixelfx (Patch), Hugo Duprez / Sprite Fusion, Hollow Crown, beanfx (bien).

## License

MIT, see [LICENSE](LICENSE).
