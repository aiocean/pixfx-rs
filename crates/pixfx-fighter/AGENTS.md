# pixfx-fighter

Layer: **Engine**. Read the root `AGENTS.md` ("The boundary" and "Layers and where code goes") before changing
anything here; this file only says what this crate holds.

## Job

Pixel fighters, from the site's duel: a drawn fighter with a pose for each thing it can be doing (`paint`, `pose`,
`look`; poses traced from art in `traced`, tables written by `pixfx trace`), a body that takes hits the way fighting
games do (`body`, `act`), the screen's answer to a blow (`feel`) and the marks that hang on a fighter (`status`).

## Belongs here

- How a fighter looks, is posed and painted, how its body reacts to a hit, which cues a blow fires.

## Does not belong here

| If the code... | It goes to |
|---|---|
| is the rules of a fight: moves, damage, AI, turns | the game |
| is a feel rule every game shares, not only fighters (hit-stop, shake, squash) | `pixfx-core` |
| is a drawing primitive (outline, blend, palette) | `pixfx-draw` |
| decides where a fighter is drawn on screen or on which layer | `pixfx-scene` |
| is a JS binding of these types | `pixfx-wasm` |

Build on `pixfx-core`'s hit-stop, shake and `Feel`; do not write a second copy here.

## Dependencies

- Required: `pixfx-core`, `pixfx-draw`, `libm`.
- Optional, behind `serde` / `ts`: `serde`, `tsify`, `wasm-bindgen`.
- Never `pixfx-scene`, `pixfx-wasm` or a shell.

## Traps

- Pure like `pixfx-core`: no I/O, no clock; randomness comes in from the caller.
- `dt = 0` (hit-stop) holds every body still.

## Before you finish

```
cargo test -p pixfx-fighter
cargo test -p pixfx-cli --test poses                          # the pose sheet paints every pose
cargo test --workspace
```
