---
name: pixfx-effect
description: Use when adding a new pixfx effect, porting one from pixelfx or Sprite Fusion's pixel art VFX, or changing how an existing effect looks, moves, fires cues or takes options - including when its golden frame hashes, parity check or feature registration fail.
---

# Write or change a pixfx effect

An effect is a Rust static in `crates/pixfx-effects/src/<category>/<name>.rs` with a `Meta` and a `spawn` that
returns its state; the state draws a frame from time `t`. Every renderer, script and shell picks it up from the
registry. Work on it **built alone** behind its own feature, so nothing else compiles or runs.

## 1. Write it

Copy the nearest effect of its kind (`fire/candle.rs` for a loop, `explosions/small_explosion.rs` for a one-shot that
fires cues, `arcane/hex_shield.rs` for a Sprite Fusion port) and change it. The shape:

```rust
use pixfx_core::{Effect, EffectState, Layer, Meta, NumOpt, Options};
use pixfx_draw::Painter;
use pixfx_draw::color::Rgba;

pub struct Ember;
pub static EMBER: Ember = Ember;

const SIZE: NumOpt = NumOpt::new("size", 3.0).within(1.0, 8.0); // an option of its own, held to a range
const META: Meta = Meta {
    id: "fire/ember",
    name: "Ember",
    desc: "A single coal glowing and dimming.",
    tags: &["fire", "prop", "light", "loop"],
    size: (40, 40),   // room for the glow's radius on every side of the origin
    origin: (20, 24), // the point that lands where the caller plays it
    duration: 2.0,    // seconds per loop (or of the one-shot)
    looping: true,
    fps: 30,
    seed: 1,
    state: None,
    options: &[SIZE.spec()],
};
const PALETTE: [Rgba; 3] = [
    Rgba::hex24(0x4a1712),
    Rgba::hex24(0xf2561d),
    Rgba::hex24(0xffc23d),
];

impl Effect for Ember {
    fn meta(&self) -> &Meta {
        &META
    }
    fn spawn(&self, opts: &Options, seed: u64) -> Box<dyn EffectState> {
        Box::new(State {
            ramp: opts.palette_or(&PALETTE),
            size: opts.num(&SIZE),
            seed: seed as f64,
        })
    }
}

struct State {
    ramp: Vec<Rgba>,
    size: f64,
    seed: f64,
}

impl EffectState for State {
    fn draw(&self, p: &mut Painter, t: f64, layer: Layer) {
        if layer == Layer::Back {
            return; // nothing behind the character
        }
        // 0..1, one period per `duration` (2 s), so the loop closes exactly
        let k = 0.5 + 0.5 * libm::sin(t * core::f64::consts::PI + self.seed);
        let last = self.ramp.len().saturating_sub(1);
        let pick = (k * last as f64) as usize; // dark to bright as it glows
        let Some(&c) = self.ramp.get(pick.min(last)) else {
            return;
        };
        let s = self.size;
        p.rect(-s, -s, s * 2.0, s * 2.0, c, 1.0);
        p.glow(0.0, 0.0, 10.0, c, 0.6 * k); // light is additive, drawn into the light layer
    }
}
```

- `size` must hold the light: the canvas reaches past the origin by at least each glow's radius, or the light is
  cut at the canvas edge. Look at the gif.
- Draw in local space: `(0, 0)` is the origin, y grows down. The instance places, flips, rotates, fades and scales
  light; the effect never does.
- Draw from `t` and the seed when you can (any frame on its own, loops close exactly). Real physics keeps state in
  `EffectState::update(dt)`, called per sub-step of at most 1/120 s.
- Options: `NumOpt` (`opts.num(&X)`), `RampOpt` (`opts.ramp(&X)`), `ColorOpt` (`opts.color(&X)`), the palette
  (`opts.palette_or(&PALETTE)`); list each `.spec()` in `meta.options`. An option that sizes a loop gets `.within`.
- An impact asks the game for shake, hit-stop, ripple and flash with `fn cues(&self, opts) -> Vec<TimedCue>`
  returning `pixfx_core::impact(t, weight, x, y, opts)`, and lists `pixfx_core::FEEL.spec()` in its options.
- Maths: `libm::*`, `pixfx_draw::math::{js_round, js_min, js_max, clamp, smooth}`; never `f64::sin`/`sqrt`/`round`
  (clippy denies them). Randomness from the seed (`math::rand`, `hash`, `noise`, a seeded `Rng`). No `unwrap`.
- A port keeps its source's number semantics: same order of operations, `js_*` helpers where JS differs (NaN in
  `Math.min`, `| 0`, clamped arrays). Do not tidy the maths. The module doc names the source file it ports.

## 2. Register it (all four, or `tests/features.rs` fails)

1. `src/<category>/mod.rs`: `#[cfg(feature = "fire-ember")] mod ember;` and
   `#[cfg(feature = "fire-ember")] pub use ember::EMBER;`
2. `Cargo.toml`: `fire-ember = []` (the id with `/` as `-`), added to its category feature (`fire = [...]`); a new
   category also goes into `all`.
3. `src/lib.rs`: `#[cfg(feature = "fire-ember")] &fire::EMBER,` in `EFFECTS`.
4. Check once with every effect: `cargo test -p pixfx-effects --test features`.

## 3. Test it alone

```sh
F="--no-default-features --features fire-ember"
cargo fmt -p pixfx-effects
cargo clippy -p pixfx-effects $F --all-targets -- -D warnings
cargo test -p pixfx-effects $F --all-targets                                   # its tests only
cargo run -p pixfx-export --example gif -- fire/ember --scale 4               # look first: target/out/fire_ember.gif
PIXFX_BLESS=1 cargo test -p pixfx-effects $F --test determinism -- --ignored  # then record its golden hashes
cargo test -p pixfx-effects $F --test determinism -- --ignored                 # and check them
cargo run --release -p pixfx-effects $F --example bench                        # what a frame of 50 costs
```

Run from the repo root (the scripts write to `target/out` relative to it). In zsh write the flags out (`$F`
does not split there). A cold build takes minutes (release bench ~7 min the first time); it has not hung. Blessing with one effect built rewrites only its entry in
`tests/goldens/frame_hashes.json`. Bless only after you have looked at the gif: a golden records what is, not what
is right. A `frame_hash` change to an existing effect needs its reason in the commit message.

A change shared by many effects (a category's `lib.rs`, `pixfx-core`, `pixfx-draw`) runs the same commands without
the feature flags (every effect), plus the wasm check once:
`wasm-pack test --node crates/pixfx-effects --test determinism -- --include-ignored`.

## 4. Parity, for a port

```sh
cargo run -p pixfx-export --example parity -- fire/candle --pixelfx /tmp/pixelfx    # a pixelfx port
cargo run -p pixfx-export --example parity -- arcane/hex-shield                      # a Sprite Fusion port
```

- pixelfx: clone `https://github.com/patch-gg/pixelfx` to `/tmp/pixelfx`, checkout `044e12a`; needs Node.
- Sprite Fusion: vendor the source verbatim at `tools/spritefusion/<category>/<name>.ts` (Node 22.6+); the parity
  script finds it there and runs it on the port's canvas, fps and seed. An effect that wraps a character checks with
  `--actor idle.png,cast.png [--actor-every 6]`.
- PASS is at most 0.5% of colour and of light pixels differing; it writes `target/parity/<id>.side.gif` (source
  left, port right). Exit 0 PASS, 1 FAIL, 2 could not run. Shipping below the gate needs the owner's approval and a
  line in `parity/LEDGER.md`.
- An effect made here (no source) has no parity check; say so in its module doc.

## Common mistakes

| Mistake | Fix |
|---|---|
| `cargo test -p pixfx-effects` while working on one effect | Add `--no-default-features --features <its feature>` |
| A test naming a specific effect breaks single-effect builds | `required-features` in `Cargo.toml`, or `tests/built/mod.rs`'s `built(id)` |
| Golden blessed before looking at the frames | Look at the gif first; bless records whatever is drawn |
| `f64::floor`, `x.round()`, `rand::random` | `libm::floor`, `js_round`, the seed |
| Fixing a parity FAIL by changing the source's maths order | Match the source's operations, including its JS quirks |
