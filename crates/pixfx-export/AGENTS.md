# pixfx-export

Layer: **Shell** (files). Read the root `AGENTS.md` ("The boundary" and "Layers and where code goes") before
changing anything here; this file only says what this crate holds.

## Job

An effect written out as files: sprite sheets (composite, colour and light PNG) with beanfx's Aseprite-style JSON
(`write_sheet`), a GIF (`write_gif`), a video through ffmpeg on `PATH` (`write_video`), rendered clips that hosts play without drawing (`render_clip`, `write_clip`,
`decode`), a still of an effect beside the character it plays on for checking scale (`board`: `busiest`, `still`), and the character a script plays an effect on (`Actors`: sprite
frames read from PNGs, shown in turn; `gif --actor`, `stage` and `scale_board` read it).

`examples/` holds the scripts that write them: `sprite_sheet`, `gif`, `mp4`, `render_clip`, `stage` (an effect
on a stage between two characters, played through `pixfx-scene`: ordered at the target, cues answered with a `Feel`),
`blood_spray` (drops thrown in arcs, each a `projectiles/trail` in blood colours that the script moves, as a game
moves any mover), `carry` (an effect carried back and forth with `carried` 1, written as `<id>.carry.gif`) and
`scale_board` (every effect's busiest frame on the fighter's sprite, with a page, to see which are out of scale)
(`cargo run -p
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

`pixfx-core`, `pixfx-draw`, `png`, `gif`, `serde`, `serde_json`; `pixfx-effects`, `pixfx-scene` and `libm` for tests and scripts only. A
shell: no crate below the shell layer may depend on it (see "Known drift" in `crates/pixfx-wasm/AGENTS.md` about `decode`).

## Traps

- Clip pages hold `color` as straight RGBA and `light` as `r, g, b, 255`; decoding must give back the exact frames.
- A clip is keyed by `ClipKey`; a field missing from the key gives two clips the same name.

## Before you finish

```
cargo test -p pixfx-export
```
