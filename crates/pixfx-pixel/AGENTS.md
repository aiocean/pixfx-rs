# pixfx-pixel

Layer: **Tool**, beside the engine, not in it. Read the root `AGENTS.md` ("Layers and where code goes") first; this
file only says what this crate holds.

## Job

Pixels for agents. An agent that draws pixel art as painter code works from a picture: an image model's drawing, an
artist's, or a customer's own sprite. This crate turns the picture into a grid the agent reads: `cut` (out of its
background, specks and guide lines gone), `nearest` / `colors` (each pixel as an index into a palette, a given one
or the picture's own), `shrink` (a picture drawn big down to its pixel grid), `rects` (equal cells merged), `text`
(a letter per cell, a colour legend, column and row numbers), `zoom` (each cell a square inside grid lines; an empty
`Map` is a blank grid for an image model to draw into), `Map::paint` (true size) and `diff`. `pixfx pixel` in
`pixfx-cli` runs it.

## Belongs here

- Reading a picture as pixels and showing pixels back to an agent: grid recovery, palette matching, cleaning,
  splitting a sprite into parts, any view that helps an agent reason about cells.

## Does not belong here

| If the code... | It goes to |
|---|---|
| paints pixels a game shows | `pixfx-draw` (primitives) or the crate that owns the thing drawn |
| reads or writes files, parses arguments | `pixfx-cli` |
| is a generated table something in the engine paints from | nowhere: the agent writes painter code instead |

## Dependencies

`pixfx-draw` (`Rgba`, `RgbaImage`, `to_hex`) and `libm`. Nothing in the engine depends on this crate; only
`pixfx-cli` does.

## Traps

- Ties go to the first value met (`shrink`, `rects`), so the same picture gives the same grid on every run.
- `text` refuses more colours than `LEGEND` has letters rather than reusing a letter: pass a palette.

## Before you finish

```
cargo test -p pixfx-pixel
cargo test -p pixfx-cli
```
