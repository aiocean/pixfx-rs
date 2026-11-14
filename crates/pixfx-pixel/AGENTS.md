# pixfx-pixel

Layer: **Tool**, beside the engine, not in it. Read the root `AGENTS.md` ("Layers and where code goes") first; this
file only says what this crate holds.

## Job

Pixels for agents. An agent that draws pixel art as painter code works from a picture: an image model's drawing, an
artist's, or a customer's own sprite. This crate turns the picture into a grid the agent reads, and checks what it
drew:

| Module | Holds |
|---|---|
| `recover` | The pixel grid of a picture drawn big (an image model's): period by autocorrelation, cuts by dynamic programming, a confidence per cell; `fit_height` resamples to a row count |
| `cut`, `grid` | A picture out of its background; pixels as palette indices (`nearest`, `colors`), `shrink`, `rects`, `Map` |
| `oklab`, `reduce` | Perceptual colour distance; colours merged down to a count, or snapped to a palette |
| `show`, `view` | A map as text with a (named) legend and back (`text_named`, `parse`); as a picture sized to its token cost (`view`, `zoom`) |
| `critique` | Checks with cells and a fix each, and a score; drift across frames |
| `sheet` | Blank sheets for an image model, guides (pivot, ground, bones, a sprite), cells cut back; `sheet.json` and bones text |
| `parts` | A sprite split into named parts with parents and pivots, from a spec the agent writes |
| `diff` | Pixels two pictures differ by; `feet` stands two figures on the same spot first |

`pixfx pixel <verb>` in `pixfx-cli` runs it; `examples/` holds scripts an agent copies when no verb fits. The skill
`skills/pixfx-pixel/SKILL.md` is the agent's guide.

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

`pixfx-draw` (`Rgba`, `RgbaImage`, `to_hex`) and `libm`; `pixfx-export` for tests and examples only (reading
PNGs). Nothing in the engine depends on this crate; only `pixfx-cli` does.

## Traps

- Ties go to the first value met (`shrink`, `rects`), so the same picture gives the same grid on every run.
- `text` refuses more colours than `LEGEND` has letters rather than reusing a letter: pass a palette, or `reduce`.
- `recover` may find no grid (a picture already one pixel a cell, or noise): pitch 1, and the picture comes back as
  it was. Low `total` confidence means the grid is doubtful.
- `critique` reports every orphan cell in one issue, so one stray-pixel habit costs one warning, not the score.

## Before you finish

```
cargo test -p pixfx-pixel
cargo test -p pixfx-cli
```
