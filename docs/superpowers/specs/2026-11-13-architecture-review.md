# pixfx-rs architecture review

Date: 2026-11-13 · Asked by the owner: "review the whole architecture — packages, components, the ways of rendering,
the core — and judge whether it is lean, easy to compose, follows the Unix philosophy and functional programming".

State reviewed: `main` 0c537b6, re-checked on `main` 439a139. Branch status as of `composable` 39603bf and `plan3-hit`
df15f9c. Method: 12 reviewers (9 areas, 3 cross-cutting lenses), 88 findings, each checked against the code by
skeptics (facts, and Chesterton's fence for medium and high). Most held but were overstated; 7 were refuted. The
key claims were re-read by hand on 439a139.

## Verdict

| Principle | Score | Verdict |
|---|---|---|
| Functional | 4/5 | Yes in the Rust core; a few slips in the JS hosts. |
| Composable | 3/5 | Parts compose well inside Rust; the seams to the hosts are where it breaks. |
| Unix | 3/5 | One job per crate, mostly; knowledge kept in code instead of data. |
| Lean | 3/5 | Small duplication (two or three copies of one rule); wide public surface. |

The core is sound. The problems sit at the seams between the core and the hosts, and in facts written in code
instead of data. Nothing calls for a rewrite; every fix below is small and local.

## The parts

```
pixfx-core (Effect, Instance, Frame, Painter, sim)          deps: libm only
  ├─ pixfx-effects (25 effects)    pixfx-fighter (body, feel, paint, status)
  ├─ pixfx-export ← pixfx-cli          (1) render_frames → PNG / JSON / GIF
  ├─ pixfx-wasm: Fx, Player*, parse_options
  │     ├─ packages/pixfx-phaser       (2) checked-in wasm build, worker pool
  │     └─ pixfx-canvas: Stage/Scene, Body/Feel/Sprites, re-exports Fx
  │                                    (3) the one wasm the site loads
  └─ pixfx-gpui (own workspace)        (4) its own copy of Player's layout
* Player is host-neutral Rust that lives in the binding crate.
```

Paths 1, 2 and 4 each work out the "effect on its own canvas" layout (size + 2 × margin, corner, origin, edge
fade): `render.rs:30-42`, `player.rs:26-34,83`, `pixfx-gpui/src/lib.rs:120-128`. Commit 7cea253 (`Effect::canvas`)
had to change all three.

## What is right

- Functional core: no static mutable state, clock or OS randomness in pixfx-core; its only dependency is libm.
  `update(&mut self, dt)` is the only state change and `draw(&self, …, t)` only reads. `render_frames` is pure.
- `Frame` is plain data and `Painter` only borrows it; pixfx-fighter draws with a bare Painter (`status.rs:287`).
- Randomness is a value (`Rng { s: u32 }`, counter `rand`) passed in by the caller; the fighter takes `roll` from the
  caller and returns events as enums.
- `Scene<T>` is pure and tested natively; `Stage` only blits. `parse_options` is strict and names the bad field.
- CLI: exit codes 0/1/2 tested, errors on stderr; pixfx-export depends only on pixfx-core; compare, pack and
  beanfx_json are pure.
- Effects share one skeleton; the 16 loops are closed-form in t. The per-category `lib.rs` mirrors pixelfx
  `_lib.js` on purpose.

## Findings

Status: **open** (nowhere yet), **main** (fixed on main), **composable** (fixed on the `composable` branch, not merged).

### Medium

1. **Effect options are code, not data; a live bug.** `Meta` has no option list; `parse_options` sends unknown keys
   to `extra`; `num_or` falls back to its default when a key is missing or misspelt. `duel.js:387` passes
   `{ size: 0.6 }` to `explosions/dirt-explosion`, which reads only `palette` and `fire` (`dirt_explosion.rs:83-84`),
   so the size is dropped without a word. The site copies campfire's height default (`effect.js:22`).
   Fix: declare each option once in `Meta` (name, kind, default), read defaults from it, reject undeclared keys in
   `parse_options`, expose the list through `meta(id)`. **Open** (`composable` still accepts any extra key).
2. **A playing effect's placement is frozen at spawn.** `Instance` keeps `opts` private and reads x, y, flip,
   angle and light only in `draw`. **composable** adds `Instance::set(Placement)`; see "Against the composable
   spec" for the form to use instead.
3. **Player is host-neutral Rust in pixfx-wasm; its layout is written three times.** Fix: move Player (with
   `pivot()` and the colour/light RGBA output) into pixfx-core beside `render_frames`, re-export it from pixfx-wasm,
   have gpui's FxView wrap it. `render_frames` keeps its own unclamped loop. **Open.**
4. **The fighter bindings decode JS loosely.** A missing field becomes 0, an unknown `how` becomes Palm, an unknown
   dome becomes Banded, a missing `right` clamps the arena at −6 (`fighter.rs:145-156, 440-452, 623-626,
   781-787`), while the same file rejects an unknown look or pose. **composable** (3ef23fd: serde,
   `deny_unknown_fields`).
5. **The body's numbers are copied into duel.js** (`duel.js:14-26,80` = `body.rs:15-76`), with no test tying them.
   **composable** for all but `KNOCKBACK`, which `duel.js:71` still copies there.
6. **Phaser's worker pool merges time steps while a lane is busy** (`pool.ts:48` `i.dt += dt`; `worker-core.ts:38`
   runs one step). A late worker takes different sub-steps and random draws than inline mode, so the README's
   promises at lines 61 and 85 do not hold. Fix: queue `steps: number[]`, update per step, draw once; test with a
   port that answers late, byte for byte against InlineBackend. **Open.**
7. **pixfx-canvas holds every JS binding**: Stage, Body, Feel, Sprites, and Fx/effects/meta re-exported through its
   pixfx-wasm dependency without saying so in its source. `composable` adds `light.rs`; plan 3 adds Creature,
   Performance and `Feel.take`. One wasm file for the site is a fair product choice; the cost is pure logic
   drifting into the binding (`Sprites::portrait`, a pure pixel loop, is still in `fighter.rs`). Fix: write
   `pub use pixfx_wasm::{Fx, effects, meta}` explicitly; move portrait into `pixfx_fighter::paint`. **Open.**

### Low

8. `EffectState::draw` takes the three-valued `Layer`; `Both` never reaches it, which leaves 5 dead match arms in 4
   styles. Fix: `fn draw_back(&self, p, t) {}` (empty default) and `fn draw(&self, p, t)`. **Open.**
9. `parse_options` mixes in Fx's canvas policy (margin; the x/y error names Fx), and Stage works around it
   (`options.rs:18-54`, `stage.rs:16-27`). Fix: return the host keys it saw as data and let each host decide.
   **Open.**
10. Phaser's zod schema copies the engine's options and has drifted (negative `light`: TS rejects, Rust accepts).
    Fix: keep zod for Phaser's own keys (scrollFactor, depth, margin); the engine validates the rest. **Open.**
11. A worker error leaves its lane busy forever and its effects freeze silently (`pool.ts:149-152`: the throw
    comes before `lane.busy = false`). Fix: mark the lane failed and end its effects with an event. **Open.**
12. `drawBars` drains the health bar inside the draw call, so the drain follows the display's refresh rate
    (`duel-art.js:124-125,163-164`). Fix: advance it with dt. **Open.**
13. The pixfx-rs design says live play steps a fixed 1/120 s; the code follows pixelfx's dt / ceil(dt × 120)
    sub-steps on purpose, so live results depend on the dt sequence. Fix the text (design spec 142-144, plan 1
    line 34, `duel.js:272`), not the code. **Open.**
14. Public surface: the 17 sim re-exports at the pixfx-core root have no user (`lib.rs:23-26`); the 15 category
    modules are `pub mod lib` (about 105 items) with one outside user, a test. Fix: drop the re-export block; make
    the libs `pub(crate)` unless cross-crate effect registration is designed first. **Open.**
15. No `pixfx list`, so a parity sweep needs ids typed by hand. Fix: print one id per line from `EFFECTS`.
    Also small: `pixfx --help` exits 2; `render` requires `--gif`, its only format. **Open.**
16. `Feel` assumes two fighters (`feel.rs:294` `1usize.wrapping_sub(by)`, `jolts() -> [_; 2]`). No caller has
    more; plan 3 uses `Feel::take`. Leave until a game with three fighters uses `Feel.react`. **Open, deferred.**
17. Small copies: the 9-colour fire ramp in 4 effects; the `Life` trait with 15 one-line impls; pixfx-fighter's
    private `js_sign` and `sign_or_one` in `body.rs` (core has both now). **Open.**

Fixed on main during the review: JS-number helpers in core (04817af), determinism over `EFFECTS` (3e9cbe0), gpui's
margin drift (7cea253). Fixed on `composable`: one canvas-exact blend (d4a0734); the fighter's two blends differed
by 1 on 125 (value, alpha) pairs.

Refuted, no change: moving the 0.25 s stall clamp out of `Instance` (a planned hang guard, plan 1 lines 49-50);
drawing particles without a `Particles` container (mirrors pixelfx); a generic paint pipeline for other sprites;
Phaser's module-level engine; a frame-unpack helper in core (the light encodings differ on purpose); Stage's own
loop (documented API); the CLI's second GIF writer (two copies, below the rule of three); splitting the site's code
per page.

## Against the composable spec and plan 3

- Covered by the composable spec: findings 2, 4, 5, the one blend, Stage events as data, pixfx-light.
- Missing from it: 1, 3, 6, 8, 13.
- **Placement as a draw argument, not a setter.** `Instance::set(Placement)` stores placement as mutable state.
  Plan 3's chain draws its parts from `&self` and works out each part's position at draw time
  (`plans/2026-11-13-plan3-hit.md:250-252`); it cannot call a `&mut` setter. `Instance::draw(&self, frame, layer,
  place)` serves both: Stage keeps each handle's placement as data and passes it in. Plan 3 also introduces
  `chain::Place`; pick one name for both.
- **Decide where bindings live before plan 3 adds three more to pixfx-canvas** (finding 7).

## Order of work

1. On `composable`: switch Placement to the draw-argument form, then merge after its gates (fight trace, goldens,
   Chrome).
2. Effect options in `Meta` and strict `parse_options` (1). Gate: goldens and parity unchanged; a misspelt option
   errors.
3. Player into pixfx-core (3). Gate: the existing Player-equals-render_frames test.
4. `draw_back` (8): mechanical across the 25 effects; goldens.
5. Phaser: steps list and failed lanes (6, 11).
6. The rest (9, 10, 12-15, 17).

## Open decisions for the owner

- Placement: draw argument (recommended) or setter.
- pixfx-canvas: one bundle for every binding (say so in its description and re-exports), or pure-state bindings
  (Body, Feel, plan 3's Creature and Performance) as a feature of pixfx-wasm so Phaser can use them too.
