# Motion and Sweep Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Everything that moves leaves its streak and reacts to its motion through one shared formula: a brush swept along poses, and motion read from a trace.

**Architecture:** `pixfx-draw::sweep` paints a `Brush` (cells across the motion) along `Pose`s, coloured by age, gap-free. `pixfx-core` reads motion off a `Trace` (now with angles) and turns it into poses. `projectiles/trail` becomes a sweep; brazier and campfire bend with their motion when the game asks for it.

**Tech Stack:** Rust (pixfx-draw, pixfx-core, pixfx-scene, pixfx-wasm, pixfx-effects), TypeScript (pixfx-web untouched), site JS.

**Spec:** `docs/superpowers/specs/2026-11-16-motion-sweep-design.md`

**Execution:** native, in the session that wrote it (the owner said "implement").

## Global Constraints

- clippy `unwrap_used`, `expect_used`, `panic` denied, `unsafe_code` forbidden; maths through `libm` and `pixfx_draw::math`.
- A port standing still, or not asked for its motion, paints exactly as now: `frame_hash` unchanged for every effect but `projectiles/trail` (new, no golden yet) and `arcane/flying-sword` (already changed this session).
- `pixfx-draw` depends on nothing new; `pixfx-core` does not re-export `pixfx-draw` items.
- Each effect builds and tests alone behind its feature.
- No commit until the owner asks; own hunks only.

## Review Focus

- A trace of one point, or a still mover: no NaN, nothing drawn, no panic (Task 2, Task 3 tests).
- A long bar turned fast between two poses (half a turn in one step): no holes (Task 3 `a_bar_turned_about_its_end_covers_a_fan`).
- A hit-stop (`dt = 0`, two poses with the same age): velocity is not divided by zero (Task 2 `same_age_points_give_no_velocity`).
- A flipped trail with angles: the angle mirrors with x (Task 1 scene test).
- `Brush::from_column` on a column with no opaque pixel: an empty brush, sweep draws nothing (Task 3).

---

### Task 1: Angles on the trace

**Files:**
- Modify: `crates/pixfx-core/src/trace.rs` (`TracePoint.angle`), `crates/pixfx-scene/src/lib.rs` (`Trail.seen` keeps the mover's `place.angle`; `follow` writes it, mirrored like x when the trail draws flipped: `PI - a`), `crates/pixfx-wasm/src/scene.rs` (`PaintJob.trace` flattened by 4: x, y, angle, age), `crates/pixfx-effects/src/projectiles/trail.rs` (`straight`), `crates/pixfx-core/tests/trace.rs`
- Test: `crates/pixfx-scene/tests/trail.rs`

**Interfaces:** Produces `TracePoint { x, y, angle, age }`.

- [ ] The scene's traces are private; the angle is pinned by Task 4's `a_trail_that_turns_with_its_mover_sweeps_an_arc` (the trail drawn through the scene). Here: existing tests still pass with the field added.
- [ ] Add the field everywhere it is built; run `cargo test -p pixfx-core -p pixfx-scene`, `cargo clippy -p pixfx-wasm --all-targets -- -D warnings`. Expected: pass.

### Task 2: Motion (`pixfx-core`)

**Files:** Create `crates/pixfx-core/src/motion.rs` (methods on `Trace`); modify `lib.rs` (`pub mod motion`); test `crates/pixfx-core/tests/motion.rs`.

**Interfaces:** Produces on `Trace`:
- `fn at(&self, age: f64) -> Option<(f64, f64, f64)>` (x, y, angle, interpolated; `None` for an empty trace; clamped to the oldest point);
- `fn velocity(&self, age: f64, window: f64) -> (f64, f64)` (central difference over `[age - window/2, age + window/2]` clamped to the trace; `(0, 0)` when the span's ages are equal or the trace has under two points);
- `fn speed`, `fn heading(&self, age, window) -> Option<f64>` (`None` below 1e-6 px/s);
- `fn turn(&self, age, window) -> f64` (heading change per second, shortest way), `fn spin(&self, age, window) -> f64` (angle change per second);
- `fn acceleration(&self, age, window) -> (f64, f64)`;
- `fn poses(&self, secs: f64) -> Vec<pixfx_draw::sweep::Pose>` (every point up to `secs`, the cut one interpolated, oldest last).

- [ ] Tests: `a_straight_trace_has_its_speed_and_heading` (points 2 px apart every 0.01 s along +x: speed 200 ± 1e-6, heading 0), `a_circle_turns_at_a_constant_rate` (points on a circle at 2 rad/s: turn 2 ± 0.05 at three ages), `a_still_trace_has_no_motion` (speed 0, heading None), `same_age_points_give_no_velocity`, `at_interpolates_between_points`.
- [ ] Implement; `cargo test -p pixfx-core`, clippy. Expected: pass.

### Task 3: Brush and sweep (`pixfx-draw`)

**Files:** Create `crates/pixfx-draw/src/sweep.rs`; modify `lib.rs`; test `crates/pixfx-draw/tests/sweep.rs`.

**Interfaces:** Produces
- `pub struct Pose { pub x: f64, pub y: f64, pub angle: f64, pub age: f64 }` (newest first in a slice);
- `pub struct Cell { pub across: f64, pub color: Rgba, pub light: f64 }`;
- `pub enum Turn { Heading, Angle(f64) }` (the column square to the way it went, or at the pose's angle plus the base);
- `pub struct Brush { pub cells: Vec<Cell>, pub turn: Turn }` with `Brush::column(palette: &[Rgba], width: f64) -> Brush` (`width` rounded to cells, centre first colour, edges one step darker) and `Brush::from_column(frame: &Frame, x: u32, origin_y: i32) -> Brush` (opaque pixels of column `x`, `across` measured from row `origin_y`, light from `frame.light`, `Turn::Angle(0.0)`);
- `pub struct Look { pub fade: f64` (palette steps moved by age 1), `pub taper: f64` (width at age 1, 0..1 of the head's), `pub wave: f64`, `pub phase: f64`, `pub breakup: f64` (age where dither starts), `pub light: f64`, `pub skip: f64` }`;
- `pub fn sweep(p: &mut Painter, poses: &[Pose], brush: &Brush, ramp: &[Rgba], look: &Look)`: between consecutive poses (oldest pair first), for each pair of neighbouring cells fill the quad they sweep, sub-stepped so no cell moves more than 1 px a sub-step; colour = the cell's colour moved `fade * age` steps along `ramp` (`ramp` empty: the cell's own colour); cells beyond `taper`'s width at that age skipped; dither (4x4 bayer, as `trail.rs` today) above `breakup`; light `look.light * cell.light * (1 - age)` into the light layer at the path's pixel.

- [ ] Tests: `a_point_swept_along_a_line_covers_the_line` (one cell, poses (0,0)->(20,0): exactly x 0..=20 at y 0 lit), `a_column_swept_straight_covers_a_band` (3 cells: rows -1..=1 over x 0..=20), `a_bar_turned_about_its_end_covers_a_fan` (cells across 0..=15, `Turn::Angle(0)`, poses at one point turning 0 -> PI/2 in one step: every pixel within radius 14 in that quarter is painted), `the_newest_pass_is_on_top` (two crossing passes, the crossing pixel has the newest colour), `an_empty_brush_or_one_pose_draws_nothing`, `from_column_reads_a_frames_column`.
- [ ] Implement; `cargo test -p pixfx-draw`, clippy. Expected: pass.

### Task 4: The trail on the sweep

**Files:** Modify `crates/pixfx-effects/src/projectiles/trail.rs`.

**Interfaces:** Consumes `Trace::poses`, `Brush::column`, `sweep`, `Look`. New option `turn` (0..1, default 0: square to its way; 1: turns with the mover's angle).

- [ ] Test in `crates/pixfx-scene/tests/trail.rs`: `a_trail_that_turns_with_its_mover_sweeps_an_arc`: a mover standing still while its angle goes 0 to PI/2 over 10 steps, trail `turn: 1`: the trail's lit box is at least 8 x 8 (an arc), while with `turn: 0` it draws nothing (it never moved).

- [ ] Render the flying sword's trail before (`/tmp` copy of today's gif via `cargo run -p pixfx-export --example gif -- projectiles/trail --scale 4`) and after; they read alike (head white, steel end, waved, dithered tail).
- [ ] Scene trail tests and `cargo test -p pixfx-effects --no-default-features --features projectiles-trail --all-targets` pass; same for `arcane-sword-formation`.

### Task 5: Fire bends with its motion

**Files:** Modify `crates/pixfx-effects/src/fire/lib.rs` (`pub fn carried(trace: Option<&Trace>) -> (f64, f64)`: lean and wind added from the velocity over 0.1 s, zero without a trace), `fire/brazier.rs`, `fire/campfire.rs` (option `carried` 0/1, default 0; `traces` returns `Some(0.15)` only when it is 1, so a still or uncarried fire shares frames and keeps its pixels).

- [ ] Test in `crates/pixfx-effects/tests/` (feature-gated): `a_carried_campfire_leans_against_its_motion`: a scene plays `fire/campfire` with `carried: 1`, moves it right 3 px a step for 10 steps; its lit pixels' mean x above the base is left of the same fire played still.
- [ ] `cargo test -p pixfx-effects --test determinism -- --ignored` with all effects: every hash unchanged except the two named in Global Constraints.

### Task 6: Docs, skills, site, goldens

- [ ] CHANGELOG, crate AGENTS.md (draw: sweep; core: motion, trace angle; scene: trail on its own, angles; wasm: trace by 4), skills `pixfx-effect` (a streak is a sweep; motion from the trace), `pixfx-play` (play a trail on its own and move it).
- [ ] Site: the trail page's sliders gain `turn`; rebuild; check in the browser.
- [ ] Show the owner the trail and flying-sword gifs; bless after their yes.
