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
| `crates/pixfx-core` | The engine: frame with colour and additive light, painter, `Effect`/`Instance`, `render_frames`, `composite`, `frame_hash`; game feel: `Feel` (shake, hit-stop, slow motion, flash, ripples), `cue::impact`, `Squash`/`Blink`. |
| `crates/pixfx-effects` | The effects, listed in `pixfx_effects::EFFECTS` and looked up with `pixfx_effects::effect(id)`. |
| `crates/pixfx-cli` | The `pixfx` binary: `parity`, `sheet`, `render`. |
| `crates/pixfx-export` | Sprite sheets (composite, colour, light PNG), beanfx-layout JSON, GIF. |
| `crates/pixfx-wasm` | The engine for JS: `effects()`, `meta(id)`, `new Fx(id, opts)`, `update(dt)`, `draw(layer)`, `color()`, `light()`, `rgba()`, `takeCues()`; `new Feel(spec?)`, `new Squash(spec?)`, `new Blink(spec?)`. Demo in `crates/pixfx-wasm/demo/`. |
| `crates/pixfx-canvas` | A renderer with its own loop on a Canvas 2D: `new Stage(canvas, scale)`, `play`, `move`, `stop`, `remove`, `start`, `pause`, `setScale`, `background`, `tick`; for a host-owned loop `update(dt)` then `drawLayer("back")` / `drawLayer("front")` around its own sprites; game feel: `shake(strength)`, `flash()`, `frozen()`, `addTrauma`, `freeze`, `slow`, `setFeel(on)`, cue events. Demo in `crates/pixfx-canvas/demo/`. |
| `packages/pixfx-phaser` | Phaser 4 scene plugin: `scene.pixfx.play(id, x, y, opts)` with follow, stop, `complete`/`destroy` events, scene time, scene-wide layers, game feel (`scene.pixfx.feel`) and `scene.pixfx.squash(sprite)`. See its [README](packages/pixfx-phaser/README.md); demo in `packages/pixfx-phaser/demo/`. |
| `crates/pixfx-gpui` | Live playback inside gpui. Needs the gpui fork (see [gpui](#gpui)). |
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
  `cue::impact`: a light poof only shakes a little, an explosion shakes, stops, ripples and flashes. Their `feel`
  option (0 to 2, default 1) scales the cues, and `feel: 0` silences them. pixelfx fires none of these, so parity
  is unchanged.
- **`Feel`** (core) turns cues and the game's own impacts (`add_trauma`, `freeze`, `slow`) into one answer:
  `advance(dt)` returns the seconds that play (none in a hit-stop, a fraction in slow motion), `shake(strength)`
  gives a smooth whole-pixel offset (strength is the player's setting, 0 holds still), and `flash()` gives 0 to 1.
  Hit-stops do not add up: the longest wins, capped at 13/60 s.
- **Hosts.** The Canvas `Stage` answers cues itself: its own loop shakes and flashes the view, and `update(dt)`
  returns the played seconds for the host's own world. `setFeel(false)` leaves the answer to the game, and cues
  still arrive in `events()`. In Phaser, `scene.pixfx.feel` shakes the main camera, flashes, and freezes the
  scene's effects; the game reads `feel.timeScale`. Any other JS host uses `Fx.takeCues()` with a `Feel`.
- **Sprites.** `Squash` stretches a body along a fast rise or fall, flattens it as it lands and eases back,
  keeping its area. `Blink` blinks it while it cannot be hurt. In Phaser, use `scene.pixfx.squash(sprite)`. The
  fighter paints with the same rules.

## Render a sheet

```sh
cargo run -p pixfx-cli -- sheet fire/campfire --out out
cargo run -p pixfx-cli -- render fire/campfire --gif --scale 4 --out out
```

`sheet` writes the composite, colour and light PNGs and the beanfx-layout JSON into `out/`. `render --gif`
writes a looping GIF.

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
