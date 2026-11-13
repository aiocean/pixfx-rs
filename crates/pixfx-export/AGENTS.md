# pixfx-export

Layer: **Shell** (files). Read the root `AGENTS.md` ("The boundary" and "Layers and where code goes") before
changing anything here; this file only says what this crate holds.

## Job

An effect written out as files: sprite sheets (composite, colour and light PNG) with beanfx's Aseprite-style JSON
(`write_sheet`), a GIF (`write_gif`), and baked clips that hosts play without drawing (`bake`, `write_bake`,
`decode`).

## Belongs here

- File formats and their layout: grids, page packing, JSON values. Layout and JSON are pure functions; only the
  `write_*` functions touch files.

## Does not belong here

| If the code... | It goes to |
|---|---|
| renders or composites a frame | `pixfx-core` (`Player`, `render_frames`) / `pixfx-draw` |
| is a command-line flag or subcommand | `pixfx-cli` |
| picks which frame a live host shows | `pixfx-scene` |

## Dependencies

`pixfx-core`, `pixfx-draw`, `png`, `gif`, `serde`, `serde_json`. A shell: no crate below the shell layer may depend
on it (see "Known drift" in `crates/pixfx-wasm/AGENTS.md` about `decode`).

## Traps

- Bake pages hold `color` as straight RGBA and `light` as `r, g, b, 255`; decoding must give back the exact frames.
- A bake is keyed by `ClipKey`; a field missing from the key gives two clips the same name.

## Before you finish

```
cargo test -p pixfx-export
cargo test --workspace
```
