---
name: pixfx-ref
description: Make a new pixfx effect from a picture an image model draws first - a spec of the effect's look and moments (forms, holds, takes a blow, ends), a labelled template sheet and prompt, the OpenRouter call, then the effect's pixels cut back out as colour and light layers frame by frame with a review page. Use when asked to design or invent an effect ("make a lotus shield", "what other shields are there, generate them and build them"), to get reference art for an effect, or when a generated sheet must be read before writing painter code.
---

# pixfx-ref

Effect references from an image model, in `tools/pixfx-rs`. The library is `crates/pixfx-ref` (pure: no files, no
network); a script per step in `crates/pixfx-ref/examples/` does the I/O. The model's picture is a **reference the
agent reads**: you write the effect's painter code by hand in `pixfx-effects` (skill `pixfx-effect`). Nothing in the
engine paints from what this crate extracts.

Run from `tools/pixfx-rs`. Each script's first lines say what it takes and writes.

| Step | Script | Writes |
|---|---|---|
| 1. Spec | write `<my>.json` (copy `crates/pixfx-ref/examples/specs/TEMPLATE.json`) | |
| 2. Template | `cargo run -q -p pixfx-ref --example template -- <my>.json` | `target/ref/<slug>/`: `template.png`, `sheet.json`, `prompt.txt`, `style.png` |
| 3. Draw | `cargo run -q -p pixfx-ref --example generate -- target/ref/<slug>` | `draw-<n>/image-01.png`, `job.json`; a line in `target/ref/spend.jsonl` |
| 4. Extract | `cargo run -q -p pixfx-ref --example extract -- <my>.json target/ref/<slug>/draw-<n>/image-01.png` | `target/ref/<slug>/review/`: per frame original, alpha, colour, light, rebuilt PNGs; diffs; `index.html` |
| 5. Gallery | `cargo run -q -p pixfx-ref --example gallery` | `target/ref/index.html`: every effect's sheet, linking to its review page |
| 6. Build | write the effect in `pixfx-effects` from what the review page shows (skill `pixfx-effect`) | |

Step 3 spends money (about $0.065-0.075 a draft sheet) and needs `OPENROUTER_API_KEY` in the environment and `curl`
on the path. Run `generate ... --dry-run` first to read the request. Every run draws into a new `draw-<n>`; nothing is
overwritten. Read `prompt.txt` before you spend.

## The spec

`crates/pixfx-ref/examples/specs/shield/*.json` are eleven worked specs (the xianxia shields); `TEMPLATE.json` shows
every field. A spec holds:

- `id`, `name`; `look`: what it is and looks like, a name then a sentence or two; `subject`: the thing that keeps its
  size and place in every frame (`shield`, `bell`).
- `rows`: the moments, top to bottom. Each has a `name` (`INIT`, `IDLE`, `HIT RIGHT`, `HIT LEFT`, `BREAK`, any you
  need), a `caption` written after the first cell's label, and a `text`: what happens, one sentence, no full stop.
  - `frames` on a row: its own cell count (a slow build-up of 5, a blow of 3). The sheet is as wide as the longest
    row; the cells past a short row's end are left off the template.
  - `notes` on a row: what each frame must show, by frame from the first (`""` says nothing about that frame).
- `rules`: lines for the whole effect, after the rows.
- `frames` (3), `cell` (96 sprite pixels), `gap` (14), `scale` (3), `figure` (the 40-pixel stick figure, as bones
  text): defaults that rarely change.

## What the model does, and the words that stop it

Found making the shields (2026-11-14); the prompt in `pixfx_ref::prompt` holds the fixes.

| Left alone, the model... | So |
|---|---|
| draws a character over the stick figure, and the effect then has to be cut from it | the prompt names the figure as a size guide only and says not to draw it |
| paints a dark backdrop that blends into every glow, so colour and light cannot be told apart | the request asks for a transparent background (`gpt-image` honours it; Gemini, the `hero` tier, does not) |
| draws the first frame of a build-up already half formed, and the last frame of an ending still mostly there | give the row more `frames` and a `note` per frame with a pixel budget: "only 3 to 6 faint specks, under 20 pixels in all; no bell shape yet". With the notes, golden bell's INIT went 9, 136, 740, 3461 colour pixels |
| changes the shape between runs (a dome one time, a sphere the next) | say the shape in `look` and in the IDLE `text` |
| keeps the template's cyan cell frames | `extract` takes cyan out near each cell's edge |

A blow from one side only shows one side: add a `HIT LEFT` row when the game needs both.

## Reading the review page

At the top, a player runs the frames in order: every row in turn or one row looped, the layer to show (rebuilt,
original, colour, light), in-betweens, fps and size; space plays and pauses, ← → step a frame. Watch the motion
there first. In-betweens (`pixfx_ref::tween`; 7 by default, `extract --between N`) are frames made between each two
drawn frames of a row by moving their pixels, so the motion plays smooth; the drawn frames keep the fps. They move
what the model drew and never redraw it: judge the drawing with in-betweens at 0.

Below it, each row is one frame: **original** (the model's cell over black), **the model's pixels** (the busiest
part of the original blown up, with the cuts between the model's pixels in cyan: each square should hold one flat
colour), **alpha**, **colour** (the opaque pixels, exactly as
drawn; checkerboard = empty), **light** (what the see-through pixels add over black), **rebuilt** (colour, else
light). Rebuilt must match original; the error under each frame name is the mean per channel (0-255), about 1-4 on
the shields. The diffs show what moves between frames: that is the motion to write.

The layers are one value per pixel the model drew, not per template pixel: the model draws its own pixels, 2 to 4
sheet pixels each, so a frame comes out 90 to 160 pixels a side, the same size in every frame of the sheet (the
sentence above the table says it). `grid` under a frame's name is how strongly its edges follow its cuts (0 none, 1
perfect; 0.2 to 0.5 on the shields). Below about 0.15, or when the page says no grid showed, the cuts are doubtful:
read the original, not the layers.

`moved` under a frame's name is how many model pixels it was moved (right, down) to line up with the other
frames; the layers and the player show it moved. The model often draws a whole row a few pixels off (HIT rows
+5 on the lotus); a frame that changes rather than moves stays put.

`matrix.json` beside the page holds every frame as a pixel matrix: per frame a colour map and a light map as text
rows (a palette letter per pixel, `.` elsewhere) and the exact colour of each of those pixels in reading order; the
palettes are at the top. Write painter code from the maps and palette; take a colour that matters from the exact
values. The page shows the palette as swatches and each frame **through the palette**. Each swatch is a colour picker, with a hue slider and a
mode (keep shading: the entry's pixels move by its change; flat: they take its colour): the player switches to
**recoloured** and every frame follows, to try a palette before writing it into the effect.

The model's alpha is a soft matte, so the colour layer holds most of the glow too and the light layer is a thin halo.
Read the light from the original's glow and the colour layer's bright pixels, and write light as light in the
effect.

When `extract` cannot find the labels (it says how many rows it found), the model moved or dropped a label: look at
the sheet, and draw again.

## Not this skill

- A character, a pose, a sprite to redraw or split: `pixfx-pixel`, `pixfx-pose`.
- Porting an existing effect from pixelfx or Sprite Fusion: `pixfx-effect` (parity, no image model).
