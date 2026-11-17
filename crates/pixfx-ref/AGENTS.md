# pixfx-ref

Layer: **Tool**, beside the engine, not in it, like `pixfx-pixel` and `pixfx-pose`. Read the root `AGENTS.md`
("Layers and where code goes") first; this file only says what this crate holds.

## Job

Effect references from an image model: a spec of an effect's parts and moments becomes a labelled template sheet
(a pixel grid in every cell, a PARTS row of the effect taken apart) and a prompt, the OpenRouter request is built and
its answer read, and the sheet the model drew is cut back into parts and frames as pixfx's colour and light layers,
with a review page. Then the agent writes the effect by hand and `compare` puts its frames beside the sheet's cells,
with each step's change measured as entropy.

| Module | Holds |
|---|---|
| `spec` | `Spec` (`grid`, `parts`, `sheet_rows`), `Part`, `Row` (a moment: name, caption, text, its own frame count, a note per frame), `rules`; `FIGURE` (the duel's fighter at the ruler, `pixfx_draw::RULER`, in engine pixels), `PARTS`, `is_part` |
| `template` | The sheet's `layout` (the PARTS row first), the cells' `labels`, `render` (on `pixfx_pixel::sheet`, labels by `pixfx_pose::captions`): the pixel grid (`MINOR`, `DARK` every `MAJOR`), no figure in the parts row, the figure scaled from a 96-pixel cell |
| `prompt` | The prompt from a spec: figure as a guide only, transparent background, rows, notes, rules; with a grid or parts, `sheet_prompt` (whole squares in flat colour, a small shared palette, each part alone and whole, moments built from the parts); on a grid, the character's height, head, hand and eye in squares, so every object is drawn at its scale |
| `openrouter` | `Tier`, `Request`, `body`, `data_url`, `parse_answer`; `ENDPOINT`, `KEY_VAR` |
| `base64` | RFC 4648 encode and decode |
| `extract` | Cells: on a grid sheet the template's columns scaled to the sheet and each row's top and bottom from the label lines (`grid_cells`), the grid known (`known_grid`); on an old sheet from the labels, the model's own pixel grid (`pixfx_pixel::frames`) and frames lined up on one origin (`pixfx_pixel::register`). The cell frames taken out, colour and light layers per pixel, parts named `parts-<name>`, frame diffs |
| `matrix` | Every frame as a pixel matrix (`Px`: empty, colour or light, its exact value and a palette entry), the palettes the frames share (32 colour, 16 light, by `pixfx_pixel::reduce`), `paletted`, `to_json` (`matrix.json`) |
| `tween` | Frames between the drawn ones of a row, made by moving their pixels: a coarse block motion, pixel pairs, a dense field each way, a growth front for pixels with no partner, gathered so no cell is left empty; unlike colours swap at a time of their own instead of mixing (`motion`, `between`, `tweens`) |
| `entropy` | Each pixel as a symbol (empty or a brightness level), `change` (H(b given a), 0 to 1), `steps` along a row: the agent's gauge of timing |
| `compare` | `effect_cell` (an effect frame's window round its origin, `ENGINE_CELL` engine pixels a cell, feet at `FEET`, averaged to the cell's size), `board` (sheet cells over effect frames), `art_ruler` (the ruler in a cell's squares) |
| `review` | The effect page (a player with in-betweens, then every frame's columns and the diffs), the gallery, `report` JSON, the file name of each column |

`examples/` holds a script per step (`template`, `generate`, `extract`, `compare`, `gallery`), sharing
`examples/common/mod.rs`, and `examples/specs/`: the eleven xianxia shields (grid sheets with parts; `shield/lotus.json`
is the worked example) and `TEMPLATE.json`. The skill `.claude/skills/pixfx-ref/SKILL.md`
is the agent's guide.

## Belongs here

- Anything that gets a better reference out of an image model: prompt wording that fixes a habit of the model, a new
  spec field, a better cut of what it drew (cells, grid, layers), a clearer review page.

## Does not belong here

| If the code... | It goes to |
|---|---|
| paints an effect a game shows | `pixfx-effects`, written by hand from the reference |
| reads a sprite or pose as pixels, splits it into parts | `pixfx-pixel` |
| draws a pose skeleton | `pixfx-pose` |
| reads or writes files, runs curl, reads the environment | a script in `examples/` |

## Dependencies

`pixfx-draw` (`RgbaImage`), `pixfx-pixel` (`sheet`, `frames`, `register`, `reduce`), `pixfx-pose` (`captions`), `serde`, `serde_json`, `libm`; for
the scripts only, `pixfx-export` (PNG), `pixfx-core` and `pixfx-effects` (a frame of an effect as the style
reference). No HTTP crate: `generate` runs `curl`, handing it the key on stdin.

## Traps

- The prompt's words are tested against the golden-bell sheet the shields were first drawn from
  (`tests/golden-bell.spec.json`, `tests/golden-bell.prompt.txt`). A spec with no `grid`, `parts`, `frames`, `notes`
  or `rules` must keep giving that text byte for byte; change the wording on purpose, and update the test with the
  reason.
- On a grid sheet the model keeps the template's columns but squashes the rows unevenly (cells 0.81 to 0.94 as tall
  as wide), boxes labels in black and draws dark frames round cells. `grid_cells` therefore takes x from the template
  and only the label lines' heights from the sheet, ignores ink in runs of `STROKE` pixels or more (frames, box
  edges), and gives the last row the median height of the others. Squares are counted up from a cell's bottom.
- A spec with `layers` is drawn as a parts sheet, then a sheet per moment (`Spec::parts_sheet`, `Spec::moment`), so
  no sheet has more than a few rows: the model returns about 1536 x 1152 whatever is asked, and a sheet of 10 rows
  leaves under 2 sheet pixels an art pixel. A moment's sheet holds no parts; its prompt names the parts sheet,
  sent as the third image (`parts.png`, copied in by `template --moment ... --parts`).
- `framed_cells` reads the cells from the cyan frames the model keeps: on the parts sheet it stretched 5 cells over
  the width of 6, so the template's columns were wrong. A frame broken by glow still counts (a column 40% cyan
  inside its row). When the frames are not found, the label paths run as before.
- Cells that are not square get the model's own pixel size (`pitch_grid`: found on every cell trimmed to the
  smallest, each cell cut on it from its bottom, padded to the largest), not the template's 64 x 64.
- A pivot is pure green (`spec::PIVOT`), read as green ahead of red and blue by 90 or more; an effect drawn in that
  green would lose those pixels. Marks are painted over in every cell (the model copies them into moments), and
  pivots are kept only on part cells, one per mark.
- `framed_cells` pairs frame sides by the one cell width that gives the row's cell count, so an upright cyan piece
  (sword-ring's swords) is not a side. A `pitch_grid` fit below `DOUBTFUL` falls back to the width pitch.
- `without_frames` also clears a column or row inside a cell that is frame cyan along 3/5 of it: the model sometimes
  draws a frame line through the art.
- Grid sheets are never registered (shifted): the grid is the registration. Parts are never shifted, diffed,
  tweened or played: they are not frames.
- The shields' specs are grid sheets at `cell: 64`; their earlier draws in `target/ref/shield_*/draw-*` were made
  from 96-pixel templates without a grid and no longer match the spec. Draw again before extracting.
- A template cell covers 96 engine pixels whatever its resolution (`compare::ENGINE_CELL`): one art pixel of a
  64 cell is 1.5 engine pixels, so the 40-pixel fighter is 27 squares and its 7-pixel head 5 (`compare::art_ruler`).
  Painter code sizes each object from the ruler, never by reading the sheet's squares as engine pixels. A `cell: 96`
  grid draws a square per engine pixel but needs 1.5 times the sheet pixels a cell; it has not been drawn yet.
- The sheet's entropy steps run higher than a clean effect's (glow and specks change every frame): compare the
  shape of the two rows of steps, not their size.
- `extract` on an old sheet finds the cells from the labels the model redraws, not from the template's coordinates:
  the model re-lays the page. Halves round to even (`libm::rint`), as the first (Python) version did, so the shields' cells
  are found again.
- The sheet must come back with alpha. A model that ignores `background: transparent` (Gemini) gives an opaque
  sheet whose every pixel reads as colour.
- Cyan near a cell's edge is taken for the template's frame and dropped, an effect's own cyan with it.
- The layers are the model's pixels, not the template's: about 2 to 4 sheet pixels each, so a frame is 90 to 160
  pixels a side where the template said 96. Every frame of a sheet has the same size, so pixel `(x, y)` is the same
  place in each. A sheet with no grid to read falls back to the template's size (`found` is false); a low `fit`
  or `strength` (below about 0.15) means the cuts are doubtful.
- Each frame's layers are moved (`Cell::shift`) to line up with the others: the model draws the subject a few
  pixels off in some frames, often a whole row. A frame whose content changes (petals flying apart) or holds few
  pixels stays where it is; motion meant by the effect (a recoil) would be undone too, so a spec that wants the
  subject to move says so and the shift is read off the review page.
- In-betweens move pixels, they do not redraw: where the model changed what it drew between two frames (thunder's
  cracks in new places) they are a believable morph, not the frame the model would have drawn. Read the drawn
  frames, not the in-betweens, when writing painter code.
- The exact colour in `matrix.json` is the truth; the palette is the handle for recolouring and painter code and
  misses it by 3 to 11 per channel (bagua's rare orange spark goes paler, faint glow bands with 16 light
  entries). Read the exact values when a colour matters.

## Before you finish

```
cargo test -p pixfx-ref
cargo clippy -p pixfx-ref --all-targets
```
