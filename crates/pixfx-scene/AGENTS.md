# pixfx-scene

Layer: **Scene**. Read the root `AGENTS.md` ("The boundary" and "Layers and where code goes") before changing
anything here; this file only says what this crate holds.

## Job

What a stage shows and where, decided once in Rust for every shell. `Scene` holds the effects playing, their
placement and margins, the scale, the order their layers are drawn in, when finished ones go, which frames are drawn
and shared (under `FRAME_BUDGET`), the game's answer to their cues (a `Feel`), and events (`StageEvent`).

It drives the `Shell` contract, which every shell implements:

- `make(frame)`: a new surface holding a frame;
- `refill(surface, frame)`: a new frame in a surface;
- `show(pass, surface)`: draw a surface at `pass.rect`, colour normally, then light additively;
- `free(surface)`: let go of a surface the scene dropped.

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

- Required: `pixfx-core`, `pixfx-draw`.
- Optional, behind `serde` / `ts` only: `serde`, `tsify`, `wasm-bindgen`.
- Never `js-sys`, `web-sys`, gpui, or any shell crate. Not `pixfx-effects`: a scene plays any `Effect` it is given.

## Traps

- An effect stopped while following a moving sprite keeps fading at the sprite's last position.
- A hit-stop (`dt = 0`) holds every frame; nothing advances.
- Past `FRAME_BUDGET`, least recently shown frames are dropped and their surfaces freed; a shell's surface count
  returns to 0 after every effect ends.
- After `setScale`, no surface is shown at its old size.

`tests/scene.rs` covers the hit-stop (`an_effects_hit_freezes_every_effect_and_tells_the_host_how_long`) and the
budget (`frames_dropped_over_the_budget_come_back_the_same`, `every_surface_a_scene_drops_is_freed`,
`clearing_a_scene_frees_everything_it_holds`). Follow-then-stop and `setScale` have no test yet (Task 3 of
`docs/superpowers/plans/2026-11-14-render-engine-boundary.md`); a change that touches them adds the test first.

## Before you finish

```
cargo test -p pixfx-scene
cargo test --workspace
cargo clippy --manifest-path crates/pixfx-gpui/Cargo.toml -- -D warnings
```
