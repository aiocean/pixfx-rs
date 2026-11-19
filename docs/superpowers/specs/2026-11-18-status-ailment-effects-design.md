# Design: four looping status ailment effects (burning, frozen, poisoned, petrified)

The chain "add status ailment effects to pixfx". Research is `docs/research-status-effects.md` (the anatomy of a
looping status, the registration checklist, bench numbers, the MIT sources); this document is the design an engineer
builds each slice from, alone. Every path is relative to the repo root; every `file:line` was checked against the
worktree at commit `fd116c0` (`git log --oneline -1`). Nothing here is committed code: the design only.

What ships: `feedback/burning`, `feedback/frozen`, `feedback/poisoned`, `feedback/petrified`, four looping,
sprite-aware effects a game plays on a character for as long as the ailment lasts and ends with `stop()` (a 0.3 s
fade, `crates/pixfx-core/src/effect.rs:27`). They join `feedback/electrified` (shocked), which already exists.
`crates/pixfx-effects/tests/status.rs` fails today on the four missing ids and is the gate: it demands, per id, a
registered effect (`status.rs:25`), `looping` (`:30`), `wraps_actor()` (`:33`), the tag `status` (`:36`), a golden
entry (`:52`) and at least two distinct frame hashes over `render_frames(fx, &Options::default(), 0)` (`:55-59`).

Decisions taken in this document, in one place (the reasons are in the sections):

| Decision | Choice |
|---|---|
| Shape | Four independent effect files, as `electrified.rs` and `soaked.rs` are (option A, section 4), plus one small shared module `src/feedback/status.rs` (`Body`, the stand-in figure, `floor_light`) and one `Silhouette::open` query in `pixfx-core` (section 3) |
| Canvas | 64 x 64, origin (32, 55) for all four: the ruler's cell (`crates/pixfx-draw/src/ruler.rs:42-53`) with the feet raised 5 rows for a pool, frost or dust under them (section 1.1) |
| Clock | 16 fps like the three ports (`electrified.rs:23`, `soaked.rs:21`, `encase.rs:24`); every beat is a whole number of frames (section 1.1) |
| Petrified | `Effect::loop_from` (`effect.rs:139-144`): the stone creeps up once over 12 frames (0.75 s), then 16 frames (1 s) of held statue loop (section 2.4) |
| Frozen | A pure loop, as its source is (section 2.2) |
| Cues | None, for all four (section 1.3) |
| The body | Drawn over the sprite on `Layer::Front` in the sprite's own colours tinted; the sprite is never moved (no shiver, no sway: section 1.4) |

---

## 1. Acceptance criteria

### 1.1 Common to the four

Checkable by the commands in 1.5, or on the gif.

- **Id and feature.** `feedback/<name>`, cargo feature `feedback-<name>` listed in `feedback = [...]`
  (`crates/pixfx-effects/Cargo.toml:21`) with its own `feedback-<name> = []` line among `Cargo.toml:31-36`, the
  `mod`/`pub use` pair in `crates/pixfx-effects/src/feedback/mod.rs` (as `mod.rs:7-8` and `:20-21`) and the `EFFECTS`
  entry in `crates/pixfx-effects/src/lib.rs` (as `lib.rs:33-34`), each under `#[cfg(feature = "feedback-<name>")]`.
  `tests/features.rs:36-48` and `:51-79` check it.
- **Meta.** `looping: true`, `fps: 16`, `wraps_actor()` true (`effect.rs:125-127`), `state: None`, tags that include
  `"feedback"`, `"status"`, `"character"`, `"loop"` (the convention of `electrified.rs:48-56`), and `size: (64, 64)`,
  `origin: (32, 55)`. Local space is then x in -32..31 and y in -55..8 (y down, (0, 0) at the feet). Why this
  canvas: the ruler's character (`RULER`, `ruler.rs:42-53`) stands 40 rows by 20 columns in a 64 x 64 cell with its
  feet at (32, 60); the measured idle sprite (`crates/pixfx-pixel/tests/fixtures/thunder-idle.png`, 64 x 64) covers
  x -12..7 and y -38..1 from its feet. The four effects need up to 15 rows above the crown (flames at full flare,
  smoke, falling snow: y down to -53) and 8 rows under the soles (a pool, frost, a dust skirt: y up to 8), and up to
  26 columns either side (the poison ring, the dust skirt), so the cell is kept and the feet are raised 5 rows.
  `Meta::glow_margin()` (`effect.rs:80-82`) is then 32 and shells and the bench see a 128 x 128 frame; what costs
  is the lit box, not the frame (`tests/crop.rs:21-45`).
- **Clock.** 16 fps; `duration * 16` is a whole number of frames (`Meta::frames`, `effect.rs:73-76`), and every
  beat (a tick, a shiver, a pop) sits on a frame. `draw` derives the frame as the ports do,
  `k = js_round(t * 16) mod N` (`electrified.rs:484`), and never keeps state: `update(dt)` stays the default
  (`effect.rs:163`) so `tests/live_frames.rs:153-183` holds (the same clock frame draws the same pixels).
- **The sprite.** Read once per pose in `set_actor` (`effect.rs:170`), never in `draw`: the `Silhouette`
  (`crates/pixfx-core/src/silhouette.rs:68`), the colours through `Actor::opaque()`
  (`crates/pixfx-core/src/actor.rs:109`), and the per-pose tables of section 2. With no sprite, the shared stand-in
  figure (section 3.1): the goldens, `tests/status.rs` and the bench all render with `set_actor(None)`
  (`crates/pixfx-core/src/render.rs:34-36`), so every effect must animate on the stand-in too.
- **Layers.** `draw` is called with `Layer::Back` then `Layer::Front` (`effect.rs:165-167`, `:681-695`). What lies
  on the floor or behind the body goes `Back`; the tinted body, what hangs on it and what flies off it go `Front`.
  Each effect's split is in its table below.
- **Determinism.** All maths through `libm` and `pixfx_draw::math` (`js_round` `math.rs:10`, `hash` `:139`, `Rng`
  `:158`, `tile_noise` `:265`, `loop_noise` `:293`); every random draw from `Rng::new(hash(&[seed, K_PURPOSE, item,
  frame]))` as `electrified.rs:399` does; no `HashMap` iteration. The golden check passes natively and under
  `wasm-pack test --node` (`tests/determinism.rs:68-71`).
- **Palette.** A default `[Rgba; N]` dark to light that `opts.palette_or(&PALETTE)` (`effect.rs:412-414`) resolves,
  so one preset word (`crates/pixfx-draw/src/palette.rs:23`, `fire` `:25`, `ice` `:31`, `poison` `:37`) recolours it.
- **Options.** Each `NumOpt` with `.within(lo, hi)` (`effect.rs:262-290`); unknown keys are refused by
  `check_options` (`effect.rs:380`), so the site sliders (`site/assets/effect.js` `OWN`, as `:71-76`) use exactly
  these names.
- **Bench budget**, 50 instances for 10 s at 60 Hz (`crates/pixfx-effects/examples/bench.rs:137-139`), release,
  each effect built alone: the ceiling is `feedback/electrified`'s measured cost, **1.34 ms draw and 0.66 ms rgba per
  display frame** (research 3.4: `104×112 | 8050 | 1.34 | 0.66`); the target is `feedback/soaked`'s 0.79 / 0.40. A
  per-effect ceiling is in 1.2. The merge request quotes the bench table.
- **Tests to pass**, the commands in 1.5: the crate's generic tests under the effect's feature (`tests/crop.rs`,
  `tests/live_frames.rs`, the bench's own unit tests), the per-effect sprite test (section 1.2, one file per effect
  with `required-features`, as `Cargo.toml:57-59` declares `body_burst`), the ignored golden test after blessing,
  `tests/features.rs` and `tests/status.rs` with `all`, and the wasm golden check once at the end.

### 1.2 Per effect

Ranges are inclusive. "Beat" names the frame index `k` within the loop (0-based) where something happens.

#### `feedback/burning`

| Criterion | Value |
|---|---|
| id, feature, file | `feedback/burning`, `feedback-burning`, `crates/pixfx-effects/src/feedback/burning.rs` |
| looping, duration, fps, frames | true, 1.0 s, 16, N = 16; a tick every 8 frames (two ticks per loop) |
| canvas, origin | (64, 64), (32, 55) |
| tags | `feedback, fire, burn, status, dot, character, loop` |
| options | `spots` 4 within 2..4 (hot spots that carry flames); `sparks` 1.0 within 0..2 (ember count scale); `smoke` 1.0 within 0..1.5 (smoke count scale); `beat` 1.0 within 0..1 (1: the tick flash, flare and spark burst; 0: a steady burn) |
| palette | 9 steps (the source's, `docs/research-status-sources/pixelfx-status-burning.js:20`): `4a1712 8a1f17 c9281f f2561d ff8a2a ffc23d ffe26b fff3a6 ffffff`; preset `fire` fits it through `palette_or` |
| layers | Back: the warm flicker on the floor (light only), the smoke. Front: the body flash and orange blend on the tick, the scorch patches, the tongues, the embers, the burst sparks, the spot glows |
| cues | none |
| bench ceiling | draw <= 1.34 ms, rgba <= 0.66 ms (the costliest of the four: tongues are drawn in nested layers and the smoke is translucent discs) |
| sprite test | `crates/pixfx-effects/tests/burning.rs`, `required-features = ["feedback-burning"]`, on the 16 x 32 rectangle sprite of section 1.6: (b1) frame 0, Front: every body cell is painted and none holds the sprite's own colours; (b2) frame 0, Front: at least one painted cell lies above the sprite's top row (y < -32) within x -8..7 (a flame standing on the crown); (b3) frame 4, Front: at least 25 % of the body cells are unpainted (no flash, no blend between ticks); (b4) no frame's Front holds the sprite's own colours |
| gif check | flames stand on the hair, a shoulder and the two flanks and lean into the wind; the body goes white for one frame then orange on frames 8 and 0; two scorch patches per tick darken the clothes and fade before the next; embers drift up and left; thin smoke behind the body |

#### `feedback/frozen`

| Criterion | Value |
|---|---|
| id, feature, file | `feedback/frozen`, `feedback-frozen`, `crates/pixfx-effects/src/feedback/frozen.rs` |
| looping, duration, fps, frames | true, 2.0 s, 16, N = 32; a shiver every 16 frames (two per loop) |
| canvas, origin | (64, 64), (32, 55) |
| tags | `feedback, ice, freeze, frozen, cold, status, character, loop` |
| options | `ferns` 6 within 0..8 (frost ferns creeping in from the sides); `icicles` 6 within 0..6; `snow` 1.0 within 0..1.5 (falling flakes scale); `shiver` 1.0 within 0..1 (1: the overlay jolts and the ferns crack off each shiver; 0: a still rime) |
| palette | 8 steps (`pixelfx-status-frozen.js:15`): `151a3d 212f6b 2c58aa 3a8bd8 62c8f4 a6ecff dcfbff ffffff`; preset `ice` fits |
| layers | Back: the cold floor light, the frost crust underfoot. Front: the chilled body, the ferns, the snow caps, the icicles and their drops, the breath, the falling and flung flakes, the sparkle, the body glows |
| cues | none |
| bench ceiling | draw <= 1.0 ms, rgba <= 0.66 ms (the body tint is a per-pose table of 500-odd `px` calls) |
| sprite test | `tests/frozen.rs`, on the sprite with arms of section 1.6: (f1) every frame, Front: every body cell is painted (the chilled body covers the sprite); (f2) frame 0, Front: cells (-5, -17) and (4, -17) are painted (an icicle under each arm); (f3) frame 8, Front: at least 3 cells strictly inside the body (not on its edge) are painted pure white (fern tips); (f4) frame 0, Back: at least 20 painted cells on row y = 0 (the frost crust) |
| gif check | the body reads blue-grey with a lit rim; ferns creep in from the flanks and crack off on each shiver; icicles grow under the chin, hair and hands, bead and drip; snow sits on the head and shoulders; breath puffs forward between shivers; frost spreads and shrinks underfoot |

#### `feedback/poisoned`

| Criterion | Value |
|---|---|
| id, feature, file | `feedback/poisoned`, `feedback-poisoned`, `crates/pixfx-effects/src/feedback/poisoned.rs` |
| looping, duration, fps, frames | true, 1.5 s, 16, N = 24; a tick every 12 frames (two per loop) |
| canvas, origin | (64, 64), (32, 55) |
| tags | `feedback, poison, toxic, sick, dot, status, character, loop` |
| options | `veins` 4 within 0..6 (vein walks from the wound); `blisters` 3 within 0..3 (per tick); `sweat` 3 within 0..4 (sweat sources); `ring` 1.0 within 0..1 (1: a ring pulses out on the floor each tick; 0: none); and one `ColorOpt` `veins_color` default `2c1a3a` (the source's `veins`, `pixelfx-status-poisoned.js:15`; `effect.rs:299-312`, read with `opts.color`, `:427`) |
| palette | 8 steps (`pixelfx-status-poisoned.js:15`): `0e1d14 173320 23552e 318232 55b92f 8eea3f cfff63 f6ffc2`; preset `poison` fits |
| layers | Back: the far half of the floor ring, the sick floor light. Front: the tinted body with its green waves, the veins and the wound, the blisters, the sweat and its hiss, the near half of the ring, the body glow |
| cues | none |
| bench ceiling | draw <= 1.0 ms, rgba <= 0.66 ms |
| sprite test | `tests/poisoned.rs`, on the sprite with arms: (p1) every frame, Front: every body cell is painted; (p2) frame 0, Front: cell (2, -13) (the wound: the right edge of the row at 60 % height, one in) is painted `P[6]` (`f6ffc2`-ish after `fit`: the palette's seventh step); (p3) frame 6, Front: at least 4 cells are painted exactly the `veins_color` (the tint never produces it); (p4) frame 2, Back: at least 6 painted colour cells with y < 0 and abs(x) <= 16, and none with abs(x) > 16 (the far half of the ring, radius 13.6 at that frame) |
| gif check | green rolls up the body in two waves per loop; veins creep out of the wound on the forearm and draw back; three blisters swell on each tick and pop at frame 7 of it; sweat beads at the brow and hands, falls and hisses on the floor; a ring pulses out across the floor on the tick |

#### `feedback/petrified`

| Criterion | Value |
|---|---|
| id, feature, file | `feedback/petrified`, `feedback-petrified`, `crates/pixfx-effects/src/feedback/petrified.rs` |
| looping, duration, fps, frames | true, 1.75 s, 16, N = 28; `loop_from() = Some(0.75)`: frames 0..11 play once (the stone creeps up over 0..7, sets at 8, the dust skirt rolls 8..11), frames 12..27 loop (the held statue) |
| canvas, origin | (64, 64), (32, 55) |
| tags | `feedback, stone, petrify, petrified, status, character, loop` |
| options | `cracks` 7 within 0..10 (crack walks through the statue); `grit` 1.0 within 0..2 (pebbles and motes per loop, and grit off the creeping front); `mottle` 1.0 within 0..1.5 (stone grain strength); `creep` 1.0 within 0..1 (1: the opening plays; 0: the statue is set from frame 0, for a game that applies the ailment mid-animation or restores a saved state) |
| palette | 8 greys (`pixelfx-status-petrified.js:18`): `16141c 2a2733 433f4d 5f5a6a 7d7889 a29dab cdc8d2 f0edf3`; no preset is grey (`palette.rs:23-84`), `pixfx_draw::color::pal::STONE` (`crates/pixfx-draw/src/color.rs:335`) is the engine's own stone ramp a caller may pass |
| layers | Back: the far dust lobes of the set (frames 8..11), the settling grit. Front: the stone body and its crackling front, the grit off the front, the cracks, the glint, the pebble, the dust mote, the near dust lobes |
| cues | none (the set at frame 8 would deserve encase's shake and flash, `encase.rs:120-143`, but a loop's cues repeat every `duration` even with `loop_from`, `effect.rs:140-141`: the game times its own) |
| bench ceiling | draw <= 0.6 ms, rgba <= 0.66 ms (the statue is a per-pose colour table; the loop draws a few extra pixels) |
| sprite test | `tests/petrified.rs`, on the sprite with arms: (q1) frame 0, Front: no painted cell above y = -4 (the stone has only begun at the feet); (q2) frame 12, Front: every body cell is painted and none holds the sprite's own colours (the statue covers it); (q3) `fx.loop_from() == Some(0.75)` and `fx.meta().frames() == 28`, and frames 12 and 18 differ (the glint); (q4) with `creep = 0`, frame 0, Front: every body cell is painted |
| gif check | grey climbs the body behind a white, flickering edge that spits grit; at the set the statue flashes pale and a dust skirt rolls out at the feet; then it holds: hairline cracks, a glint running along one, a pebble dropping off a hand or the chin, a mote of dust |

### 1.3 No cues, and why

The four fire no cues (no `cues` impl, no `FEEL` option), as `electrified`, `soaked` and `cloak-shimmer` do not
(research 1.6). `cues_between` repeats a loop's cues every `duration` (`crates/pixfx-core/src/cue.rs:82-86`), so a
`Shake` or `Flash` on the tick would shake the game twice a second for the whole ailment; `loop_from` does not
exempt the opening (`effect.rs:140-141`). The game owns the feel of a damage tick. The site stages each effect's
beat with `impact` in `site/assets/effect-roles.js` (section 5, S5), as it does for electrified (`:60`).

### 1.4 The sprite stays where it is

pixelfx's `burning`, `frozen` and `poisoned` draw the whole body themselves and hide the stand-in (`actorAlpha`,
`pixelfx-status-burning.js:38`, `-frozen.js:27`, `-poisoned.js:27`); frozen shakes it by a pixel and poisoned sways
it row by row. In pixfx the sprite stays on screen under the effect (only `replaces_actor`, `effect.rs:131`, hides
it, and a status never sets it), so the body is drawn over the sprite on `Layer::Front`, opaque, in the sprite's own
colours tinted (`Actor::opaque` gives the colour per cell, as `power_up.rs:746` reads it). A shift of that overlay
would leave a strip of the sprite showing on one side, so there is no shiver and no sway: frozen's shiver moves
only its overlay (ferns, snow, icicles) and poisoned's wobble is dropped; its motion is the green wave. A body that
really shakes or sways is a feel rule in `pixfx-core` (`Squash`, `crates/pixfx-core/src/squash.rs`), outside this
design.

### 1.5 The commands

From the repo root, `export PATH="$HOME/.cargo/bin:$PATH"`. In zsh write the feature flags out (a `$F` variable does
not split). `--offline` because the lockfile is complete; drop it if a build wants the network.

```sh
# one effect alone (replace burning)
cargo fmt -p pixfx-effects
cargo clippy --offline -p pixfx-effects --no-default-features --features feedback-burning --all-targets -- -D warnings
cargo test --offline -p pixfx-effects --no-default-features --features feedback-burning --all-targets
# look before blessing: the gif on the idle fighter, and alone
cargo run --offline -p pixfx-export --example gif -- feedback/burning --scale 4 \
    --actor crates/pixfx-pixel/tests/fixtures/thunder-idle.png --show-actor
cargo run --offline -p pixfx-export --example gif -- feedback/burning --scale 4 --out target/out/alone
PIXFX_BLESS=1 cargo test --offline -p pixfx-effects --no-default-features --features feedback-burning --test determinism -- --ignored
cargo test --offline -p pixfx-effects --no-default-features --features feedback-burning --test determinism -- --ignored
cargo run --release --offline -p pixfx-effects --no-default-features --features feedback-burning --example bench
# with everything (S5, and after any change to pixfx-core or feedback/status.rs)
cargo test --offline -p pixfx-effects --all-targets
cargo test --offline -p pixfx-effects --test features --test status
cargo test --offline -p pixfx-effects --test determinism -- --ignored
wasm-pack test --node crates/pixfx-effects --test determinism -- --include-ignored
```

The gif script's flags are at `crates/pixfx-export/examples/gif.rs:1-13`; `--actor` stands the PNG's bottom row
on the origin, and the fixture's lowest opaque row is row 61 of 64, so the fighter stands 2 px above the origin in
that gif (its pool and frost sit 2 px under the soles); the scene places a sprite by its feet, so games are not
affected. `scale_board` (`crates/pixfx-export/examples/scale_board.rs:1-11`) lays the busiest frame on the fighter
to check the ruler.

### 1.6 The synthetic sprites of the sprite tests

Built as `tests/body_burst.rs:12-21` builds its 12 x 20 sprite (`Actor::new(x, y, w, h, pixels)`, `RED` over `TEAL`,
luma 0.32 and 0.46) and rendered with `render_frames_with_actor` (`render.rs:40`), reading pixels back as
`tests/body_burst.rs` and the slash chain's `tests/sword_slash.rs` do.

- **Rectangle** (burning): 16 x 32 standing on the origin, x -8..7, y -32..-1, top half `RED`, bottom half `TEAL`.
  Both lumas fall in burning's scorch band (0.3..0.6).
- **Sprite with arms** (frozen, poisoned, petrified): a trunk x -4..3 for y -24..-1, a head x -3..2 for y -32..-25,
  and two arms x -8..-5 and 4..7 for y -24..-18, so there is empty space under each arm (hang cells with 17 clear
  rows below), a right edge at 60 % height (row -13) at x = 3, and up cells on the head and the arms' tops. Top
  half `RED`, bottom half `TEAL`.

Each assertion in 1.2 follows from the storyboard by construction on these sprites; the reasoning is in section 2
under "Why the sprite test holds". An assertion that turns out false in code is a design bug: fix the design (or
the test's sprite) and say so in the commit, never loosen a threshold until it passes.

---

## 2. Storyboards

Conventions: `k` is the frame index in the loop (`k = js_round(t * 16) mod N`); `ph` is seconds since the last
beat (`(k mod BEAT) / 16`); `Body` is the per-pose read of section 3.1; `C[i]` is the resolved palette step `i`;
sizes are given against the ruler with `Ruler::size(part, ratio)` (`ruler.rs:68-75`; head 7, hand 2, limb 3, eye
1 x 2, width 20, height 40). The source body is about 48 rows tall on a 72-wide canvas (research 5.3); ours is 40
rows, so a source size in pixels is restated as a ratio of a body part, not copied.

The source helpers (`docs/research-status-sources/pixelfx-status-_lib.js`) map to pixfx this way, for every effect:

| pixelfx | pixfx | Note |
|---|---|---|
| `shape(a)` (`_lib.js:20-57`): `on`, `body`, `up`, `side`, `outline`, `top`, `rows` | `Silhouette` (`covers` `silhouette.rs:118`, `cells` `:126`, `edge` `:136`, `ring` `:159`, `bounds` `:189`) and `Silhouette::open(dx, dy)` (section 3.2) for `up` (0, -1), `side` (-1/1, 0), hang (0, 1); `rows` from `Body` | built in `set_actor` |
| `colorAt(a, x, y)` (`_lib.js:60`), mid grey `6b6b6b` with no pixels | `Body::cells` (colour and luma per covered cell) from `Actor::opaque()` (`actor.rs:109`); the stand-in is that grey | |
| `tnoise(t, D, f, seed, off)` (`_lib.js:70-73`): 1D value noise looping every `D` s | `tile_noise((t / D) * P + off, 0.0, P, 0.0, seed)` with `P = max(2, js_round(f * D))` (`math.rs:265`): the same lattice, a different hash, so the loop closes the same way | pixels are not bit-identical to pixelfx; nothing requires that |
| `travel(v, g, d, a)` (`_lib.js:76`) | `pixfx_core::particles::fly(x, y, vx, vy, t, g, drag)` (`crates/pixfx-core/src/particles.rs:7-14`): the same closed form | |
| `cycle(t, D, n, seed, life, f)` (core) | `pixfx_core::particles::cycle` (`particles.rs:19-36`), the port of it | seamless particle loops |
| `rand`, `Rng`, `clamp`, `smooth`, `mix`, `luma`, `fit`, `WHITE` | `math::rand` (`:152`), `Rng` (`:158`), `clamp` (`:75`), `smooth` (`:112`), `color::mix` (`color.rs:101`), `luma` (`:114`), `fit` (`:130`), `WHITE` (`:52`) | |
| `p.px`, `p.dot`, `p.blend`, `p.disc`, `p.circle`, `p.line`, `p.shade`, `p.glow`, `p.light` | `Painter::px` (`painter.rs:391`), `dot` (`:397`), `blend` (`:317`), `disc` (`:550`), `circle` (`:566`), `line` (`:480`), `shade` (`:454`, `ShadeOut` `:76-81`), `glow` (`:729`), `light` (`:620`) | |
| `sparkle` (core) | `shapes::flash_star` (`crates/pixfx-draw/src/shapes.rs:23`) | frozen only |
| `band` (core, poisoned's floor ring) | `Painter::each` (`painter.rs:407`) over the ring's box: a cell is on the band when its normalised ellipse radius is within half the width of 1, the half picked by the sign of its angle's sine | poisoned only |
| `tongue`, `smoke`, `ember` (`_lib.js:85-137`, Sprite Fusion's fire) | private functions in `burning.rs`, ported 1:1 | burning only; lifted to `pixfx_draw::shapes` only when a second effect needs them |
| `eyes(a)` (`_lib.js:219-261`) | not ported: the mouth, the brow and the eye sweat are placed from the bounds (2.2, 2.3) | |
| `Particles`, `Bits`, `Lights` (petrified's simulation) | closed-form particles by frame (`fly`, `cycle`) and `Painter::glow`; no `update` state | petrified only |

### 2.1 `feedback/burning`

**Idea, role.** The character is on fire: small flames lick from four hot spots on its outline, embers stream up and
away, thin smoke drifts behind it; twice a loop a tick flares every flame, the body flashes white then orange, and
two scorch patches char the clothes. Role: `buff` on the site (a status on the caster, section 5 S5). Source:
`docs/research-status-sources/pixelfx-status-burning.js` (MIT, Copyright (c) 2026 Patch, Copyright (c) 2026 Hugo
Duprez; the tongues, embers and smoke adapt Sprite Fusion's fire, lines 1-3: the module doc keeps both credit lines,
as `crates/pixfx-effects/src/disappear/incinerate.rs:4-5` does).

**Moments** (N = 16, a tick at k = 0 and k = 8; `ph = (k mod 8) / 16`):

| Moment | Frames | Seconds | What shows |
|---|---|---|---|
| TICK | k mod 8 = 0 | 0.0625 | every body cell white (`WHITE` over the sprite); flames at full flare (x 2.6 height); a burst of 4 sparks per spot; the scorch patches of this tick appear small (40 % radius) |
| FLASH OFF | k mod 8 = 1..3 | 0.19 | the body blended orange `C[4]` at alpha `0.55 * (1 - (ph - 0.034) / 0.186)` with `Painter::blend` (translucent, as the source's `p.blend`); flames shrinking (`flare = exp(-ph / 0.12)`); scorch grows to full radius by frame 2 |
| BURN | k mod 8 = 4..7 | 0.25 | the body bare (the sprite shows); flames at rest, swaying in the wind; scorch fades `1 - (k mod 8 - 3) / 5` (0.8, 0.6, 0.4, 0.2) and is gone at the next tick, when the other patch set starts |
| always | | | embers `cycle(t, 1.0, 20, seed + 6, life 0.5..0.9)` rising and drifting left; smoke `cycle(t, 1.0, 16, seed + 4, life 0.8..1.2)` behind the body; a warm glow at each spot and a flicker on the floor |

The source ticks every 0.6 s on a 1.2 s loop at 30 fps (`burning.js:14-16, 34`); at 16 fps that is 9.6 frames, so
the tick is 8 frames (0.5 s) on a 1.0 s loop and the scorch windows are restated in frames (above) so a patch set
has faded before the next tick replaces it. Everything else keeps its seconds.

**Parts.**

- *Hot spots* (4; `spots` option takes the first 2..4 of the list): the hair tip, a shoulder, the left flank at 62 %
  height, the right flank at 60 % height (`burning.js:51-57`). Each carries 3, 2, 2, 2 tongues with `dx = (j - (n -
  1) / 2) * 1.6 + Rng(-0.4..0.4)` and a height `h = base * Rng(0.8..1.2)`, base `size(head, 0.65) = 5` for the hair
  and `size(head, 0.5) = 4` for the others (the source's 4.5 and 3.5..4 on a 48-row body).
- *Tongue* (`_lib.js:85-98`, kept 1:1): four nested layers `LAYERS` (`_lib.js:79`), rows `r = 0..=H` from the root
  up, half-width `w * lw * (1 - u)^0.85 * min(1, (r + 1.5) / 3.5) + tnoise * 0.5`, the tip swept `sx` sideways
  with `sx = WIND * 0.08 * h / 4 + tnoise(..) * 0.8` (`WIND = -14`); root half-width `1.1 + 0.3 * flare`, i.e. about
  `size(limb, 0.4) = 1` plus the flare. The root sits 2 rows inside the body (`sp.y = cell y + 1.5`, `burning.js:58`),
  so the flame looks rooted.
- *Scorch* (2 patches per tick, 2 ticks: 4 fixed patches per pose, `burning.js:60-68`): centre a body cell within
  9 px (`size(head, 1.3)`) of the patch's spot, below 42 % height, luma 0.3..0.6 (else any cell below 45 %); radius
  `Rng(3.2..4.4)`, about `size(hand, 2.0) = 4`. Drawn over the body cells within `r` (with a `rand` jitter of 0.9),
  the rim in `C[3]`/`C[4]` flickering, the inside in the char colours `CH` (`120c0a 2a1a14 4a2c1c`, `burning.js:21`)
  at alpha `fade * 1.1` through `Painter::dot`.
- *Embers* (`_lib.js:131-137`): 1 px, hot `C[1]` blinking to `C[3]`, cooling to `C[5]`, a 3 px glow; count `20 *
  sparks`.
- *Burst sparks*: on `ph < 0.3` (k mod 8 in 0..4), 4 per spot, thrown up in a 2.2 rad fan at 50..100 px/s with
  gravity `60 * ph^2`, `C[1]` then `C[3]`, a `light` of 0.4 fading.
- *Smoke* (`_lib.js:119-128`, Back): three translucent discs per puff via `Painter::blend_px` (`painter.rs:292`),
  size `Rng(1.1..1.9)` (`size(eye.1, 0.75) = 2`), colour `mix(5b5566, C[6], 0.2)`, count `16 * smoke`, rising
  `fly(.., -18, 6 ..)` and drifting with the wind.
- *Light*: a spot glow `glow(sp.x, sp.y - 2, 9 + 6 * flare, C[4], 0.22 + 0.35 * flare)`; the floor flicker
  `floor_light(p, 26, 4.5, mix(C[3], C[4], 0.5), (0.16 + 0.22 * flare) * flick)` (Back; section 3.1).

**Exact in code.** 4 spots in that order; 3 + 2 + 2 + 2 tongues; 2 scorch patches per tick alternating between 2 sets
(`tick = (k / 8) mod 2`); 20 embers, 16 smoke puffs; 4 burst sparks per spot; the tick at k = 0 and 8; the wind
blows to -x (the scene mirrors the effect with the sprite, so a sprite facing left burns leftward).

**What the game sets.** The sprite it wraps (`fx.wrap(actorOf(sprite))` / `FxScene::set_actor`); `palette`;
`spots`, `sparks`, `smoke`, `beat`; `seed`.

**Look.** Palette above (preset `fire`). Opaque strokes, light in the light layer; no `halo`.

**Scale.** A tongue 4..5 px tall at rest (`size(head, 0.5..0.65)`), 10..13 at the tick; a tongue root 1 px half-wide
(`size(limb, 0.4)`); an ember 1 px (`size(eye.0, 1.0)`); a smoke puff 2 px (`size(eye.1, 0.75)`); a scorch patch 4
px radius (`size(hand, 2.0)`); the floor flicker 26 half-wide (`size(width, 1.3)`).

**How it reads the sprite** (`set_actor`, once per pose; `Body::read` of section 3.1 gives `sil`, `cells`, `bounds`,
`rows`):

- `up = sil.open(0, -1)` (covered cells with nothing above); `H = bounds.h`, `W = bounds.w`.
- hair: the up cell with the smallest y (then x) among those with `x < bounds.x + 0.42 W`; shoulder: the up cell with
  the smallest y among those with `x > bounds.x + W / 2` and `0.4 H < y - bounds.y < 0.62 H`, else `rows[0.5 H].right`;
  flanks: `rows[0.62 H].left` and `rows[0.6 H].right` (`burning.js:47-57`).
- the scorch candidates as above, from `cells` (luma); the patch centre picked with `Rng(seed * 13 + 5)`.
- With no sprite: the stand-in figure (section 3.1) read the same way.

Why the sprite test holds (rectangle, 1.6): (b1) the tick paints `WHITE` over every body cell before anything else,
nothing erases, and no later stroke uses `RED` or `TEAL`; (b2) the hair spot is the top-left up cell (-8, -32), its
tongues' root row is -30 and their height at the tick is at least 6 rows (`5 * 0.8 * 0.6 * 2.6`, the noise at its
lowest), with a half-width over 0.3 from the third row up, so rows -33..-36 are painted in the columns -10..-6; (b3) at k = 4 there is no flash and no blend (ph = 0.25 > 0.22), so only the two
scorch patches (at most 2 x 61 cells) and the tongues at rest (`flare = 0.12`, about 5 rows by 5 columns each, half
of them outside the body) touch the body: under 260 of 512 cells; (b4) as (b1).

**The port: kept and changed.**

| Kept 1:1 | Changed, and why |
|---|---|
| the four spots and how they are found; the tongue, ember and smoke recipes; the flash-then-orange tick; the scorch rule (candidates, radius, rim, char); the burst fan; the wind; the smoke on the back layer | the clock: 16 fps, 1.0 s, tick at 8 frames (the source's 0.6 s tick is 9.6 frames); scorch windows in frames |
| the 9-step palette and the char colours | the body flash is drawn over the sprite instead of replacing it (1.4); `actorAlpha` has no equivalent |
| `travel`, `cycle`, `tnoise` as `fly`, `cycle`, `tile_noise` | sizes restated against the ruler (a 48-row source body is 40 rows here) |
| | the `char` and `smoke` colour options of the source are constants here (the palette is the one ramp a game recolours; a second ramp option would be the first in the crate) |

### 2.2 `feedback/frozen`

**Idea, role.** Chilled to the bone: the body goes blue-grey with a lit rim, frost ferns creep in from its flanks,
icicles hang from the chin, hair and hands and drip, snow sits on the head and shoulders, breath steams between
shivers, frost spreads underfoot; every second a shiver cracks the ferns off and flings flakes. Role: `buff`.
Source: `docs/research-status-sources/pixelfx-status-frozen.js` (MIT, Patch / Hugo Duprez), a loop as ours is: no
opening is needed, so no `loop_from` (compare petrified, 2.4).

**Moments** (N = 32, a shiver at k = 0 and 16; `ph = (k mod 16) / 16`):

| Moment | Frames | Seconds | What shows |
|---|---|---|---|
| SHIVER | k mod 16 = 0..3 | 0.25 | the overlay (ferns, snow, icicles, breath) jolts `dx = -1, +1, -1, +1` (the source's `shake`, `frozen.js:28`, on the overlay only: 1.4); the ferns are mostly gone (`grow = 0.2`); 8 flakes fly off the body (k mod 16 = 0..7); a 4-point sparkle on the head shrinks over frames 0..3 |
| CREEP | k mod 16 = 4..15 | 0.75 | ferns creep back in: `grow = 0.2 + 0.65 * smooth(0.05, 0.95, ph)`, the newest pixels white; breath puffs forward from the mouth (k mod 16 = 6..15, four discs) |
| always | | | the chilled body; snow caps; icicles growing, beading and dripping (two periods, 16 and 32 frames); falling flakes `cycle(t, 2.0, 9, seed + 2, life 1.2..1.8)`; the cold glow surging with the shiver; frost underfoot breathing `reach = 13 + 3 * sin(t / 2 * TAU)` |

**Parts.**

- *Chilled body* (Front, per-pose table): every body cell drawn opaque `mix(c, ice, 0.38)` with `ice = C[clamp(1 +
  round(luma * 5.6), 1, 7)]`; a rim cell (open left, up or right) whose luma is under 0.3 is `C[5]` in the top half or
  when open left, else `C[3]` (`frozen.js:110-115`). Baked in `set_actor` as `Vec<(x, y, Rgba)>`.
- *Ferns* (6, `ferns` option 0..8): each a stem walking in from a side cell between 20 % and 85 % height, 9 steps of
  1.1 px (about `size(head, 1.4) = 10` reach), fronds of 1..3 px branching off every second step; each pixel keyed by
  when it grows (`frozen.js:38-59`). Drawn for `key < grow`: white when `key > grow - 0.1`, else `C[6]`/`C[5]` by
  `rand`.
- *Icicles* (6, `icicles` 0..6): under hang cells (covered above, open below with at least 4 clear rows), one per
  region (two under the top 36 %, two in 36..50 %, two below 50 %, left and right), length `min(clear - 1, Rng(3..5.5))`
  (`size(head, 0.5..0.8)` = 4..6), a `C[4]` shadow column beside the top half; a drop that beads at `u in 0.7..0.86`
  and falls at 760 px/s² to the floor (`frozen.js:61-77, 124-139`). 2 px wide with its shadow (`size(hand, 1.0)`).
- *Snow caps*: on every up cell above 60 % height, 1 px `C[7]`, a second px for 45 % of them (`frozen.js:79, 119-123`).
- *Breath*: four discs from the mouth, `rad = 0.8 + 2.4 * sqrt(u)` (`size(eye.1, 1.6) = 3` at most), `C[7]` at alpha
  `0.5 * (1 - u)` with a white highlight, drifting forward 16 px/s with drag (`frozen.js:140-148`).
- *Flakes*: 9 cycled, falling from 13 rows above the crown (`a.y - 16` scaled by 40/48) at 14 px/s with a sine
  drift, dithered `dot`; stop at 2 rows under the crown's row (`frozen.js:149-154`).
- *Shiver flakes*: 8, thrown off at `ph < 0.5` (`frozen.js:155-159`); *sparkle*: `flash_star` at `(bounds.x + 0.3 W,
  bounds.y + 4)`, radius `3 * (1 - ph / 0.25)`.
- *Frost underfoot* (Back, `frozen.js:86-96`): on row 0 a crust from `-reach` to `reach` (white where a `rand` twinkle
  says, `C[5]` at the ends, `C[6]` inside), a `C[6]` px on row -1 where `rand < 0.3 k`, and 1..3 dithered rows of
  `C[5]`/`C[4]` below it; `reach = 13 + 3 * sin(t / D * TAU)` (`size(width, 0.65) = 13`, `size(hand, 1.5) = 3`).
- *Light*: `floor_light(p, 24, 4, C[4], 0.24)` (Back); `glow(0, bounds.y + 0.45 H, 30, C[4], 0.2 + 0.12 * exp(-ph /
  0.2))` and `glow(0, bounds.y + 2, 16, C[5], 0.1)` (Front); an icicle glow `glow(x, y0 + 0.6 L, 4, C[5], 0.18)`.

**Exact in code.** 6 ferns, 6 icicles, 9 falling flakes, 8 shiver flakes, 4 breath discs; the shiver at k = 0 and
16; `dx` = -1, 1, -1, 1 on the shiver's four frames; icicle periods alternate 32 and 16 frames (`i mod 2`); the
sparkle on frames 0..3.

**What the game sets.** The sprite; `palette`; `ferns`, `icicles`, `snow`, `shiver`; `seed`.

**Look.** Palette above (preset `ice`). The rim light is painted (the `C[5]`/`C[3]` rim cells), not blurred.

**Scale.** Fern reach 10 px (`size(head, 1.4)`), stroke 1 px; icicle 4..6 long (`size(head, 0.5..0.8)`), 2 wide with
its shadow (`size(hand, 1.0)`); snow 1..2 px; a flake 1 px; breath 3 px radius at most (`size(eye.1, 1.6)`); frost
reach 13 +- 3; floor light 24 half-wide (`size(width, 1.2)`).

**How it reads the sprite** (`set_actor`): `Body::read` gives `sil`, `cells` (colour, luma), `bounds`, `rows`; then
`side = sil.open(-1, 0)` and `sil.open(1, 0)` filtered to 20..85 % height, shuffled with `Rng(seed * 11 + 1)`, first
6 as fern roots; the fern walk uses `sil.covers`; hang cells from `sil.open(0, 1)` with `sil.covers(x, y - 1)`,
`clear(x, y)` counting uncovered cells below (up to 7), `y < 0.72 H`, `y > bounds.y + 4`; snow from `sil.open(0, -1)`
above 60 % height; the mouth at `(rows[0.3 H].right - 1, bounds.y + 0.3 H)` (the face's front when the sprite faces
+x; the scene mirrors a flipped sprite); the chilled colour per cell. With no sprite: the stand-in figure.

Why the sprite test holds (sprite with arms, 1.6): (f1) the chilled body paints every body cell opaque first, and
nothing erases; (f2) the hang cells with 4 or more clear rows are the arms' bottoms (y = -18, x -8..-5 and 4..7,
17 clear rows under them); they lie in the 36..50 % band (mask row 14 of 32), so the band's two icicles are the
right-most of the left arm, (-5, -18), and the left-most of the right arm, (4, -18); an icicle starts at `y + 1` and
is at least `0.55 * 3 = 1.65` px long, so (-5, -17) and (4, -17) are painted on every frame; (f3) at k = 8 `grow =
0.2 + 0.65 * smooth(0.05, 0.95, 0.5) = 0.525`, every fern draws its steps 0..4 and its step 4 is white (key 0.444 in
`(0.425, 0.525)`); a root on a flank walks inward at +-0.5 rad, so step 1 is already 1 px inside the body and every
later step too; the top-row flakes stop above the body, the shiver flakes and sparkle are off at ph = 0.5, snow and
icicles are outside the body, breath is at the right edge; a root on an arm's top row (2 of about 42 candidates)
may step off the arm and break at once, so at least 4 of the 6 ferns keep their step 4 and at least 4 white
interior cells survive (3 is asked);
(f4) at t = 0 `reach = 13` and every column with `abs(x + 0.5) <= 13` paints row 0: 26 cells.

**The port: kept and changed.**

| Kept 1:1 | Changed, and why |
|---|---|
| the chilled colour rule; the fern plan and its growth curve; the icicle regions, lengths, bead and drop; snow caps; breath; the flake cycles; the sparkle; the frost crust; the glows; the 8-step palette | 16 fps on 2.0 s (32 frames), shiver at 16 frames: all whole frames (the source is 30 fps) |
| | the shake moves the overlay only, never the body (1.4) |
| | no `eyes()`: the mouth sits at 30 % down the bounds on the front edge; `size`s against the ruler |
| | `sparkle` is `flash_star`; `tnoise` is `tile_noise` |

### 2.3 `feedback/poisoned`

**Idea, role.** Sick to the bone: green rolls up the body in waves, dark veins creep out of a wound on the forearm
and draw back, blisters swell and pop, toxic sweat beads and falls and hisses on the floor, a ring pulses out across
the floor on each tick. Role: `buff`. Source: `docs/research-status-sources/pixelfx-status-poisoned.js` (MIT, Patch
/ Hugo Duprez), a loop.

**Moments** (N = 24, a tick at k = 0 and 12; `ph = (k mod 12) / 16`; `D = 1.5`):

| Moment | Frames | Seconds | What shows |
|---|---|---|---|
| TICK | k mod 12 = 0..6 | 0.44 | this tick's 3 blisters swell out of the skin (`grow2 = clamp(ph / 0.16)`: 0, 0.39, 0.78, 1 over frames 0..3), shine, with a 4 px glow; the floor ring runs out (`kr = ph / 0.55`, frames 0..8) in two halves; the floor and body light surge `exp(-ph / 0.2)` |
| POP | k mod 12 = 7..8 | 0.125 | the blisters burst into 6 dithered specks flying out and up, a 6 px glow fading |
| REST | k mod 12 = 9..11 | 0.19 | nothing on the skin but the veins and the wave |
| always | | | the tinted body with two green waves per loop; veins growing `smooth(0, 0.55 D, t) * (1 - smooth(0.62 D, D, t)) * 1.15` (full by frame 13, gone by 24); sweat on its own 12-frame period per source (offset `i * 0.37` s); the wound bright `C[6]` with a `C[3]` cell beside it and a 6 px glow |

The source loops 1.6 s with a 0.8 s tick at 30 fps (`poisoned.js:10, 24`); 25.6 frames at 16 fps, so the loop is
1.5 s (24 frames) with the tick at 12 frames (0.75 s). The windows keep their seconds.

**Parts.**

- *Tinted body* (Front): per cell `wave = 0.5 + 0.5 * sin(TAU * ((my / H) * 1.6 + (t / D) * 2))` with `my` the row
  within the bounds; `sick = C[clamp(1 + round(luma * 6.2), 1, 7)]`; `k = 0.22 + 0.4 * wave^2`; colour `mix(c, wave >
  0.85 && luma > 0.4 ? mix(sick, C[6], 0.3) : sick, k)` (`poisoned.js:91-96`). `sick` and `luma` are baked per cell in
  `set_actor`; the wave is 40-odd row values per frame.
- *Wound*: `(rows[0.6 H].right - 1, bounds.y + 0.6 H)` (`poisoned.js:36`).
- *Veins* (4, `veins` 0..6): random walks from the wound, 11 steps of 1 px at a heading `PI + (i / 4 - 0.45) * 2.2`
  drifting `Rng(-0.5..0.5)` a step, a 16 % chance per step past the second to fork (depth < 2, half length); each
  pixel keyed `s / len` (`poisoned.js:37-50`). Drawn for `key < grow`, the newest 0.12 in `C[2]`, the rest in
  `veins_color`. Reach 11 px (`size(head, 1.6)`).
- *Blisters* (2 sets of 3, `blisters` 0..3): interior cells (covered left, right and above, `y > bounds.y + 4`),
  radius `Rng(1.1..1.8)` (`size(eye.0, 1..2)`): a `C[3]` ring, a `C[5]` disc, a white px (`poisoned.js:54-57,
  105-122`).
- *Sweat* (up to 4, `sweat` 0..4): two hands (the lowest-left and lowest-right cells of `open(0, 1)` between 50 %
  and 70 % height at the body's extreme columns), the brow `(rows[0.25 H].right - 1, bounds.y + 0.25 H)`; a bead
  gathers 0.35 s, runs 2 px, falls at 760 px/s² with a 3 px tail, hisses on the floor 0.4 s: a spreading `C[6]`/`C[4]`
  line, 3 specks, a `C[5]` vapour disc, a glow (`poisoned.js:123-148`).
- *Ring* (`ring` option; `poisoned.js:81, 151`): an ellipse `rx = 5 + 18 * sqrt(kr)` (`size(width, 0.25) + size(width,
  0.9)`, at most 23), `ry = 0.18 rx`, band width `max(1, 2.5 * (1 - kr))`, alpha `1 - kr`, colours `C[3..6]` by
  angle; the far half (`sin(angle) < 0`) on Back, the near half on Front.
- *Light*: `floor_light(p, 24, 4, C[4], 0.14 + 0.2 * exp(-ph / 0.2))` (Back); `glow(0, bounds.y + 0.45 H, 26, C[4],
  0.14 + 0.2 * exp(-ph / 0.18))` (Front).

**Exact in code.** 4 veins from one wound; 3 blisters per tick in 2 alternating sets; up to 4 sweat sources with
offsets `i * 0.37` s; the tick at k = 0 and 12; the pop at k mod 12 = 7..8; the ring over frames 0..8 of each tick.

**What the game sets.** The sprite; `palette`; `veins`, `blisters`, `sweat`, `ring`, `veins_color`; `seed`.

**Look.** Palette above (preset `poison`); the veins in their own dark violet (`shadow` preset's `2a163e` is the
nearest word if a game wants a preset).

**Scale.** Vein reach 11 px, 1 px wide; blister 1..2 px; sweat bead 1 px, drop with a 3 px tail; ring radius 23 at
most (`size(width, 1.15)`), band 1..2.5 px; floor light 24 half-wide.

**How it reads the sprite** (`set_actor`): `Body::read`; `rows[0.6 H]` for the wound; `sil.covers` for the vein walks
and the blister interior test; `sil.open(0, 1)` for the hands; `cells` for `sick`/`luma` per cell. With no sprite:
the stand-in figure.

Why the sprite test holds (sprite with arms): (p1) the tint paints every body cell opaque first; (p2) the row at
60 % height is mask row 19 (y = -13), below the arms, so its right edge is x = 3 and the wound is (2, -13), drawn
`C[6]` after the tint; a blister centre is a seeded pick among interior cells and could land there (about 1 in 300):
if it does with the default seed, the test uses another seed and says so; (p3) at k = 6 (`t = 0.375`) `grow = 0.43 *
1.15 = 0.497`, so each vein shows its steps 0..5 and all but its newest 0.12 are `veins_color`; the walks head left
from (2, -13) into a 24-row trunk, so at least two veins keep 5 steps; the tint never yields `2c1a3a` (it mixes the
sprite's colour with a green) and blisters cover at most 3 discs of radius 2; (p4) at k = 2 `kr = 0.227`, `rx =
13.6`, `ry = 2.45`, band 1.9: the far half is the arc with y < 0 from (-13.6, 0) to (13.6, 0), at least 20 cells;
the only other thing on Back is `floor_light`, which writes light, not colour (`shade` paints colour only when
`out.c` is set, `painter.rs:454-470`).

**The port: kept and changed.**

| Kept 1:1 | Changed, and why |
|---|---|
| the wave tint rule; the wound; the vein walks and their growth window; the blister sets, swell and pop; the sweat cycle and hiss; the ring and its two halves; the glows; the 8-step palette and `veins` colour | 16 fps on 1.5 s (24 frames), tick at 12 (the source's 1.6 s / 0.8 s is not whole at 16 fps) |
| | the sway and the tick wobble (`offsets`, `poisoned.js:71-75`) are dropped: the sprite cannot move (1.4) |
| | no `eyes()`: the brow sweat sits at 25 % down the bounds on the front edge |
| | `band` is drawn with `Painter::each`; sizes against the ruler |

### 2.4 `feedback/petrified`

**Idea, role.** Turned to stone: grey creeps up from the feet behind a crackling white edge that spits grit, sets
with a pale flash and a dust skirt at the feet, then holds as a statue: hairline cracks, a glint running along one,
a pebble dropping off, a mote of dust. Role: `buff` (a held status; the site's take stops it). Source:
`docs/research-status-sources/pixelfx-status-petrified.js` (MIT, Patch / Hugo Duprez; the creeping front, cracks
and burst adapt Sprite Fusion's ice freeze, lines 1-3: both credit lines in the module doc), a **one-shot** that
`feedback/encase` already ports with its crumble (`encase.rs:1-8`). A held ailment needs the first two of its three
acts (creep, hold) and never the third (shatter).

**Looping it: `loop_from` or a pure loop.** Two designs were compared:

| | A pure loop (held statue only) | `loop_from` (creep once, then the held loop): chosen |
|---|---|---|
| What the player sees | the character is grey from the first frame; the "turned to stone" beat is missing | the stone climbs the body once (0.5 s), sets, then holds |
| Game code | the game plays `encase` first for the climb, then this: two effects, timed by the game, and encase crumbles at its end (`encase.rs:437-501`) | one effect: play it, `stop()` it |
| Engine risk | none | `loop_from` is implemented and tested in core (`Clock::frame`, `effect.rs:526-547`, `crates/pixfx-core/tests/loop_from.rs:44-55`) and honoured by `tests/live_frames.rs:100`, but no shipped effect uses it: this is its first user, and the goldens pick frames 0, 1, n/4, n/2, n-1 (`tests/determinism.rs:28`), which cover the opening and the loop |
| Cues | none either way | none: a cue in the opening would repeat every `duration` (`effect.rs:140-141`) |
| Frames that must not leak | | nothing born in the opening may still be drawn at a loop frame: grit lives at most 4 frames and is born at k <= 7; the dust skirt ends at k = 11 |

`loop_from` is chosen: the opening is the readable beat of the ailment and a game should not have to stitch two
effects. The `creep` option (0) gives a game the pure-loop look when it needs the statue at once.

**Moments** (N = 28; opening k = 0..11, loop k = 12..27; `loop_from() = Some(0.75)`; `Clock::frame` maps a clock frame
`k >= 12` to `12 + (k - 12) mod 16`, `effect.rs:526-547`, and `Sim::seek` brings the state there, `:659`):

| Moment | Frames | Seconds | What shows |
|---|---|---|---|
| CREEP | 0..7 | 0.5 | `level(k) = 1.2 * smooth(0, 8, k) + 0.025 * sin(k / 8 * TAU * 3)`; `front(mx) = level + 0.06 * tile_fbm(mx * 0.4, 0.5, 0, 0, seed, 2)`; a body cell with `up = 1 - (y + 0.5 - bounds.y) / H` is stone when `front(x) - up >= 0`: the top 0.035 of it flickers `C[7]`/`C[6]` by `rand(x, y, k)`, the next 0.035 is `C[5]`, the rest its stone colour; a 5 px glow every second column along the front; 3 grit particles born at the front each frame, flying `side * 15..60, -30..-90` px/s for 2..4 frames (`C[4..6]`), with a 30 % chance of a dust px and a 25 % chance of a white mote |
| SET | 8 | | every body cell stone; a pale flash `glow(0, bounds.y + 0.5 H, 34, C[6], 0.35)`; the far dust lobes (Back) and near lobes (Front) open at the feet as `encase.rs:293-352` builds them, capped at 26 px reach (`size(width, 1.3)`) |
| SKIRT | 9..11 | 0.19 | the dust lobes roll out and hollow (ages 1..3 of encase's table, `encase.rs:330`), the flash fades over frames 8..9 |
| HOLD (loops) | 12..27 | 1.0 | the statue; its cracks fully drawn in `C[0]` with a `C[6]` px above each (the source's set look, `petrified.js:159-163`, without the growth: a held statue is already cracked); the cold light `glow(0, bounds.y + 0.45 H, 30, C[5], 0.12 + 0.06 * sin((k - 12) / 16 * TAU))` |
| GLINT | 18..19 | | the tip of crack 0 white for two frames, a 12 px `C[6]` glow at the crack origin fading (`petrified.js:164`) |
| PEBBLE | 20..25 | | one hang cell (open below) picked by `Rng(hash(seed, K_PEBBLE))` drops a 1 px `C[3]` pebble: `fly` from `(x + 0.5, y + 1)` with `vx = Rng(-8..8)`, `vy = 0`, gravity 760, until it meets y = -1, then it rests one frame |
| MOTE | 22..27 | | a dust px `C[4]` drifting up and sideways from the feet, dithered out |

**Parts.**

- *Stone colour per cell* (`petrified.js:42-48`, kept): `tone = clamp((luma - 0.06) / 0.78)`, `mottle = tile_fbm(x *
  0.32, y * 0.32, 0, 0, seed + 5, 2) * mottle_opt`, `i = 1 + round(tone * 4.6 + mottle * 1.1)`, `+1` when open left or
  up, `-1` when open right or down, `C[clamp(i, 1, 6)]`. Baked in `set_actor`.
- *Cracks* (7 walks, `cracks` 0..10; `petrified.js:50-63`, kept): from `(bounds.x + 0.55 W, bounds.y + 0.42 H)`, 14
  steps of `Rng(1.4..2.4)` px at a heading `i / 7 * TAU + Rng(-0.3..0.3)` drifting `Rng(-0.55..0.55)`, a 14 % chance
  to fork past the third point, at most 16 walks; stopped by `sil.covers`. Baked in `set_actor`. Reach 14 px
  (`size(head, 2.0)`), 1 px wide.
- *Grit, dust, mote* off the front: closed-form (`fly`) from their birth frame and cell, 1 px, 2 px for 30 % of the grit.
- *Dust skirt*: `encase.rs:293-352`'s lobes and ramps, reach capped at 26.
- *The held statue's loop*: the glint, the pebble, the mote above; nothing else moves (stone does not).

**Exact in code.** 7 crack walks from one origin; 3 grit per creep frame; the set at k = 8; the skirt over 8..11; the
glint on 18..19; one pebble per loop at 20..25; one mote at 22..27; the loop from k = 12.

**What the game sets.** The sprite; `palette`; `cracks`, `grit`, `mottle`, `creep`; `seed`.

**Look.** The 8 greys above; lit on the top-left (`+1` step), shadowed bottom-right (`-1`): the source's shading.

**Scale.** The front's wobble 0.06 of the body (2 px); grit 1..2 px (`size(eye.0, 1..2)`); a crack 1 px wide, 14 px
reach; the dust skirt 26 px reach (`size(width, 1.3)`), lobes 4..6 px radius as encase; a pebble 1 px; a mote 1 px.

**How it reads the sprite** (`set_actor`): `Body::read`; per cell the stone colour (luma and the four `sil.open`
tests, or `sil.covers` of the four neighbours), `up`; the crack walks through `sil.covers`; hang cells `sil.open(0,
1)` for the pebble. With no sprite: the stand-in figure.

Why the sprite test holds (sprite with arms): (q1) at k = 0 `level = 0`, so only cells with `up <= 0.06` (the
bottom two rows, y = -2..-1) can be stone, grit is born at the front (y about -1) and has not flown yet, and the
glow is light: no colour above y = -4; (q2) at k = 12 `level = 1.2 > up` for every cell, so every body cell is
stone, a grey from the palette; (q3) `loop_from` and `frames()` are constants, and frames 12 and 18 differ by the
glint's two white pixels; (q4) with `creep = 0` the opening frames draw the held statue (`level = 1.2`).

**The port: kept and changed.**

| Kept 1:1 | Changed, and why |
|---|---|
| `level`, `front`, the crackling edge and its grit; the stone colour rule; the crack walks; the set flash; the pebble; the 8-step palette | the shatter (`petrified.js:97-123`) and the pre-shatter shudder and crack growth (`:140, :156-163`) are not ported: a held ailment never breaks; `encase` keeps the crumble |
| | 16 fps; the creep is 8 frames (0.5 s, the source's 0.95 s felt long on a 40-row body and the dust skirt follows); `loop_from` 0.75 s; a 1 s loop of glint, pebble and mote so the goldens and `tests/status.rs` see frames that change |
| | particles are closed-form by frame (no `Particles`/`Bits`/`Lights` simulation, no `update`), so any frame draws alone and the loop closes |
| | the dust skirt reuses encase's lobes (`shapes::lobes`, `shapes.rs:108`) and is capped to the canvas; cues dropped (1.3) |

---

## 3. Shared code

Rule (root `AGENTS.md`, "Rules"): a shared primitive earns its place only when 2+ callers already exist and no
benchmark regresses; push it to the lowest layer every caller reaches; never couple effect A to effect B. The
smallest shared surface that stops the same rule being written four times:

### 3.1 `crates/pixfx-effects/src/feedback/status.rs` (category level, `pub(crate)`)

Declared in `src/feedback/mod.rs` as `#[cfg(any(feature = "feedback-burning", feature = "feedback-frozen",
feature = "feedback-petrified", feature = "feedback-poisoned"))] mod status;`, the shape `src/lib.rs:9-10` gives
`dither.rs` (compiled when any of its users is). It holds:

- `pub(crate) struct Body { pub sil: Silhouette, pub cells: Vec<Cell>, pub bounds: Area, pub rows: Vec<(i32, i32)> }`
  with `Cell { x: i32, y: i32, c: Rgba, l: f64 }` (colour and luma): the sprite read once. `Body::read(actor:
  Option<&Actor>, area: Area) -> Body`: with a sprite covering the area, `Silhouette::of` and `Actor::opaque`; else
  the stand-in. `rows[j]` is `(left, right)` of row `bounds.y + j`, `(0, -1)` for an empty row. Callers: all four
  (`cells` and `rows` in burning; `cells`, `rows`, `bounds` in frozen, poisoned, petrified).
- The **stand-in figure**, a `Silhouette::from_fn` (`silhouette.rs:88`) body to the ruler, in mid grey `6b6b6b`
  (pixelfx's `colorAt` fallback, `_lib.js:60`): a head disc of radius 3.5 at (0, -34.5) (`size(head, 1.0) = 7`
  across), a trunk `abs(x) < 5` for y in -31..-14, arms `5 <= abs(x) < 7` for y in -29..-18, legs `1 <= abs(x) < 4`
  for y in -14..0. It has up cells (the head, the shoulders), hang cells (under the arms, the chin), flanks and a
  60 % row with a right edge: every effect finds its spots on it, and the goldens are rendered on it. (power-up's
  own stand-in, `power_up.rs:310-322`, stays as it is: changing it would change its goldens.)
- `pub(crate) fn floor_light(p: &mut Painter, rx: f64, ry: f64, c: Rgba, k: f64)`: the ellipse of light under the feet
  that the three sources draw with `shade` (`burning.js:77-80`, `frozen.js:100-103`, `poisoned.js:82-85`):
  `p.shade(-rx - 1, -3, rx + 1, 7, |x, y, out| { d = hypot(x / rx, (y - 0.5) / ry); if d < 1 { out.l = c; out.k = k
  * (1 - d)^2 } None })`. Callers: burning, frozen, poisoned.
- The shared `BOX: Area { x: -32, y: -55, w: 64, h: 64 }` the silhouettes are read in (the whole canvas).

Lands in S1 as burning's own module (`#[cfg(feature = "feedback-burning")]`), with `floor_light` and the stand-in;
S2 widens the `cfg` to `any(...)` when frozen, the second caller, arrives (the rule's letter: shared when two
callers exist). Nothing in it depends on which effect calls it.

### 3.2 `Silhouette::open(dx, dy)` in `pixfx-core` (`crates/pixfx-core/src/silhouette.rs`)

`pub fn open(&self, dx: i32, dy: i32) -> impl Iterator<Item = (i32, i32)> + '_`: the covered cells whose neighbour
at `(dx, dy)` is uncovered, row by row: pixelfx's `up` (0, -1), `side` (-1, 0) and (1, 0), and the hang cells (0, 1),
next to `edge()` (`silhouette.rs:136-142`, which is the union of the four). A query of the body, as generic as `edge`
and `ring`, so it belongs with them (pixfx-fighter's `status.rs` draws rime off a body's sides the same way,
`crates/pixfx-fighter/src/status.rs:571`, and could use it later). Callers: burning (up), frozen (up, both sides,
hang), poisoned (hang), petrified (all four, hang). It adds code and changes no pixel of any existing effect, so
every golden stays; the slice that adds it runs the crate's tests with `all` and the wasm golden check once
(`crates/pixfx-effects/AGENTS.md:71-72`), and a unit test in `silhouette.rs` on a 3 x 3 body. S1 writes the five-line
filter privately in `status.rs`; S2 moves it to core (two callers then) and burning's golden check proves nothing
moved.

### 3.3 What is not shared, and why

- *Flames, embers, smoke* (`tongue`, `ember`, `smoke`): one caller (burning). They would go to `pixfx_draw::shapes`
  beside `lobes` when a second effect burns; not now.
- *Drips*: `particles::cycle` and `fly` already exist in `pixfx-core` and are what the sources use; soaked's
  `Grid`/`drip` (`soaked.rs:196-368`) is a different look (fat teardrops) and stays soaked's.
- *A creep-up-the-silhouette helper*: one caller (petrified; frozen's ferns creep in from the sides, encase closes
  collars). Petrified's `front`/`level` stay private; `loop_from` is the engine's own way to play an opening once.
- *A body-tint helper*: the three tints differ in their rule (frozen mixes toward a luma-indexed ice step with a rim
  rule; poisoned mixes with a row wave; petrified replaces with a shaded grey). What they share is the per-cell
  colour and luma, which `Body::cells` holds; the rule stays in each file (a shared `tint(cells, f)` would be a
  one-line `for` loop).
- *Option B's single painter*: section 4.

---

## 4. Two options

**A. Four independent ports plus the small shared module** (sections 2 and 3): each effect is one file like
`electrified.rs` or `soaked.rs`, owns its `META`, options, palette, per-pose tables and `draw`; `status.rs` holds
only what all of them read (`Body`, the stand-in, `floor_light`), and `Silhouette::open` goes to core.

**B. One shared status painter with palettes** (the `dither.rs` shape, `src/dither.rs:51-116`): a `pub(crate)`
`Status` painter in `src/feedback/status.rs` with `enum Ailment { Burning, Frozen, Poisoned, Petrified }`, one
`set_actor` building every table, one `draw(p, t, layer, ailment)` matching on the ailment, and four thin id files
holding a `META` and a `State(Status)` each.

| | A | B |
|---|---|---|
| Fit to the precedent | `dither.rs` shares one painter because `appear/dither` and `disappear/dither` draw the same frames reversed (`src/appear/dither.rs:71`); the four ailments share no frame | forces four unlike looks through one `match` |
| Builds alone | `--features feedback-burning` compiles burning and the 60-line `status.rs` | compiles the whole painter (every ailment's tables and draw) for one feature: a game that plays burning ships frozen's code, against "a game cuts size by building only the effects it plays" (root `AGENTS.md`, "Fast") |
| Review and bench | one file per slice; the bench isolates each | one 1,500-line file reviewed four times; a bench change in one ailment is a diff in the shared file |
| Goldens | a change to one effect re-blesses one entry | a refactor of the shared painter can move four entries |
| Reuse | `Body`, the stand-in, `floor_light`, `Silhouette::open`: what is the same | the same plus `match` arms that are not reuse |
| Coupling | none between effects (the rule "never couple effect A to effect B") | each ailment's code sits beside the others'; a fix for frozen's rim touches the file petrified lives in |
| Second-order | four more files in `src/feedback/` (fourteen effects there); the catalogue grows four rows | one big module to keep in order; a fifth ailment (sleep, stunned) means another arm |

**Pre-mortem, A.** (1) *The four drift apart in conventions* (one keeps state, one reads the sprite in `draw`): the
acceptance criteria 1.1 and the reviewer checks in section 5 pin the conventions, and the sprite tests pin the
reads. (2) *`status.rs` grows into B by accretion*: it holds only what every ailment reads; a helper with one caller
stays in that caller's file (3.3). (3) *Burning's cost blows the budget* (nested tongue layers, translucent smoke):
measured in S1 before anything else; the fallback is to draw the smoke with `dot` instead of `blend_px` and cap the
tongue rows, both within the ceiling of electrified. (4) *The first `loop_from` user finds a shell that recomputes
frames itself*: no shell does (grep `loop_from`: core, its test and `live_frames.rs` only; the scene takes
`clock.frame()`, `crates/pixfx-scene/src/lib.rs:816`), and S4's reviewer plays petrified in the Canvas 2D demo
past two loops. (5) *Goldens conflict with the slash chain*: section 5.6.

**Pre-mortem, B.** (1) *A game's wasm grows by three ailments it never plays*: inherent. (2) *One golden change
ripples*: a refactor to the shared `draw` moves four entries and each needs its reason in the commit. (3) *Reviewers
cannot judge one ailment alone*: the diff of S2 touches the file S1 landed. (4) *The thin id files still need their
own options, palettes and tags*: the sharing buys the `set_actor` and the stand-in, which A shares too.

**Recommended: A.** It is the shape of the three existing status ports, it keeps "one effect builds, tests and
benches alone behind its feature" true, and it shares exactly the code that is the same.

---

## 5. Slices, in build order

Each slice is one commit (or a few) on this branch, reviewable alone: it builds, tests and benches behind its own
feature with the commands of 1.5, blesses its own golden, and touches only the files it lists. S1..S4 are one
effect each, in the order that lets the shared code earn its place (burning first because it needs the most of
`Body` and the floor light; frozen second because it is the second caller of everything shared); S5 is the
registration everything else reads. Commit messages: `effects: feedback/<name>, <one line of what it shows>`, with
the attribution line the chain's reminder gives.

Registration rows are research section 2's checklist (rows 1-12 at `docs/research-status-effects.md:204-217`).

### S1. `feedback/burning` (and `status.rs` as its own module)

| | |
|---|---|
| Files | `crates/pixfx-effects/src/feedback/burning.rs` (new), `src/feedback/status.rs` (new, `#[cfg(feature = "feedback-burning")]` in `mod.rs`), `src/feedback/mod.rs` (two `cfg` lines for burning, one for `status`), `crates/pixfx-effects/Cargo.toml` (`feedback-burning` in the `feedback` list at `:21`, `feedback-burning = []` before `feedback-cloak-shimmer` at `:31`, a `[[test]] name = "burning" required-features = ["feedback-burning"]` block placed right after the `status` block at `:53-55`), `src/lib.rs` (`EFFECTS` entry before `CLOAK_SHIMMER` at `:31`), `tests/burning.rs` (new), `tests/goldens/frame_hashes.json` (blessed entry) |
| Checklist rows | 1, 2, 3, 4, 5 |
| Must pass | the one-effect block of 1.5 with `feedback-burning`: fmt, clippy `-D warnings`, `--all-targets` (crop, live_frames, bench's unit tests, `tests/burning.rs`), the two gifs looked at, bless, the golden check, the release bench under the ceiling (draw <= 1.34, rgba <= 0.66) |
| Reviewer / QC checks | the gif on the idle fighter: flames on hair, shoulder and flanks, leaning left; the white frame at 0 and 8; two scorch patches per tick gone before the next; smoke behind the body. `set_actor` builds every table; `draw` reads only `self` and `t`; no `update`; every random draw hashed from `(seed, K, item, frame)`; `size`s named against a ruler part in comments; the module doc carries the pixelfx and Sprite Fusion credit lines; the bench table pasted in the commit or MR; `status.rs` holds nothing burning-specific |

### S2. `feedback/frozen` (and the shared code lifted)

| | |
|---|---|
| Files | `src/feedback/frozen.rs` (new), `src/feedback/status.rs` (the `cfg` widened to `any(...)` of the four features; `open` removed), `crates/pixfx-core/src/silhouette.rs` (`Silhouette::open` next to `edge`, with a unit test), `src/feedback/burning.rs` (calls `sil.open(0, -1)` instead of its private filter: no pixel changes), `mod.rs`, `Cargo.toml` (`feedback-frozen` in the list; `feedback-frozen = []` after `feedback-encase` at `:33`; the `[[test]] name = "frozen"` block after burning's), `lib.rs` (entry after `ENCASE` at `:36`), `tests/frozen.rs` (new), goldens (frozen's entry) |
| Checklist rows | 1, 2, 3, 4, 5 |
| Must pass | the one-effect block with `feedback-frozen`; then, because `pixfx-core` and a shared module changed: `cargo test --offline -p pixfx-core`, `cargo test --offline -p pixfx-effects --all-targets`, `cargo test --offline -p pixfx-effects --test determinism -- --ignored` (every golden unchanged, burning's included), `wasm-pack test --node crates/pixfx-effects --test determinism -- --include-ignored`; burning's bench again (no regression: within 0.05 ms of S1's number) |
| Reviewer / QC checks | the gif: the body blue-grey with a lit rim, ferns cracking off on the shiver and creeping back, icicles under the chin and hands dripping, snow caps, breath, frost underfoot breathing; the overlay jolts on frames 0..3 and the body does not. `Silhouette::open` is a pure filter with a doc comment and a test; `git diff` of burning.rs is the one call site; the goldens diff adds frozen's entry and changes nothing else |

### S3. `feedback/poisoned`

| | |
|---|---|
| Files | `src/feedback/poisoned.rs` (new), `mod.rs`, `Cargo.toml` (`feedback-poisoned` in the list; `feedback-poisoned = []` after `feedback-outline` at `:34`, after the `feedback-petrified` line once S4 lands or before it: alphabetical; the `[[test]] name = "poisoned"` block), `lib.rs` (entry after `OUTLINE` at `:38`), `tests/poisoned.rs` (new), goldens |
| Checklist rows | 1, 2, 3, 4, 5 |
| Must pass | the one-effect block with `feedback-poisoned`; `cargo test --offline -p pixfx-effects --all-targets` (the `any(...)` module compiles with three callers) |
| Reviewer / QC checks | the gif: two green waves per loop, veins from the forearm wound growing and receding, blisters swelling and popping at frame 7 of the tick, sweat from brow and hands hissing on the floor, the ring in two halves round the feet; `veins_color` changes the veins and nothing else; the bench under 1.0 / 0.66 |

### S4. `feedback/petrified`

| | |
|---|---|
| Files | `src/feedback/petrified.rs` (new), `mod.rs`, `Cargo.toml` (`feedback-petrified` in the list; `feedback-petrified = []` after `feedback-outline` and before `feedback-poisoned`; the `[[test]] name = "petrified"` block), `lib.rs` (entry after `OUTLINE`, before `POISONED`), `tests/petrified.rs` (new), goldens |
| Checklist rows | 1, 2, 3, 4, 5 |
| Must pass | the one-effect block with `feedback-petrified`; `cargo test --offline -p pixfx-effects --all-targets`; and, as the first `loop_from` user, a live check: `packages/pixfx-canvas2d/demo/main.ts` (the demo that plays power-up on a hero, skill `pixfx-sprite-aware`) switched to `feedback/petrified` for a look of three loops, or `cargo run --offline -p pixfx-export --example mp4 -- feedback/petrified` for 5 s if the demo is not at hand: the creep plays once, the hold loops |
| Reviewer / QC checks | the gif: grey climbing behind a flickering white edge spitting grit, the set flash and dust skirt, then the held statue with its cracks, the glint, one pebble, one mote; `creep = 0` shows the statue from frame 0; no grit or dust visible on any loop frame (12..27) that was born in the opening; the bench under 0.6 / 0.66; `loop_from` returns `Some(0.75)` and `META.duration` is 1.75 |

### S5. Registration everything else reads, and the final bless

| | |
|---|---|
| Files | `crates/pixfx-effects/AGENTS.md:9-12` (the four ids in the crate's list), `README.md:12-16` (the four ids), `CHANGELOG.md` (four bullets under the next version's heading; `## Unreleased` if the slash chain's heading is there after a rebase, else a new one above `## 0.3.0` at `:3`), `site/docs/effects.html` (four rows in the table, alphabetical: burning before cloak-shimmer at `:36`, frozen after encase at `:38`, petrified and poisoned after outline at `:39`), `site/assets/effect-roles.js` (four `TABLE` entries after `feedback/electrified` at `:60`: `{ role: "buff", impact: <beat>, power: 16, spell: "ward" }` with `impact` 0.5 for burning (the tick at frame 8), 1.0 for frozen (the second shiver), 0.75 for poisoned (the tick at frame 12), 0.5 for petrified (the set at frame 8)), `site/assets/effect.js` `OWN` (a slider per number option, four blocks after `feedback/soaked` at `:77-82`; `veins_color` has no slider: `OWN` holds numbers), `tests/goldens/frame_hashes.json` (re-blessed with `all`: no entry changes) |
| Checklist rows | 6, 7, 8, 9, 10, 11 (12 is already done: `tests/status.rs` names the ids) |
| Must pass | `cargo test --offline -p pixfx-effects --all-targets` (features, status, crop, live_frames, the four sprite tests, body_burst); `cargo test --offline -p pixfx-effects --test determinism -- --ignored` with `all`; `wasm-pack test --node crates/pixfx-effects --test determinism -- --include-ignored`; `site/build.sh` builds (it needs the arena checkout at `../../projects/arena`, `site/build.sh:17-19`; if that is not at hand, say so and check the three site files by eye); `bun run build:wasm` in `packages/pixfx-web` so the `EffectId` union (`crates/pixfx-wasm/build.rs:1-18`) carries the ids; `cargo run --offline -p pixfx-export --example scale_board -- --actor crates/pixfx-pixel/tests/fixtures/thunder-idle.png --id feedback/burning --id feedback/frozen --id feedback/poisoned --id feedback/petrified` for the ruler check |
| Reviewer / QC checks | `cargo test --offline -p pixfx-effects --test status` passes with its two tests; each effect's page on the site stages it on the duel's fighter with its sliders; the four rows read like the others; `git diff` touches no `.rs` file |

### 5.6 Avoiding conflicts, between the slices and with the sword-slash chain

The other chain (`/Users/firegroup/.remit/work/ch_3d3b8109bb4d/pixfx-rs`, branch
`feature/mc-3d3b8109bb4d-implement-sword-slash-effect-in`, at `daf176f`, four commits on `d5cc189`) adds
`melee/sword-slash` in a new category `melee`. Its diff against `d5cc189` (11 files, `git diff --stat d5cc189 HEAD`
in that worktree) touches: `Cargo.toml` (`all` at `:16`, a `melee = [...]` line after `feedback` at `:21`,
`melee-sword-slash = []` after `feedback-soaked` at `:36`, a `[[test]] name = "sword_slash"` block after
`body_burst`), `lib.rs` (`pub mod melee;` after `feedback`, an `EFFECTS` entry after `SOAKED` at `:42`), new
`src/melee/`, `tests/sword_slash.rs`, the goldens tail (one entry appended), `CHANGELOG.md` (a `## Unreleased` heading
and a bullet at the top), `README.md:13-14`, `crates/pixfx-effects/AGENTS.md:9-10`, `site/docs/effects.html` (a row
after soaked), `site/assets/effect-roles.js` (an entry after cloak-shimmer at `:66`). Rules that keep the two apart:

- **Alphabetical, interior insertions.** Our `feedback-*` feature lines, `EFFECTS` entries and catalogue rows go
  in alphabetical order inside the feedback block (`burning` before `cloak-shimmer`, `frozen` after `encase`,
  `petrified` and `poisoned` after `outline`): every insertion of ours has an unchanged line between it and theirs
  (`feedback-power-up`, `feedback-soaked`; `POWER_UP`, `SOAKED`; the power-up and soaked rows), so git merges them
  without a conflict. The one line both chains edit is `Cargo.toml:21` (we add to `feedback = [...]`; they add
  `melee = [...]` on the next line): adjacent edits conflict in git; the resolution is to keep both lines.
- **`[[test]]` blocks** go right after the `status` block (`Cargo.toml:53-55`), before `body_burst`; theirs goes
  after `body_burst`: the `body_burst` block separates them.
- **`effect-roles.js`** entries go after `feedback/electrified` (`:60`), not at the table's end; theirs is after
  cloak-shimmer at the end.
- **Goldens.** Blessing appends at the end of the object (`tests/determinism.rs:79`, `preserve_order`), and theirs
  did too: the tail will conflict. Whoever rebases second re-blesses their own effects (`PIXFX_BLESS=1 cargo test
  --offline -p pixfx-effects --no-default-features --features feedback-burning,feedback-frozen,feedback-poisoned,feedback-petrified
  --test determinism -- --ignored` keeps every other entry, `:79`) and runs the whole golden check once.
- **Prose** (`CHANGELOG.md`, `README.md`, `AGENTS.md`): the same lines; a trivial conflict resolved by keeping both
  lists. S5 is the only slice that touches them, and it is the last, so a rebase onto a merged slash chain happens
  once, before S5.
- **Each slice commits only its own files**, through a temporary index (`git add <files>`, never `git commit -a`;
  skill `pixfx`, "Several sessions share this tree"), and leaves `docs/research-*` and this file alone (the Chair
  decides whether the design documents are committed).
- **Nothing of ours touches** `crates/pixfx-draw/src/sweep.rs`, `crates/pixfx-fighter`, `src/melee/` or
  `tests/sword_slash.rs`; nothing of theirs touches `src/feedback/`, `pixfx-core/src/silhouette.rs` or
  `tests/status.rs`.
- **Their `tests/features.rs`** will see our four features in `feedback` and ours will see their `melee` in `all`
  after either merge: both pass, the test reads the manifest.

---

## 6. Risks and assumptions, for the merge request

Product choices the goal leaves open, taken conservatively:

1. **Names.** `burning`, `frozen`, `poisoned`, `petrified`: the ids `tests/status.rs:13-19` already names, under
   `feedback/` beside `electrified`.
2. **The sprite stays** (1.4): no shiver, no sway. A game that wants the body to shake asks `pixfx-core` for a feel
   rule; it is not in this design.
3. **No cues** (1.3). A damage tick's shake or sound is the game's; the site fakes the beat with `impact`.
4. **One canvas for all four**, 64 x 64 with the feet at (32, 55): the ruler's cell, 128 x 128 frames in shells.
   A shell that batches by frame size is not affected (the atlas packs the lit box).
5. **Petrified opens once** (`loop_from`, 2.4) and is the first shipped user of that hook; the `creep` option gives
   the pure loop. If the live check in S4 finds a shell that mishandles it, the fallback is `creep = 0` as the
   default and a note in the changelog, not a change to the hook.
6. **Clock changes against the sources**: burning 1.0 s / 0.5 s tick (source 1.2 / 0.6), poisoned 1.5 s / 0.75 s
   tick (source 1.6 / 0.8), petrified's creep 0.5 s (source 0.95 s): whole frames at 16 fps. Frozen keeps 2.0 s / 1 s.
7. **Pixels are not pixel-identical to pixelfx** (the noise hash and the clock differ); the 0.3.0 ports' "100 %
   parity" (`CHANGELOG.md:12`) is not claimed. The goldens pin our frames.
8. **The stand-in figure** is new (3.1) and what the goldens, the bench and the status test render; it is not
   power-up's, whose goldens stay.
9. **Sizes against the ruler** (section 2, "Scale"): a source size on a 48-row body is restated as a part and a
   ratio; a reviewer who finds a flame or an icicle too big on the fighter changes the ratio, not the pixel count.
10. **Bench ceilings** are electrified's measured numbers on this machine (research 3.4); a slower machine compares
    against its own electrified run, not the number.
11. **`Silhouette::open` in core** changes no existing pixel; the full golden check and the wasm check in S2 prove
    it. If the Chair prefers no core change, `open` stays in `status.rs` (category level) with the same callers.
12. **Credits**: burning and petrified adapt Sprite Fusion art through pixelfx; their module docs carry both credit
    lines as `incinerate.rs:4-5` and the sources' headers do. `NOTICE.md:5-9` already credits pixelfx at `044e12a`,
    the commit the sources were taken from; no change there.
13. **The site build** needs the arena checkout (`site/build.sh:17-19`); S5 says if it could not run it.
14. **Not in scope**: `sleep`, `stunned`, `slowed`, `charmed`, `confused`, `cursed`, `rage`, `haste` (pixelfx's other
    statuses, research 5.2); a `stone` palette preset word (`pal::STONE` exists as a ramp); `eyes()` detection.

---

## Appendix: measured numbers behind the canvas

The idle fighter fixture (`crates/pixfx-pixel/tests/fixtures/thunder-idle.png`, 64 x 64, feet at (32, 60) per
`RULER`), read by a script: opaque box x 20..39, y 22..61, so x -12..7 and y -38..1 from the feet; 521 body cells,
33 up cells, 33 hang cells, 104 edge cells; luma spread 0..1 with 172 cells under 0.1 (the outline) and 125 in
0.8..0.9 (the skin). The canvas of 1.1 holds it with 15 rows above the crown (-53 of -55) and 8 under the soles.
