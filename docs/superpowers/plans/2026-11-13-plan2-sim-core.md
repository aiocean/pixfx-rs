# pixfx-rs Plan 2: Simulation Core and First Sample Effects

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Port the rest of pixelfx's simulation core (`sim.js`, the parts of `particles.js`, `shapes.js`, `math.js`,
`color.js` and `painter.js` that effects need), then prove it with 21 effects, at least one from each of the 17
categories that need no acting, each passing `pixfx parity`.

**Architecture:** `pixfx-core` gains `sim.rs` (the whole of pixelfx `sim.js`), `particles.rs` and `shapes.rs`, the
missing math and colour helpers, and the missing painter shapes. Each core piece is checked against values
printed by a Node script that runs pixelfx's own code (the `tests/golden/*.mjs` pattern from
`smoke_ramp.mjs`). The simulation does not own a random generator: every call that draws random numbers takes
`&mut Rng` from the effect, because pixelfx effects share one `Rng` between their own code, their particle
systems and their blazes, and parity depends on the exact order of draws. Effects then port one category at a
time into `pixfx-effects/src/<category>/`, the way `fire/` is laid out.

**Tech Stack:** Rust (pinned nightly), `libm`, wasm-pack (`wasm-bindgen-test` under Node), Node ≥18 for the pixelfx
reference scripts.

**Spec:** `docs/superpowers/specs/2026-11-11-pixfx-rs-design.md` (Goals 1 and 2, *Determinism*, *Verification*).

**This plan's slice of the spec:** the simulation part of Goal 1 and the first 21 of Goal 2's 244 effects.
**Not in this plan:** acting (`act.js`, `actor.js`, `rig.js`, `poses.js`, `chain.js`) and the `melee`, `moves` and
`status` categories that need it; `font.js` and the `ui` category; painter `sprite`, `text`, `measure`, `mask`,
`outline`, `arc`, `beam`, `polyline`, `get`; `particles.js` `burst`, `stream`, `flyV`, the simple `Particles`
class (pixelfx's `core/index.js` re-exports `sim.js`'s `Particles` over it, so effects never see it);
`shapes.js` `star`, `ngon`, `trail`. A sample effect that calls one of these ports it in its own task, in the
same file the JS has it in.

**Reference sources:** pixelfx at `/tmp/fxref/pixelfx` (commit `044e12a`). If it is gone:
`git clone https://github.com/patch-gg/pixelfx /tmp/fxref/pixelfx && git -C /tmp/fxref/pixelfx checkout 044e12a`.

## Global Constraints

- Every crate: `#![forbid(unsafe_code)]`; clippy `unwrap_used`, `expect_used`, `panic` = deny; no `clone()` or
  `Arc<Mutex>` added only to satisfy the borrow checker.
- In `pixfx-core` and `pixfx-effects`: `sin cos tan atan2 pow exp ln sqrt hypot floor ceil` go through `libm`;
  JS `Math.round` is `pixfx_core::math::js_round`; JS `Math.min`/`Math.max` are `js_min`/`js_max` (NaN
  semantics). `clippy.toml` `disallowed-methods` enforces the libm part. f64 throughout. No `HashMap` iteration in
  draw paths. No I/O, clock or OS randomness.
- Random draws happen in exactly the order pixelfx makes them. JS evaluates call arguments left to right, then
  runs the call; Rust does the same. A port may not hoist, merge or reorder `R.range(...)`, `R.next()`,
  `R.chance(...)` or `R.pick(...)` calls, and `Particles::spawn` draws its particle seed after its arguments,
  as JS `spawn` does.
- Lists that JS walks backwards and `splice`s remove with `Vec::remove` (keeps order), never `swap_remove`.
  Items pushed during such a walk are not visited in that walk (index the loop; do not iterate a borrowed slice).
- Headers: a file whose JS says "adapted from Sprite Fusion" keeps that comment verbatim.
- Repo-wide check before every commit: `cargo fmt --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace`.
- Parity gate per effect: `cargo run -p pixfx-cli -- parity <id> --pixelfx /tmp/fxref/pixelfx` prints `PASS`
  (colour and light each ≤0.5% of pixels differing). The fire ports reached 0 differing pixels; aim for 0. A FAIL
  is debugged (superpowers:systematic-debugging) until PASS or until it is shown to be JS-vs-libm float drift (the
  first differing sub-step value located); then a line `<id> · <date> · colour x% light y% · pending owner
  approval` goes in `parity/LEDGER.md` and the task reports it. Never write "approved by owner" yourself.

## Review Focus

1. **Random draws reordered.** A port that computes `R.range` into a `let` above a `spawn` whose other arguments
   also draw, builds a `Spawn` or `Bit` with `..base` while the explicit fields draw (Rust evaluates the base
   last), or draws the particle seed before the arguments, shifts every later particle. Expected: the same particles as pixelfx. Pinned by the Task 3 golden scene (a FIRE that dies
   and spawns SMOKE, and a BRAND whose landing draws inside the update loop) and by every effect's parity run.
2. **Particle at rest.** A particle with `vx = vy = 0` (a STAR spawned still, a GRIT that settled) has
   `Math.hypot(0, 0) || 1` = 1 in JS; a Rust `hypot` of 0 divides by zero and paints NaN coordinates. Expected:
   it draws like pixelfx. Pinned by the still MOTE in the Task 3 scene.
3. **Loop effects with a live simulation, played for minutes.** Particles spawned every step of a loop must die
   as fast as they are born. Expected: drawing one frame after 10 minutes of play takes about as long as after
   one loop. Test `long_play_stays_bounded` in Task 6.
4. **Native and wasm disagree on a new effect.** A `std` float method slipped past clippy (for example through a
   trait method), or a `HashMap` walked while drawing. Expected: identical frame hashes. Every effect task adds
   its id to the determinism test and runs it under wasm.
5. **Caller seeds.** `Options.seed` must reach the effect's `Rng` in stateful effects (pixelfx `new Rng(o.seed)`),
   not only `Meta::seed`. Expected: seeds 1, 2 and 7 give different frames. The determinism goldens carry three
   seeds per effect; a task whose three seed rows are identical has the bug.

---

### Task 1: Math and colour helpers

**Files:**
- Modify: `crates/pixfx-core/src/math.rs` (add `ease`, `fbm`, `tile_noise`, `tile_fbm`, `loop_noise`)
- Modify: `crates/pixfx-core/src/color.rs` (add `pal`)
- Create: `crates/pixfx-core/tests/golden/math.mjs`
- Test: `crates/pixfx-core/tests/math.rs`

**Interfaces (produces):**
- `pub mod ease` inside `math.rs` with one `pub fn(t: f64) -> f64` per pixelfx `ease` key, snake_case:
  `linear in_quad out_quad in_out_quad in_cubic out_cubic in_out_cubic in_quart out_quart in_expo out_expo
  in_sine out_sine in_out_sine out_elastic out_bounce`, plus `in_back(t: f64, s: f64)` and
  `out_back(t: f64, s: f64)` (JS default `s = 1.70158` is the const `ease::BACK_S`).
- `fbm(x: f64, y: f64, seed: f64, octaves: u32) -> f64`, `tile_noise(x: f64, y: f64, px: f64, py: f64, seed: f64) -> f64`,
  `tile_fbm(x: f64, y: f64, px: f64, py: f64, seed: f64, octaves: u32) -> f64`, `loop_noise(phase: f64, radius: f64, seed: f64) -> f64`.
  JS defaults become explicit arguments.
- `pub mod pal` inside `color.rs`: one `pub const NAME: [Rgba; N]` per pixelfx `PAL` entry (`FIRE`, `EMBER`,
  `LAVA`, ... `NIGHT`, 27 ramps), values copied from `src/core/color.js:52-81`.

- [ ] **Step 1: Write the golden script.** `math.mjs <pixelfx dir>` imports `src/core/math.js` and `color.js`
  and prints, as Rust array literals ready to paste: every `ease` function at `t` in
  `[-0.1, 0, 0.3, 0.5, 0.9, 1, 1.2]`; `fbm(1.3, 2.7, 5, 3)`; `tileNoise(3.3, 7.6, 4, 8, 2)` and
  `tileNoise(3.3, 7.6, 0, 0, 2)`; `tileFbm(3.3, 7.6, 4, 8, 2, 3)`; `loopNoise(p, 1.5, 4)` for `p` in
  `[0, 0.37, 1]`; every `PAL` ramp as packed words. Header comment as in `smoke_ramp.mjs`.
- [ ] **Step 2: Write the failing tests** in `tests/math.rs`: `ease_matches_pixelfx`, `noise_matches_pixelfx`
  (each value within `1e-12` of the golden), `loop_noise_seamless` (`loop_noise(0.0, ..) == loop_noise(1.0, ..)`
  within `1e-12`), `pal_matches_pixelfx` (packed words equal).
- [ ] **Step 3:** `cargo test -p pixfx-core --test math` → FAIL (unresolved names).
- [ ] **Step 4: Implement.** Straight ports of `math.js:26-50, 114-152` and `color.js:52-81`.
- [ ] **Step 5:** `cargo test -p pixfx-core --test math` → PASS; repo-wide check passes.
- [ ] **Step 6: Commit** `core: ease, fractal and tiling noise, palettes`.

### Task 2: Painter shapes

**Files:**
- Modify: `crates/pixfx-core/src/painter.rs`
- Create: `crates/pixfx-core/tests/golden/painter_shapes.mjs`
- Test: `crates/pixfx-core/tests/painter.rs` (add tests)

**Interfaces (produces), same parameter order as JS, `a` explicit:**
- `disc(&mut self, cx: f64, cy: f64, r: f64, c: impl Param2<Rgba>, a: impl Param2<f64>)` (`painter.js:81`)
- `circle(&mut self, cx: f64, cy: f64, r: f64, c: impl Param2<Rgba>, a: impl Param2<f64>)` (`painter.js:171`)
- `ring(&mut self, cx: f64, cy: f64, r: f64, w: f64, c: impl Param2<Rgba>, a: impl Param2<f64>)` (`painter.js:193`)
- `light(&mut self, x: f64, y: f64, c: Rgba, k: f64)` (`painter.js:102`)

- [ ] **Step 1: Write the golden script.** `painter_shapes.mjs <pixelfx dir>`: for each case, a fresh
  `createFrame(64, 48, 0, 0)`, `new Painter(frame).place(32, 40, false, 0)`, draw, print the frame's FNV-1a 64
  (over `color` then `light` words, 4 little-endian bytes each, BigInt; the same as `pixfx_core::frame_hash`)
  as hex. Cases: `disc(0,-16,9,"#ff8a2a",0.6)`; `disc(0,-16,0.4,"#ff8a2a",1)`;
  `circle(0,-16,9,(d)=>d<0.5?WHITE:hex("#3f8cff"),1)`; `circle(0,-16,0.5,"#3f8cff",1)`;
  `ring(0,-16,12,3,"#9a48ec",(v)=>v)`; `light(3,-5,"#ffe26b",0.7)`; the disc case again with
  `place(32, 40, true, 0.6)` (mirrored and rotated).
- [ ] **Step 2: Write failing tests** `shapes_match_pixelfx`: each case drawn in Rust, `frame_hash` equals the
  golden.
- [ ] **Step 3:** run → FAIL. **Step 4:** implement. **Step 5:** run → PASS; repo-wide check passes.
- [ ] **Step 6: Commit** `core: painter disc, circle, ring, light`.

### Task 3: Particle simulation

**Files:**
- Modify: `crates/pixfx-core/src/sim.rs` (keep its Sprite Fusion header), `crates/pixfx-core/src/lib.rs`
- Create: `crates/pixfx-core/tests/golden/sim_scene.mjs`
- Test: `crates/pixfx-core/tests/sim.rs` (add tests)

**Interfaces (produces):**
- `pub const GRAVITY: f64 = 950.0;` `pub const FIRE_RAMP: [Rgba; 9]` (`sim.js:15`); `HOT` stays private.
- `pub enum Kind { Spark, Mote, Streak, Flake, Vapour, Dust, Star, Ember, Smoke, Fire, Brand, Grit }` (pixelfx `P`).
- `pub struct Flags(pub u8)` with `SMOKES = 1`, `BIG = 2`, `BACK = 4`, `NONE = 0` as associated consts, `BitOr`,
  and `contains(self, Flags) -> bool` (pixelfx `PF`).
- `pub struct Spawn { pub kind: Kind, pub x: f64, pub y: f64, pub vx: f64, pub vy: f64, pub life: f64,
  pub col: Option<Rgba>, pub size: f64, pub delay: f64, pub flags: Flags }` and
  `Spawn::new(kind, x, y, vx, vy, life)` with JS defaults (`col: Some(WHITE)`, `size: 1.0`, `delay: 0.0`,
  `flags: Flags::NONE`). JS `col` 0 is `None` (FIRE and SMOKE use their own ramps). Rust evaluates the fields of a
  struct expression in written order and the `..base` expression **last**, so `Spawn { size: R.range(..),
  ..Spawn::new(.., R.range(..)) }` draws `size` before `life` and breaks parity. Rule: when any extra field draws
  from the rng, write the full literal `Spawn { kind, x, y, vx, vy, life, col, size, delay, flags }` in JS
  argument order; use `..Spawn::new(..)` only when the extra fields draw nothing. The same rule holds for `Bit`
  (Task 4), whose fields follow the JS object literal's order.
- `pub struct ParticlesOpts { pub ground: f64, pub sparks: Vec<Rgba>, pub haze: Option<Rgba> }`, `Default` =
  `{ ground: f64::INFINITY, sparks: HOT, haze: None }`. `ground` is a constant floor: every `ground:` in pixelfx
  `044e12a` (142 uses) is `() => 0`, `() => 4`, `() => Infinity`, a `floor` defined as `() => 0`, or scifi
  `_lib.js:85` passing its caller's one through, and those callers pass `() => 0`.
- `pub struct Particles` with `new(opts: ParticlesOpts) -> Self`,
  `spawn(&mut self, rng: &mut Rng, s: Spawn)`,
  `update(&mut self, rng: &mut Rng, dt: f64)`,
  `update_with(&mut self, rng: &mut Rng, dt: f64, on_land: impl FnMut(&mut Rng, f64, f64))` (pixelfx `onLand`, called
  inside the loop at the moment a BRAND lands, with the same `rng`; `update` is `update_with` with a no-op),
  `draw(&self, p: &mut Painter, layer: Layer)` (`Layer::Both` is JS's `layer` undefined),
  `count(&self) -> usize`.
  The rng is not stored: pixelfx effects pass their own `Rng` as `opts.rng` and keep drawing from it
  (`impacts/parry.js:20-21`), and `Blazes` draws from the same one.

- [ ] **Step 1: Write the golden script** `sim_scene.mjs <pixelfx dir>`. The scene, which the Rust test repeats
  line for line:
  - `R = new Rng(5)`; `blazes` declared first; `ps = new Particles({ rng: R, ground: () => 0, haze: "#88ccff",
    onLand: (x, y) => blazes.add(x, y - 0.25, R.range(0.9, 1.5), R.range(0.55, 0.85)) })`;
    `blazes = new Blazes(ps)`; `lights = new Lights()` (Task 4 types; write this script and its test in Task 3
    with only `ps`, then extend both in Task 4).
  - For `k` from 0 to 11: `ps.spawn(k, (k - 6) * 8, -20, R.range(-80, 80), R.range(-160, -40),
    R.range(0.6, 1.4), k === P.SMOKE || k === P.FIRE ? 0 : "#ffcc66", R.range(0.8, 2.6), k * 0.02,
    k === P.FIRE ? PF.SMOKES : k === P.SMOKE ? PF.BIG : 0)`.
  - `ps.spawn(P.MOTE, 0, -10, 0, 0, 1)` (still); `ps.spawn(P.SPARK, 4, -30, 50, -50, 1, WHITE, 1, 0, PF.BACK)`;
    `ps.spawn(P.BRAND, -10, -40, 30, -80, 2, WHITE, 1)`.
  - Step at `1/120` for 300 steps; each step `ps.update(dt)`, then (Task 4) `blazes.update(dt)`, `lights.update(dt)`.
  - After steps `0, 12, 60, 150, 300`: fresh `createFrame(160, 120, 0, 0)`, `new Painter(f).place(80, 100, false, 0)`,
    draw (Task 4: `blazes.drawBack(p)` first), `ps.draw(p, "back")`, `ps.draw(p, "front")` (Task 4: then
    `lights.draw(p)`), print `step, ps.count, hash` with the FNV-1a 64 from Task 2.
- [ ] **Step 2: Write failing test** `particle_scene_matches_pixelfx`: the scene in Rust; at each checkpoint
  `count()` and `frame_hash` equal the golden.
- [ ] **Step 3:** run → FAIL. **Step 4:** port `Particles` (`sim.js:20-268`) with the interfaces above.
  `Math.hypot(vx, vy) || 1` is `if sp > 0 { sp } else { 1.0 }` (also covers NaN).
- [ ] **Step 5:** run → PASS; repo-wide check passes.
- [ ] **Step 6: Commit** `core: particle simulation`.

### Task 4: Blazes, debris, lights and sim shapes

**Files:**
- Modify: `crates/pixfx-core/src/sim.rs`, `crates/pixfx-core/tests/golden/sim_scene.mjs`, `crates/pixfx-core/tests/sim.rs`

**Interfaces:**
- Consumes: Task 3's `Particles`, `Spawn`, `Kind`, `Flags`; Task 2's painter shapes.
- Produces:
  - `pub struct Blazes` with `new() -> Self`, `add(&mut self, rng: &mut Rng, x: f64, y: f64, life: f64, power: f64)`
    (draws `acc` from `rng` as JS `this.ps.R.next()`), `update(&mut self, ps: &mut Particles, rng: &mut Rng, dt: f64)`,
    `draw_back(&self, p: &mut Painter)`, `count(&self) -> usize`.
  - `pub enum BitKind { Shard, Chunk, Drip }`; `pub struct Bit { pub kind, pub x, pub y, pub vx, pub vy, pub a,
    pub va, pub s, pub c: Rgba, pub ramp: Option<Vec<Rgba>>, pub g, pub drag, pub bounce, pub life }` (f64 unless
    named) and `Bit::new(kind: BitKind, x: f64, y: f64)` with JS defaults (`vx vy a va drag bounce` 0, `s` 1,
    `c` WHITE, `ramp` None = `["#1c2030", "#3a4a66", "#6e86a8", "#a8c0dc", "#e8f2ff"]`, `g` 1, `life` 1).
  - `pub struct Bits` with `new(ground: f64) -> Self`, `add(&mut self, b: Bit)`, `update(&mut self, dt: f64)`,
    `draw(&self, p: &mut Painter)`, `count(&self) -> usize`.
  - `pub struct Lights` with `new()`, `add(&mut self, x: f64, y: f64, r: f64, color: Rgba, k: f64, life: f64)`,
    `update(&mut self, dt: f64)`, `draw(&self, p: &mut Painter)`.
  - `sparkle(p: &mut Painter, x: f64, y: f64, arm: f64, c: Rgba, tip: Rgba, k: f64)` (JS defaults `tip = c`,
    `k = 0.6` become explicit), `flare(p: &mut Painter, x: f64, y: f64, len: f64, k: f64, ramp: &[Rgba])`,
    `band(p: &mut Painter, cx: f64, cy: f64, r: f64, sy: f64, th: f64, k: f64, ramp: &[Rgba], keep: impl Fn(f64) -> bool, hot: bool)`,
    `crystal(p: &mut Painter, x: f64, base: f64, h: f64, w: f64, lean: f64, ramp: &[Rgba], o: CrystalOpts)` with
    `pub struct CrystalOpts { pub a: f64, pub white: bool, pub glint: f64 }`, `Default` = `{ a: 1.0, white: false, glint: -1.0 }`.
  - `lib.rs` re-exports the sim types and functions next to the existing ones.

- [ ] **Step 1: Extend the golden script and the Rust scene** with the blazes and lights lines marked "Task 4" in
  Task 3, plus `lights.add(0, -20, 30, "#ffaa44", 0.8, 0.5)` before the steps.
- [ ] **Step 2: Add a debris scene** to the script: `bits = new Bits({ ground: () => 0 })`;
  `bits.add({ kind: B.SHARD, x: -20, y: -30, vx: 60, vy: -120, va: 9, s: 5, c: "#d8f4ff", bounce: 0.4, life: 2 })`;
  `bits.add({ kind: B.CHUNK, x: 0, y: -40, vx: -40, vy: -90, va: -5, s: 3, bounce: 0.3, drag: 0.5, life: 2 })`;
  `bits.add({ kind: B.DRIP, x: 20, y: -50, c: "#58b4ec", g: 0.5, life: 2 })`; steps of `1/120`; hashes of a
  160x120 frame placed at (80, 100) after steps `0, 60, 240`, each with `bits.count`.
- [ ] **Step 3: Add a shapes case**: one frame, placed as above, with `sparkle(p, -40, -40, 5, hex("#ffe066"),
  hex("#b07a1a"), 0.6)`, `flare(p, 0, -60, 12, 0.7, R5)`, `band(p, 30, -30, 18, 0.4, 3, 0.9, R5,
  (g) => Math.sin(g) < 0, true)`, `crystal(p, -20, -5, 24, 10, 3, R7, { glint: 0.5 })`, where
  `R5 = ["#0c1430", "#1f4f94", "#58b4ec", "#d8f4ff", "#ffffff"]` and
  `R7 = ["#0c1430", "#162c5c", "#1f4f94", "#2f7fcc", "#58b4ec", "#9bdcff", "#d8f4ff"]`; one hash.
- [ ] **Step 4: Write failing tests** `particle_scene_matches_pixelfx` (now with blazes and lights),
  `debris_scene_matches_pixelfx`, `sim_shapes_match_pixelfx`.
- [ ] **Step 5:** run → FAIL. **Step 6:** port `sim.js:270-442`. **Step 7:** run → PASS; repo-wide check passes.
- [ ] **Step 8: Commit** `core: blazes, debris, lights, sparkle, flare, band, crystal`.

### Task 5: Closed-form particles and bolts

**Files:**
- Create: `crates/pixfx-core/src/particles.rs`, `crates/pixfx-core/src/shapes.rs`, `crates/pixfx-core/tests/golden/shapes.mjs`
- Modify: `crates/pixfx-core/src/lib.rs`
- Test: `crates/pixfx-core/tests/shapes.rs`

**Interfaces (produces):**
- `fly(x: f64, y: f64, vx: f64, vy: f64, t: f64, g: f64, drag: f64) -> (f64, f64)` (`particles.js:12`).
- `cycle(t: f64, period: f64, count: usize, seed: f64, life: impl Fn(&mut Rng) -> f64, f: impl FnMut(f64, f64, &mut Rng, usize))`
  (`particles.js:37`). A JS numeric `life` is a closure that ignores its rng (`|_| 0.8`): JS draws nothing from
  `g` for a number, and a closure that ignores `g` draws nothing either. Argument order of `f`: `(age, life, r, i)`.
- `bolt(r: &mut Rng, x0: f64, y0: f64, x1: f64, y1: f64, jag: f64, depth: u32) -> Vec<(f64, f64)>`
  (JS defaults `jag = 0.22`, `depth = 5` explicit).
- `forks(r: &mut Rng, path: &[(f64, f64)], count: usize, len: f64, jag: f64, spread: f64) -> Vec<Vec<(f64, f64)>>`
  (JS defaults `3, 20, 0.3, 0.9` explicit).

- [ ] **Step 1: Write the golden script** `shapes.mjs <pixelfx dir>` printing: `fly(0, 0, 40, -90, t, 300, d)` for
  `t` in `[0, 0.25, 1]` and `d` in `[0, 1.5]`; the list of `(age, life, i)` that
  `cycle(1.3, 2, 6, 9, (g) => 0.5 + g.next(), fn)` and `cycle(1.3, 2, 6, 9, 0.8, fn)` call `fn` with, plus the
  first `r.next()` of each call; `bolt(new Rng(4), 0, 0, 40, 60)` points; `forks(new Rng(4), thatBolt)` points
  (the second rng continues from after the bolt, as one `Rng` passed to both).
- [ ] **Step 2: Write failing tests** `fly_matches_pixelfx`, `cycle_matches_pixelfx`, `bolt_and_forks_match_pixelfx`
  (values within `1e-12`, call lists equal).
- [ ] **Step 3:** run → FAIL. **Step 4:** implement. **Step 5:** run → PASS; repo-wide check passes.
- [ ] **Step 6: Commit** `core: fly, cycle, bolt, forks`.

---

## Porting an effect (every task below follows this recipe)

For each effect id `<cat>/<name>` in the task:

1. **Meta test**, in `crates/pixfx-effects/tests/<cat>.rs`: `<name_snake>_meta_matches_pixelfx` asserts
   `size`, `origin`, `duration`, `looping`, `fps` and `seed` equal the row in the task's table.
2. Run `cargo test -p pixfx-effects --test <cat>` → FAIL (`<id>` not registered).
3. **Port.** `src/<cat>/lib.rs` gets the `_lib.js` helpers this task's effects import, and only those (as
   `fire/lib.rs` did). `src/<cat>/<name_snake>.rs` holds the effect: a `pub static <NAME_UPPER>` implementing
   `Effect`, its meta and option defaults copied exactly, its header comment kept. A draw-only JS module (no
   `create`) spawns state holding its resolved options; a JS `create(o)` becomes `spawn`, its closure variables
   become state fields (the `Rng` included, seeded from the `seed` argument), its `update(dt)` and `draw(p)` /
   `drawBack(p)` become `EffectState::update` and `draw` (`Layer::Back` is `drawBack`). Register the module in
   `src/<cat>/mod.rs` (new category: also `pub mod <cat>;` in `src/lib.rs`) and the static in `EFFECTS`.
4. Run `cargo test -p pixfx-effects --test <cat>` → PASS.
5. **Parity**, per id: `cargo run -p pixfx-cli -- parity <id> --pixelfx /tmp/fxref/pixelfx` → `PASS`. Open
   `target/parity/<id>.side.gif` once to see it move. A FAIL follows the parity rule in *Global Constraints*.
6. **Determinism.** Add the ids to `IDS` in `crates/pixfx-effects/tests/determinism.rs`, then
   `PIXFX_BLESS=1 cargo test -p pixfx-effects --test determinism`, then
   `cargo test -p pixfx-effects --test determinism` → PASS and
   `wasm-pack test --node crates/pixfx-effects --test determinism` → PASS. Check that the three seed rows of each
   new id differ (Review Focus 5).
7. Repo-wide check passes. **Commit** `effects: <cat> <names>`.

Effect tasks depend on Tasks 1-5 and not on each other, except that two tasks adding to `EFFECTS`, `src/lib.rs`
and `determinism.rs` conflict textually; run them one after another, or in parallel worktrees merged in order.

### Task 6: impacts (first, because it uses most of the sim)

| id | size | origin | duration | loop | fps | seed |
|---|---|---|---|---|---|---|
| `impacts/parry` | 200x112 | 100,96 | 1 | one-shot | 30 | 11 |

`_lib.js` helpers: `star`, `brokenRing`. Painter: `glowRing`, `glowLine` (exist).

Run this task first among the effect tasks: a sim bug shows up here before it spreads to other ports.

### Task 7: explosions

| id | size | origin | duration | loop | fps | seed |
|---|---|---|---|---|---|---|
| `explosions/dirt-explosion` | 260x240 | 130,206 | 3.6 | one-shot | 30 | 3 |
| `explosions/small-explosion` | 300x240 | 150,196 | 5.4 | one-shot | 30 | 7 |
| `explosions/implosion` | 208x184 | 104,92 | 1.65 | one-shot | 30 | 2 |
| `explosions/plasma-explosion` | 224x200 | 112,100 | 1.7 | one-shot | 30 | 7 |

`_lib.js` helpers: `drawDirt`, `crater`, `rock`, `dome`, `pool`, `cull`, `fadeEdges`. `small-explosion`'s
`onLand` (line 59) is the `update_with` closure: it calls `blazes.add(rng, ..)` with the same `rng`, so the
state keeps `ps`, `blazes` and `rng` as separate fields and borrows them separately.

### Task 8: arcane

| id | size | origin | duration | loop | fps | seed |
|---|---|---|---|---|---|---|
| `arcane/rune-circle` | 136x100 | 68,80 | 4 | loop | 30 | 8 |

`_lib.js` helpers: `floorRing`, `glyph`, `mote`. Uses `pal` (Task 1).

### Task 9: critters

| id | size | origin | duration | loop | fps | seed |
|---|---|---|---|---|---|---|
| `critters/wisp` | 128x104 | 64,96 | 3.2 | loop | 30 | 1 |

`_lib.js` helpers: `Swarm`, `shift`, `lnoise`, `tongue`, `firePuff`, `shadow` (and what they call).

### Task 10: fire

| id | size | origin | duration | loop | fps | seed |
|---|---|---|---|---|---|---|
| `fire/spirit-flame` | 80x124 | 40,94 | 3 | loop | 30 | 7 |

Every helper it imports (`noiseFor`, `hotFirst`, `looped`, `drift`, `soft`) is already in `src/fire/lib.rs`;
reuse them.

### Task 11: holy

| id | size | origin | duration | loop | fps | seed |
|---|---|---|---|---|---|---|
| `holy/feather-burst` | 160x160 | 80,60 | 3 | one-shot | 30 | 4 |

`_lib.js` helpers: `Feathers`.

### Task 12: ice

| id | size | origin | duration | loop | fps | seed |
|---|---|---|---|---|---|---|
| `ice/icicle-fall` | 152x120 | 76,100 | 2.6 | one-shot | 30 | 4 |

`_lib.js` helpers: `Panes`, `splitPoly`, `centroid`, `endBy`. Uses `Bits` with `BitKind::Drip`.

### Task 13: scifi

| id | size | origin | duration | loop | fps | seed |
|---|---|---|---|---|---|---|
| `scifi/jetpack` | 96x116 | 48,30 | 2 | loop | 30 | 30 |

`_lib.js` helpers: `Ghosts` (it builds a `Particles` with its own `new Rng(seed ?? 1)` when given no `rng`: the
Rust `Ghosts` owns that `Rng`).

Extra test in `tests/scifi.rs`, `long_play_stays_bounded` (Review Focus 3): an `Instance` of `scifi/jetpack` at its
origin, `update(1.0 / 30.0)` 18000 times (10 minutes), then one `draw` of a full frame finishes in under 50 ms,
same pattern as `huge_campfire_height_draws_in_bounded_time` in `tests/fire.rs`. Write it in step 1 with the
meta test.

### Task 14: lightning

| id | size | origin | duration | loop | fps | seed |
|---|---|---|---|---|---|---|
| `lightning/ball-lightning` | 136x104 | 68,92 | 4 | loop | 30 | 9 |

`_lib.js` helpers: `VOLT`, `PLASMA`, `zigzag`, `stroke`, `crackle`, `flight`, `spark`.

### Task 15: nature

| id | size | origin | duration | loop | fps | seed |
|---|---|---|---|---|---|---|
| `nature/poison-cloud` | 256x100 | 128,86 | 4 | loop | 30 | 3 |

No `_lib.js` helpers; creates `src/nature/lib.rs` only when a later effect needs one.

### Task 16: pickups

| id | size | origin | duration | loop | fps | seed |
|---|---|---|---|---|---|---|
| `pickups/coin-spin` | 56x56 | 28,48 | 1.6 | loop | 30 | 3 |

`_lib.js` helpers: `Layer` (named `Layer` in JS; call the Rust type `CoinLayer` or similar so it does not
shadow `pixfx_core::Layer`), `coin3d`, `glint`, `blobShadow`.

### Task 17: projectiles

| id | size | origin | duration | loop | fps | seed |
|---|---|---|---|---|---|---|
| `projectiles/laser-bolt` | 128x32 | 108,16 | 0.5 | loop | 30 | 2 |

`_lib.js` helpers: `beat`.

### Task 18: shadow

| id | size | origin | duration | loop | fps | seed |
|---|---|---|---|---|---|---|
| `shadow/void-orb` | 168x160 | 84,150 | 3.2 | loop | 30 | 5 |

`_lib.js` helpers: `VOID`, `halo`, `ringLight`, `ringFast`, `unlight`.

### Task 19: spawn

| id | size | origin | duration | loop | fps | seed |
|---|---|---|---|---|---|---|
| `spawn/poof` | 112x92 | 56,80 | 1.6 | one-shot | 30 | 21 |

No `_lib.js` helpers.

### Task 20: transitions

| id | size | origin | duration | loop | fps | seed |
|---|---|---|---|---|---|---|
| `transitions/crt-off` | 320x180 | 0,0 | 2 | one-shot | 30 | 1 |

`_lib.js` helpers: `W`, `H`, `screenMeta`, `clipTo`, `glint`, `rect`. A full-screen effect (`Meta::is_screen`):
the render keeps light up to the edge, as pixelfx does.

`transitions/burn-away` is ported by the docs-site session on branch `burn-away`, which creates the
`transitions` module and its `lib.rs`. This task runs last among the effect tasks, after that branch is merged
into `plan2-sim-core`. It adds `crt-off` to the existing module and only the `lib.rs` helpers still missing.
- The site assumes the screen is fully covered at 1.35 s (pixelfx `COVER`). The port keeps that moment.

### Task 21: water

| id | size | origin | duration | loop | fps | seed |
|---|---|---|---|---|---|---|
| `water/water-jet` | 188x80 | 14,32 | 1 | loop | 30 | 3 |

`_lib.js` helpers: `WATER`, `bead`.

### Task 22: weather

| id | size | origin | duration | loop | fps | seed |
|---|---|---|---|---|---|---|
| `weather/snow` | 320x180 | 160,170 | 12 | loop | 30 | 6 |
| `weather/fireflies` | 320x180 | 160,170 | 12 | loop | 30 | 2 |

`_lib.js` helpers: `clip`, `pass`, `trips`, `rows`. Both are full-screen.

### Task 23: Close-out

- [ ] **Step 1:** README: the crate table's `pixfx-effects` row and the parity paragraph list the effects now
  ported (or say "the effects in `pixfx_effects::EFFECTS`" so the list is not repeated); `CHANGELOG.md` gets one
  entry for the simulation core and the 21 effects.
- [ ] **Step 2:** Run the whole-repo check from the README (`cargo fmt --check && cargo clippy --workspace
  --all-targets -- -D warnings && cargo test --workspace`, then `wasm-pack test --node` for `pixfx-wasm` and
  `pixfx-canvas`) and every effect's parity in one loop:
  `for id in <the 24 ids: the 3 fire ports plus this plan's 21>; do cargo run -q -p pixfx-cli -- parity $id --pixelfx /tmp/fxref/pixelfx || echo "FAIL $id"; done`.
  Expected: no `FAIL` line, or only ids listed in `parity/LEDGER.md` as pending owner approval.
- [ ] **Step 3:** Check that the canvas demo still lists and plays effects (it reads the registry):
  build `pixfx-canvas`'s demo, open it, play `impacts/parry` and `weather/snow`.
- [ ] **Step 4: Commit** `docs: plan 2 close-out`.
