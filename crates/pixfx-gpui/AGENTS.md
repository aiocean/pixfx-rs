# pixfx-gpui

Layer: **Shell** (gpui). Read the root `AGENTS.md` ("The boundary" and "Layers and where code goes") before changing
anything here; this file only says what this crate holds.

Its own workspace: it needs the gpui fork at `../zed-gpui` (relative to the repo, `tools/zed-gpui`), and the root
`cargo` commands do not build it.

## Job

Shows pixfx in a gpui window: uploads frames as gpui images (nearest-neighbour, colour normal, light additive) and
paints them where it is told. This is the shell a Rust game (a native mobile app) uses.

## A shell may only

Create and free gpui images, copy pixels Rust painted into them, place them where `Pass` says, cache uploads, and
hand Rust the elapsed time.

## Does not belong here

| If the code... | It goes to |
|---|---|
| decides placement, layout, frame budget, which frame shows, fade after stop | `pixfx-scene` |
| a feel or effect rule | `pixfx-core` / `pixfx-effects` |
| paints or composites pixels | `pixfx-draw` |

## Known drift

`FxView` (over `Player`) and `FxFrames` keep their own layout and frame store instead of a `Scene` driven through a
gpui `Shell`. That is item 1 of "Out of place today" in the root `AGENTS.md`. Do not grow `FxView` or `FxFrames`;
new behaviour goes into `Scene`, and this crate moves onto it.

## Dependencies

`gpui` (the local fork), `image`, `anyhow`, `pixfx-core`, `pixfx-draw`; `pixfx-scene` once it moves onto `Scene`.

## Before you finish

```
cargo clippy --manifest-path crates/pixfx-gpui/Cargo.toml -- -D warnings
cargo test --manifest-path crates/pixfx-gpui/Cargo.toml
```
