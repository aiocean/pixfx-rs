# pixfx-export

Layer: **Shell** (files). Read the root `AGENTS.md` ("The boundary" and "Layers and where code goes") before
changing anything here; this file only says what this crate holds.

## Job

An effect written out as files: sprite sheets (composite, colour and light PNG) with beanfx's Aseprite-style JSON
(`write_sheet`), a GIF (`write_gif`), a video through ffmpeg on `PATH` (`write_video`), baked clips that hosts play without drawing (`bake`, `write_bake`,
`decode`), and the parity check of a port against its source (`parity`: `compare`, `read_reference`,
`write_side_gif`, `ParityOpts`, `Actors`; `gif --actor` reads its sprite frames through `Actors` too).

`examples/` holds the scripts that write them: `sprite_sheet`, `gif`, `mp4`, `bake` and `parity` (`cargo run -p
pixfx-export --example <name> -- ...`), sharing `examples/common/mod.rs`. A script reads its arguments, calls the
library and writes files; anything worth testing moves into the library.

## Belongs here

- File formats and their layout: grids, page packing, JSON values. Layout and JSON are pure functions; only the
  `write_*` functions touch files.

## Does not belong here

| If the code... | It goes to |
|---|---|
| renders or composites a frame | `pixfx-core` (`Player`, `render_frames`) / `pixfx-draw` |
| reads a script's arguments | that script in `examples/` |
| picks which frame a live host shows | `pixfx-scene` |

## Dependencies

`pixfx-core`, `pixfx-draw`, `png`, `gif`, `serde`, `serde_json`; `pixfx-effects` for tests and scripts only. A
shell: no crate below the shell layer may depend on it (see "Known drift" in `crates/pixfx-wasm/AGENTS.md` about `decode`).

## Traps

- Bake pages hold `color` as straight RGBA and `light` as `r, g, b, 255`; decoding must give back the exact frames.
- A bake is keyed by `ClipKey`; a field missing from the key gives two clips the same name.

## Before you finish

```
cargo test -p pixfx-export
cargo run -p pixfx-export --example parity -- <id> --pixelfx /tmp/pixelfx   # when parity code changed
```
