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
| `frames` | The pixel grid shared by a sheet's frames (`recover_frames`): pitch from the gaps between edges of every frame, one pixel count for all, cuts per frame, a strength per frame; `grid_view` and `busiest` to check cuts by eye |
| `register` | The frames of one drawing lined up on one origin: a whole-pixel shift per frame from its edges against the median of every frame's edges, taken only when the match is clear; `apply` moves a frame |
| `cut`, `grid` | A picture out of its background; pixels as palette indices (`nearest`, `colors`), `shrink`, `rects`, `Map` |
| `oklab`, `reduce` | Perceptual colour distance; colours merged down to a count, or snapped to a palette |
| `show`, `view` | A map as text with a (named) legend and back (`text_named`, `parse`); as a picture sized to its token cost (`view`, `zoom`) |
| `critique` | Checks with cells and a fix each, and a score, the line craft (stray, doubled corner, jagged line, banding) after klad's pixel check; drift across frames |
| `ramp` | A material's hue-shifted shades from one base colour (`oklab::rgb` takes OKLab back to sRGB) |
| `sheet` | Blank sheets for an image model, guides (pivot, ground, bones, a sprite), cells cut back; `sheet.json` and bones text |
| `parts` | A sprite split into named parts with parents and pivots, from a spec the agent writes |
| `diff` | Pixels two pictures differ by; `feet` stands two figures on the same spot first |
| `scale` | A sprite against the game's ruler (`pixfx_draw::Ruler`): `standing` (a standing character's height and width), `against` (a thing as many cells as a body part times a ratio), `part` (a part by name); `critique --standing` and `--against PART*RATIO` |
| `tool` | The agent's steps as pure functions: `clean`, `read`, `draw`, `view`, `sheet`, `cut`, `parts`, `Palette` (from a look, hex, a `.gpl` or `.hex` file, or ramps; written as `.gpl`) |

`examples/` holds a script per step (`clean`, `read`, `draw`, `view`, `critique`, `ramp`, `sheet`, `cut`, `parts`, `diff`;
`cargo run -p pixfx-pixel --example <step> -- ...`), sharing `examples/common/mod.rs`, and scripts an agent wrote
when no step fit (`split_by_colour`). The skill `.claude/skills/pixfx-pixel/SKILL.md` is the agent's guide.

## Belongs here

- Reading a picture as pixels and showing pixels back to an agent: grid recovery, palette matching, cleaning,
  splitting a sprite into parts, any view that helps an agent reason about cells.

## Does not belong here

| If the code... | It goes to |
|---|---|
| paints pixels a game shows | `pixfx-draw` (primitives) or the crate that owns the thing drawn |
| reads or writes files, parses arguments | a script in `examples/` |
| is a generated table something in the engine paints from | nowhere: the agent writes painter code instead |

## Dependencies

`pixfx-draw` supplies pixels, `pixfx_core::body` supplies the rig, and `serde` and `serde_json` handle artwork specs.
Tests and examples use `pixfx-export` for PNGs, `pixfx-fighter` for named palettes, and `pixfx-core` and
`pixfx-effects` for style references. The engine and its shells build without this crate.

## Traps

- Ties go to the first value met (`shrink`, `rects`), so the same picture gives the same grid on every run.
- `text` refuses more colours than `LEGEND` has letters rather than reusing a letter: pass a palette, or `reduce`.
- `recover` solves one picture alone: on a sheet's frames it gives each its own size, and none on a sparse frame.
  Frames of one drawing go to `frames::recover_frames`.
- `recover` may find no grid (a picture already one pixel a cell, or noise): pitch 1, and the picture comes back as
  it was. Low `total` confidence means the grid is doubtful.
- `critique` reports every orphan cell in one issue, so one stray-pixel habit costs one warning, not the score.

## Pose and reference sheets

- `pose` draws captioned skeletons for image models using the `pixfx_core::body` rig. See [pose instructions](src/pose/AGENTS.md).
- `reference` makes prompts and templates, reads model output, and compares the resulting frames. See [reference instructions](src/reference/AGENTS.md).
- `sheet::captions` serves both modules. Model calls and file I/O live in `examples/`.

## Before you finish

```
cargo test -p pixfx-pixel --all-targets
cargo clippy -p pixfx-pixel --all-targets
```
