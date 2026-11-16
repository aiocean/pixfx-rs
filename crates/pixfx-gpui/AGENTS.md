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

## Shape

`FxScene` wraps one `Scene` per window and is the gpui `Shell`: `play` an effect, `tick` the scene, `paint` each
effect at its view's origin (`Scene::draw_handle`), then `end_frame` once per window frame (`Scene::end_frame`, then
`Frames::end_frame`). It owns its `Frames` and their painter: `InlinePainter` on the spot (`FxScene::default()`), or
`ThreadPainter` (`src/threads.rs`, `FxScene::with_threads(n)`, not on wasm), which keeps each effect's state on one
thread for its life and sends back ready images (`Painted`), taken at the first `paint` of the next window frame
(`Receive<Painted>`).
An effect that takes orders: declare bodies with `set_body`, `order(fx, x, y, aim, caster)` (from its origin), paint
each of `launched(fx)` where `fx` is painted, and read strikes from `events()`; `burst` bursts a piece. `trail(fx,
id, opts)` lays a trail under a moving effect.
Images the scene lets go of are queued and dropped from the atlas at `end_frame`, never mid-frame. New behaviour goes
into `Scene`, not here.

## Dependencies

`gpui` (the local fork), `image`, `anyhow`, `pixfx-core`, `pixfx-draw`, `pixfx-scene`.

## Traps

- With threads, an effect's state lives on one thread for its life; its `End` goes on the same channel after its
  last job, so the thread paints the jobs before it lets go.
- Nothing shows on an effect's first window frame with threads: its frame comes back for the next one.

## Before you finish

```
cargo clippy --manifest-path crates/pixfx-gpui/Cargo.toml --all-targets -- -D warnings
cargo test --manifest-path crates/pixfx-gpui/Cargo.toml
# speed claims: both measurements, in release
cargo test --release --manifest-path crates/pixfx-gpui/Cargo.toml -- --ignored --nocapture
```
