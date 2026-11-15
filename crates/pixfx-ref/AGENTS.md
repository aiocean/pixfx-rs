# pixfx-ref

Layer: **Tool**, beside the engine, not in it, like `pixfx-pixel` and `pixfx-pose`. Read the root `AGENTS.md`
("Layers and where code goes") first; this file only says what this crate holds.

## Job

Effect references from an image model: a spec of an effect's look and moments becomes a labelled template sheet and
a prompt, the OpenRouter request is built and its answer read, and the sheet the model drew is cut back into the
effect's frames as pixfx's colour and light layers, with a review page.

| Module | Holds |
|---|---|
| `spec` | `Spec`, `Row` (a moment: name, caption, text, its own frame count, a note per frame), `rules`; `FIGURE` |
| `template` | The sheet's `layout`, the cells' `labels`, `render` (on `pixfx_pixel::sheet`, labels by `pixfx_pose::captions`) |
| `prompt` | The prompt from a spec: figure as a guide only, transparent background, rows, notes, rules |
| `openrouter` | `Tier`, `Request`, `body`, `data_url`, `parse_answer`; `ENDPOINT`, `KEY_VAR` |
| `base64` | RFC 4648 encode and decode |
| `extract` | Cells from the labels, the cell frames taken out, the model's own pixel grid (`pixfx_pixel::frames`), frames lined up on one origin (`pixfx_pixel::register`), colour and light layers per model pixel, frame diffs |
| `matrix` | Every frame as a pixel matrix (`Px`: empty, colour or light, its exact value and a palette entry), the palettes the frames share (32 colour, 16 light, by `pixfx_pixel::reduce`), `paletted`, `to_json` (`matrix.json`) |
| `tween` | Frames between the drawn ones of a row, made by moving their pixels: a coarse block motion, pixel pairs, a dense field each way, a growth front for pixels with no partner, gathered so no cell is left empty; unlike colours swap at a time of their own instead of mixing (`motion`, `between`, `tweens`) |
| `review` | The effect page (a player with in-betweens, then every frame's columns and the diffs), the gallery, `report` JSON, the file name of each column |

`examples/` holds a script per step (`template`, `generate`, `extract`, `gallery`), sharing `examples/common/mod.rs`,
and `examples/specs/`: the eleven xianxia shields and `TEMPLATE.json`. The skill `.claude/skills/pixfx-ref/SKILL.md`
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

- The prompt's words are tested against the golden-bell sheet the shields were drawn from
  (`tests/golden-bell.prompt.txt`). A spec with no `frames`, `notes` or `rules` on its rows must keep giving that text
  byte for byte; change the wording on purpose, and update the test with the reason.
- `extract` finds the cells from the labels the model redraws, not from the template's coordinates: the model
  re-lays the page. Halves round to even (`libm::rint`), as the first (Python) version did, so the shields' cells
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
