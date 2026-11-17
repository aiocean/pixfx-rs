---
name: pixfx-perf
description: Use when measuring or improving pixfx speed - an effect that is slow to simulate or paint, a game dropping frames with many effects, Phaser frame time on a phone, gpui painting cost, wasm size versus speed - or before claiming a change is faster.
---

# Measure and improve pixfx speed

"Fast" is a measurement: a change that claims speed shows the measurement **before and after**, in the commit
message. Each part measures itself. Pick the measurement of the layer you change.

| You change | Measure with |
|---|---|
| One effect's simulation or drawing | effects bench, that effect built alone |
| `pixfx-core`, `pixfx-draw`, a category's shared `lib.rs` | effects bench, every effect |
| A JS shell or `pixfx-web` (upload, atlas, workers, batching) | Phaser `bun run perf` |
| `pixfx-gpui` | gpui release measurements |

## Effects bench (the engine)

```sh
cargo run --release -p pixfx-effects --no-default-features --features feedback-outline --example bench   # one effect
cargo run --release -p pixfx-effects --example bench -- --ids feedback/outline,projectiles/trail --n 100      # chosen ids
cargo run --release -p pixfx-effects --example bench                                                 # every effect
```

Flags: `--ids`, `--n` (instances, default 50), `--secs` (default 10), `--hz` (default 60). It prints the clocks'
update cost for all ids together, then a markdown table with a row per id (canvas, frames drawn, draw and RGBA cost), drawn the way a host draws (only on a new frame, both layers, cropped). Always
`--release`; a debug number means nothing. The first release build takes minutes (fat LTO); don't run two cargo
commands on one target dir at once, they wait on its lock. `cargo test -p pixfx-effects --example bench` runs its own tests.

## Phaser (the JS path on a phone)

```sh
cd packages/pixfx-phaser
bun run build:wasm                                  # when Rust changed
PIXFX_PERF_COUNTS=10,50,100 bun run perf            # before your change, then again after
bun run perf:check                                  # pixels and culling still right
```

It sweeps effect counts at 1× and 4× CPU throttle (4× is a mid-range phone), with painting on the main thread and on
workers, and profiles one load each. Vite serves `src/` live: run it from a git worktree no other session edits,
or a half-saved file stops the page. Compare the same counts and throttle before and after.

## gpui

```sh
cargo test --release --manifest-path crates/pixfx-gpui/Cargo.toml -- --ignored --nocapture
```

Needs the gpui fork in game-factory's `tools/zed-gpui`, beside `pixfx-rs` (not inside it). `FxScene::with_threads(n)` moves painting off the window's thread.

## Where speed work goes

- **Painting and simulation** (fewer ops per pixel, cheaper particles, skipping empty rows): the lower layers
  (`pixfx-effects`, `pixfx-core`, `pixfx-draw`). It must keep every `frame_hash`:
  `cargo test -p pixfx-effects --test determinism -- --ignored`. A faster effect with moved hashes is a different
  effect: don't bless the new hashes to make the test pass.
- **Showing** (surface caches, atlas, batching, worker hand-off, GPU upload): the shell or `pixfx-web`. The engine's
  decisions stay in Rust.
- The architecture already holds: frames shared game-wide and painted once, effects off screen paint nothing, workers
  paint and transfer bytes that reach the main thread once, WebGL shells show from one atlas. No layer groups.
  Work with it, not round it.
- Release builds keep `opt-level = 3` with fat LTO (`Cargo.toml` `[profile.release]`). A game cuts wasm size by
  building only its effects (`bun run build:wasm --no-default-features --features pixfx-effects/feedback-outline`: the id with `/` as `-`), never by
  `opt-level = "s"`/`"z"`. Trading speed for bytes needs the owner's yes.
- A game with many effects at once: `setDrawBudget(n)` (JS) / `set_draw_budget` (gpui) caps layers drawn per frame.

## Common mistakes

| Mistake | Fix |
|---|---|
| Benching in debug | `--release` |
| One number after the change, none before | Run the same measurement on the unchanged tree first |
| `bun run perf` in the shared tree while others edit | A worktree |
| Hashes moved by an optimisation, blessed | Find the arithmetic that changed; keep the same operations and order |
| Rebuilding every effect to bench one | `--no-default-features --features <its feature>` |
