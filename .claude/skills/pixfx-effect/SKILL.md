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
- An effect made here from an image model's sheet: skill `pixfx-ref` first (spec, parts sheet, moment sheets), then
  write it here a function per part, comparing a held frame with the sheet side by side until they match.

### Make it look hand-made

- **Glow as steps of colour, not a blur.** Pixel artists paint a glow as solid rings round each stroke: white core,
  pale, mid, dark, a dithered edge. Stamp strokes into `pixfx_draw::halo::Halo` with a heat and a kind,
  `spread(fall)`, and paint each pixel with the ramp colour of its heat's step (`arcane/restriction.rs` `paint`).
  Draw a stroke's own pixel in its own colour (kind) and let its heat only set how far its halo reaches, or thin
  strokes drown in their glow. A flash or a hit raises the heat, so the halo grows. Additive `glow` stays as a faint
  bloom on top; turning it up only blurs.
- **Opaque solids, light as light.** Stacked translucent pieces wash out to white; draw them opaque and put the glow
  in the light layer (`holy/lotus`).
- **Depth.** A ring round the character: its far half on `Layer::Back` (mirrored glyphs, a little dimmer), its near
  half on `Layer::Front`; a glyph seen edge-on shows only its middle stroke.
- **Counts and motion belong to code.** Writing one rune at a time, a ring's spin, exactly three rings: an image
  model will not keep these across frames, a timeline in code does (`written(r, k)`, `turn(r, t)`).
- **A blow anywhere.** Number options in local pixels from the feet (`hitX`, `hitY`, `.within(...)`), defaulting to
  where the reference drew the blow; from that point derive the struck piece, where the flare starts, the tilt, the
  sparks and the `impact` cue. Render a few with `gif ... --num hitX=-20 --num hitY=-48`.
- **Cost.** Work only over the box the strokes cover, and only for the layer being drawn: restriction's halo went
  from 11.75 to 4.06 ms for 50 by both, pixels unchanged. Bench before and after.
- **Bake what nothing changes; draw live what the game sets.** Choose by what the work depends on:

  | The work depends on | Do it | Example |
  |---|---|---|
  | nothing the game sets: a grown tree, cracks, a layout, a map | once, in a script `examples/bake_<name>.rs` that writes it as data into `src/<category>/<name>_<what>.rs` ("Baked by ...; do not edit, run the script again"); the effect reads it, built into a map once for every instance (`std::sync::OnceLock`) | `shadow/winter-ordeal` (its sky), `shadow/blood-shield` (its veins: starting a shield went from 10.6 ms to 0) |
  | only where a pixel is, the same on every frame (a shape's reach, a highlight's angle) | a table built once (`OnceLock`), read per pixel; a point off it (moved by a dent, a turned painter) is worked out as before, so the pixels stay the same bit for bit | `blood_shield.rs` `Px`: 3 `pow` and an `atan2` a pixel gone |
  | the time, an option (`hitX`), the seed | live, every frame | the blow, the heartbeat |

  A whole effect with no options and a costly draw can also ship as a baked clip of frames (skill `pixfx-render`,
  `loadBaked`); a clip is fixed to one set of options and costs download and memory, so bake data first. A bake made
  from one seed gives every seed that layout: bless the goldens again and say so in the commit. Profile before you
  bake (`sample <pid>` on the bench): bake what the profile shows, not what looks heavy.

### Hit at any distance: an effect that takes orders

An effect draws on its own canvas, so it cannot reach a foe far off. One that throws at a foe the game names (a
sword formation, a volley of arrows, a talisman barrage) takes an **order**: it holds until the game calls
`attack(x, y, aim, caster)`, then answers with pieces, each another registered effect, and the scene flies them
(`pixfx_core::volley`, `Scene::order`). The point gives the direction, not where they burst: a piece flies its path
and on past the point, and bursts on the first **body** it meets (the game declares bodies with `setBody` /
`Scene::set_body`; the caster's is spared), or when the game calls `burst`. `arcane/sword-formation.rs` is the
worked example.

```rust
// the formation: opens once, then loops while it waits for its order
fn loop_from(&self) -> Option<f64> { Some(OPENED) }
fn order(&self, opts: &Options, t: f64, at: (f64, f64), flip: bool, order: &Order) -> Vec<Launch> {
    let shots: Vec<Shot> = (0..N).map(|k| {
        let (x, y, a) = where_shot_k_is(k, t);                    // local pixels, the way it points
        Shot { at: (at.0 + if flip { -x } else { x }, at.1 + y), heading: if flip { PI - a } else { a } }
    }).collect();
    VOLLEY.aim(&shots, order, opts.seed.unwrap_or(META.seed) as f64)
}
static VOLLEY: Volley = Volley {
    piece: &FLYING_SWORD, hit: Some(&SWORD_STRIKE),               // effects of their own
    held: Some(&HELD_SWORD), poise: 0.12,                         // hangs where it was, turning to its way, first
    tail: Some(Tail { effect: &TRAIL, opts: tail }),              // a trail swept behind it (projectiles/trail)
    gap: 0.1, speed: 300.0, min_secs: 0.2, max_secs: 2.4,         // launched in turn, nearest the target first
    nose: 7.0, sink: 2.0,                                         // the piece strikes with its tip
    bend: 0.2, bend_min: 12.0, bend_max: 28.0,                    // converge: a short swing out, then curving in
    spread: (6.0, 12.0), ease: 1.0, beyond: 200.0,                // flies on this far past the point, then fades
    lands: false,                                                 // true: it also strikes the point itself
};
```

- `Volley::aim` plans any like shots: `converge` curves each into the point (it strikes and goes), `pierce` flies
  straight through and on (it strikes as it passes). A path of your own: build `Launch`es yourself.
- The piece is drawn pointing along +x from its origin, with no streak of its own: its trail is `projectiles/trail`
  (a `tail`), a column swept along the way it really flew, so it curves with it. The scene turns the piece to its
  heading in 32 steps, so frames are shared, mirrored as its caster is. The strike effect fires the `impact` cue; the game gets a `strike` event per shot
  and decides the damage.
- Once ordered, the formation is no longer drawn (its shots are it) and ends with its last shot; a second order
  returns false.
- Register the shot and the strike as effects of their own (JS workers find every effect by id), each behind
  `cfg(any(feature = "<its own>", feature = "<the formation's>"))` in `mod.rs` and `EFFECTS`, so the formation
  built alone brings them; shared drawing goes in a module under the same `any()` (`arcane/sword.rs`).
- On the site it plays the `volley` role (`effect-roles.js`: `hold`, `aim`, `blows`, `power`); test the flight in
  `crates/pixfx-scene/tests/order.rs` style: strikes near the point, in turn, and the formation gone after.

### A streak, or an effect that reacts to how it moves

A streak is a brush swept along a path, as a camera sees a bright thing waved in a long exposure: a point leaves a
line, a column a ribbon, a turning bar a fan. Draw every streak with `pixfx_draw::sweep::sweep` (a `Brush`:
`Brush::column(palette, width)`, or `Brush::from_column(frame, x, origin_y)` for a sprite's own pixels; a `Look`
for how age colours, thins, waves and breaks it up), never with lines of your own. The path is the effect's own
(a slash: a bar turned through an arc; a spark: a point along its flight) or the way it was really moved: ask for it
with `Effect::traces(opts) -> Option<secs>` and take it in `EffectState::set_trace`; `trace.poses(secs)` feeds the
sweep. Read motion from the trace with `pixfx_core::motion` (`velocity`, `speed`, `heading`, `turn`, `spin`,
`acceleration`), never by differencing yourself, so every effect agrees (`fire/lib.rs` `carried`: a fire leans
back against its velocity). An effect that reads its motion paints its own frames (no sharing), so ask only when an
option says it is moved (`carried: 1`), and keep a port's pixels when it stands still (`fire/lib.rs` `nudge`).

**The brush is the mover's cross-section: pick it by the mover's shape.**

| Mover | Brush | Example |
|---|---|---|
| thin: a sword's point, a drop, a spark, a bolt | `Brush::column(palette, width)`, a few pixels wide, via `projectiles/trail` laid under it | `arcane/sword-formation` tail, `blood_spray` |
| broad: a mirror, a shield, a disc | `Brush::edge(its pixels, 0)`: its back edge, as tall as itself, in its own colours, swept in its own draw over its own trace | `arcane/bagua-mirror-flying` (its jade rim) |
| a character (a sprite) | `projectiles/trail` played on it: it wraps the sprite (`set_actor`) and smears its back edge one in from the outline (`Brush::edge(.., 1)`), upright (`Turn::Angle(0)`, poses' angles 0), since a body does not turn round when it walks back | the trail page: fighters gliding |
| a blade that turns | `Brush::from_column(frame, x, origin_y)` with `Turn::Angle` | a slash smear |

A narrow ribbon behind a broad body reads as a beam stuck on it, not as its motion: the bagua mirror's first
shared trail was a 7-pixel jade band behind a 30-pixel mirror and looked worse than the hand-drawn lines it
replaced, and a trail played on a fighter swept one point at palm height. An effect that rides a body must be aware
of it, as a wrap is: read the sprite, never stand in for it with a point. Take the edge colour from the right pixel
(a sprite's backmost pixel is its dark outline: the smear came out a shadow box until it read one in) and keep the
brush upright on a body that does not turn (turned with its heading it swung round the fighter as they turned back).

**Replacing a look that already exists** (a hand-drawn trail moved onto the shared sweep, any restyle): render the
old and the new side by side where the game shows them (`stage` with `--order-at`, zoomed), and show the user before
blessing or deploying. Shared code is a means; a change that looks worse than what it replaces is a regression even
when it is more general.

## 2. Register it (all four, or `tests/features.rs` fails)

1. `src/<category>/mod.rs`: `#[cfg(feature = "fire-ember")] mod ember;` and
   `#[cfg(feature = "fire-ember")] pub use ember::EMBER;`
2. `Cargo.toml`: `fire-ember = []` (the id with `/` as `-`), added to its category feature (`fire = [...]`); a new
   category also goes into `all`.
3. `src/lib.rs`: `#[cfg(feature = "fire-ember")] &fire::EMBER,` in `EFFECTS`.
4. Check once with every effect: `cargo test -p pixfx-effects --test features`.

On the site an effect's page stages it by its role: one with a blow at a set time (a shield) gets an entry in
`site/assets/effect-roles.js` (`role: "ward", blows: [<its HIT>], directional: true`), and its own number options a
slider each in `site/assets/effect.js` `OWN`. Rebuild with `site/build.sh`.

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
| Matching a reference's glow by turning `glow` up | Read its colour and light layers; paint the halo in steps (`pixfx_draw::halo`) |
| A blow fixed to one side | `hitX` / `hitY` options and everything derived from that point |
