# pixfx-scene

Layer: **Scene**. Read the root `AGENTS.md` ("The boundary" and "Layers and where code goes") before changing
anything here; this file only says what this crate holds.

## Job

What a stage shows and where, decided once in Rust for every shell. `Scene` holds the effects playing (each a
`Clip` and its `Clock`), their placement and margins, the scale, the order their layers are drawn in, when finished
ones go, which frame each shows, which effects the host can see (a `View` with the camera's scroll, and each
effect's scroll factor: the rest play on undrawn), the character each effect plays on (`set_actor`, kept in the
effect's own space and mirrored with its flip), the game's answer to their cues (a `Feel`), and events
(`StageEvent`). It carries out orders (`order`): an effect that takes one answers with a volley
(`pixfx_core::volley`), and the scene plays each shot when it is due, flies it along its path turned to its heading
(snapped to 32, so frames are shared), sends a `Strike` event and plays the hit effect where it strikes, and ends
the ordered effect after its last shot. A piece flies on past the point and strikes the first body the game
declared (`set_body`, `Body`; its caster's spared, a piercing piece through every one) or where the game bursts it
(`burst`). A trail (`trail(mover, clip)`, an effect whose `traces` asks for it) is handed its mover's trace, in its own
space (turned and mirrored back), after every update; an effect that asks for a trace played on its own trails
itself as the game moves it, and ends once stopped. `follows(handle)` names a trail's mover. `launched(handle)` names what a volley launched, for a host that paints
each effect on its own (gpui).

`Frames` is game-wide and shared by every `Scene` of the game (a canvas, a Phaser scene, a gpui window each, made
with `Frames::scene`): the shared frames under one budget (`FRAME_BUDGET`) and the `Painter` that paints them. A
frame one scene painted shows in every scene. The host ends each of its frames with `Frames::end_frame` once every
scene drew.

A `Painter` paints a `Job` (an effect's state brought to a frame, drawn placed and faded): `InlinePainter` on the
spot (Canvas 2D, gpui, files, tests), or another painter elsewhere (a worker), whose frames come back through
`Frames::deliver` (shared) and `Scene::deliver_own` (the frames an effect has alone: fading after stop, or wrapping
its actor). Both take a `Frame`, or whatever the
shell takes from its painter through `Receive<D>` (a JS shell takes the shape of the RGBA a worker sent it, so pixels
skip the scene). Until then the effect shows its last frame. The painter keeps each effect's state (`Sim`) between paints; `Scene` keeps only clocks, so deciding stays
cheap on the thread that runs it.

It drives the `Shell` contract, which every shell implements:

- `make(frame)`: a new surface holding a frame;
- `refill(surface, frame)`: a new frame in a surface;
- `show(pass, surface)`: draw a surface at `pass.rect`, colour normally, then light additively;
- `free(surface)`: let go of a surface the scene dropped.

Every draw call takes the `Frames` it draws from: `scene.draw(&mut frames, &mut shell)`, then
`frames.end_frame(&mut shell)`.

## Belongs here

Every decision a shell would otherwise make: placement, layering, follow, fade after stop, lifetimes, which frame to
show, which surface to free, scale changes. When two shells would write the same rule, it comes here.

## Does not belong here

| If the code... | It goes to |
|---|---|
| creates a texture, image, canvas or DOM element, or copies pixels into one | a shell, through `Shell` |
| runs a loop, reads a clock, uses workers | a shell; it hands `Scene` the elapsed time |
| is a `wasm-bindgen` type or JS option parsing | `pixfx-wasm` |
| paints pixels | `pixfx-draw` (or the engine crates) |
| is one effect or a feel rule | `pixfx-effects` / `pixfx-core` |

A new method on `Shell` is a contract change for every shell (Canvas 2D, Phaser, gpui, and the next ones). Add one
only when every shell needs it, and say so in the commit.

## Dependencies

- Required: `pixfx-core`, `pixfx-draw`, `libm` (a trail's trace turned into its own space, the same bits natively and in wasm).
- Optional, behind `serde` / `ts` only: `serde`, `tsify`, `wasm-bindgen`.
- Never `js-sys`, `web-sys`, gpui, or any shell crate. Not `pixfx-effects`: a scene plays any `Effect` it is given.

## Traps

- An effect stopped while following a moving sprite keeps fading at the sprite's last position.
- An effect that wraps its actor paints its own frames (no sharing): each `set_actor` that changes the sprite
  relative to the effect bumps a version (`OwnAt::actor`) and draws it again; the same sprite set again changes
  nothing, so a host may set it every frame. A painter elsewhere gets the sprite in the `Job` (`Job::actor`).
- A hit-stop (`dt = 0`) holds every frame; nothing advances.
- Past `FRAME_BUDGET`, least recently shown frames are dropped and their surfaces freed; a shell's surface count
  returns to 0 after every effect ends.
- After `setScale`, no surface is shown at its old size.
- A frame painted elsewhere is asked of the painter once, however many effects or scenes wait for it; each shows its
  last frame until the frame is delivered. `Frames::end_frame` runs once per host frame, after every scene drew:
  ending it per scene would drop a frame another scene shows in the same host frame.
- A painter elsewhere has at most one frame in flight per layer of an effect (`Entry::awaiting`, `asked` for its own
  frames): while it is out, the effect asks for no newer one, so a slow worker never builds a backlog. A frame that
  comes back after the clock moved on still shows once, then the current one is asked for.

`tests/scene.rs` covers the hit-stop (`an_effects_hit_freezes_every_effect_and_tells_the_host_how_long`) and the
budget (`frames_dropped_over_the_budget_come_back_the_same`, `every_surface_a_scene_drops_is_freed`,
`clearing_a_scene_frees_everything_it_holds`); `tests/frames.rs` covers frames shared by several scenes and painters
that paint elsewhere, including one that falls behind
(`a_painter_that_falls_behind_is_asked_for_one_frame_per_layer_at_a_time`,
`a_frame_that_comes_back_after_its_clock_moved_on_still_shows`). Follow-then-stop and `setScale` have no test yet (Task 3 of
`docs/superpowers/plans/2026-11-14-render-engine-boundary.md`); a change that touches them adds the test first.

## Before you finish

```
cargo test -p pixfx-scene
```
