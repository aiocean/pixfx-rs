---
name: pixfx-pixel
description: Turn a picture into pixel art an agent can read and rewrite as painter code, with the pixfx-pixel Rust library and its step scripts. Use when a sprite or pose in a pixfx game looks wrong and needs redrawing ("the knocked-down pose looks bad", "draw this cultivator"), when an image model's drawing must become a sprite, when a customer's sprite must be read or split into parts (head, arms, legs) for effects that act on it, or when pixel art needs a critique, a blank sheet for a model to draw into, or a comparison with what the engine paints.
---

# pixfx-pixel

Pixels for agents, in `tools/pixfx-rs`. The library is `crates/pixfx-pixel`: each step is a pure function in
`pixfx_pixel::tool`, and a script per step in `crates/pixfx-pixel/examples/` runs it on files. It reads pictures and
shows pixels back. **It never ships in a game**: you read what it
shows and write painter code by hand, as the existing poses are written. Never paint from a generated table.

Run from `tools/pixfx-rs`: `cargo run -q -p pixfx-pixel --example <step> -- ...`. Every step writes into `--out`
(default `target/pixel`) and prints what it wrote. `--palette` takes hex colours
split by commas or a fighter look (`thunder`, `frost`); a look names each colour in the legend (`a robe0 #f4efe2`).

| Step | Does |
|---|---|
| `clean <png> [--cut 40] [--height N] [--palette P] [--max-colors 32] [--crop X,Y,W,H] [--flip]` | An image model's picture down to one pixel a cell: cut from its background, grid found (prints pitch, cells, confidence), resampled to `N` rows, snapped to the palette or merged to 32 colours. |
| `read <png> [--palette P] [--region X,Y,W,H] [--rects]` | A sprite as text: a letter per cell under a legend, column and row numbers; `--rects` the cells merged into rects (what painter code fills). |
| `draw <txt>` | The text (edited or not) back to a PNG. |
| `view <png> [--region X,Y,W,H] [--max-side 512]` | A picture sized to what it costs you to look at; grid lines from 8x, an overview when too big. |
| `critique <png>... [--frames] [--intended X,Y;X,Y]` | Issues with cells and a fix each, and a score (100 − 30 an error, 12 a warning, 4 a note); `--frames` checks drift across a row; exits 1 on an error. |
| `sheet --cell WxH --grid CxR [--scale 8] [--gap 2] [--pixel-lines] [--bones FILE]` or `sheet --directions <png> [--scale 8]` | A blank sheet for an image model (`sheet.png` + `sheet.json`); bones (`X,Y X,Y` per line) drawn into each cell as a skeleton to draw on. `--directions` is a 3x3 round the sprite with a gap of 2. |
| `cut <drawn.png> <sheet.json>` | Each cell the model drew, back to one pixel a cell (`cell-C-R.png`). |
| `parts <png> <spec.txt> [--palette P]` | A sprite split into named parts with a parent and a pivot each (`<part>.png`, `<part>.txt`). |

Beside them: `diff a.png b.png --anchor feet` (two pictures stood on the same spot, then compared; exits 1 when
they differ), and in `pixfx-fighter`, `cargo run -q -p pixfx-fighter --example poses -- <look> --pose P --states`
(one pose at three moments, a row per state: up or out, facing, flash, ice, shock).

## The loop: redraw a sprite that looks wrong

1. See what is there now: the `poses` script with `thunder --pose down --states --out target/pixel` writes the sheet
   (`thunder-down.png`, a row per state it prints) and each cell alone (`thunder-down/r<row>-m<moment>.png`);
   `view` them.
2. Get a reference drawing (optional; skip when you can fix it by hand). One subject per call, on the cheap tier,
   with the game's own sprites as the style reference: that reference, not a grid plate, is what makes the model
   draw at the game's pixel size (spike of 2026-11-14: draft + style reference gave 40 rows, the fighters' height).
   ```
   bun --env-file=.env \
     ../openrouter-image/generate.ts \
     --prompt-file prompt.txt --out-dir target/art/<new dir> --game <slug> --tier draft --ref target/pixel/thunder-poses.png
   ```
   A new `--out-dir` each call (a rerun costs money); `--dry-run` shows the request for free. Ask the user before
   spending on `hero`.
3. `clean <drawing> --cut 40 --height <the sprite's rows> --palette <look>`. Check the confidence it prints (the mean over
   painted cells): below about 0.8 the grid is doubtful, so `view` the result before trusting it.
4. `read` the clean sprite with the same palette (and `--rects`), then write the pose as painter code in the
   fighter's painter, the way the neighbouring poses are written. Copy shapes and proportions, not every cell.
5. Paint the result (`poses --pose P`), compare one cell with the clean drawing,
   `diff <clean.png> target/pixel/thunder-down/r0-m1.png --anchor feet`, and `critique` that cell (or
   `critique --frames r0-m0.png r0-m1.png r0-m2.png` for the three moments). Fix, repeat.

## Parts: effects that act on a sprite

`read` the sprite, decide the parts yourself, write a spec, and let `parts` check it (every painted cell in exactly
one part, each parent touching its part):

```
# name parent rule   (boxes are X,Y,W,H in cells; colours are legend letters, `colors d,e [in X,Y,W,H]`)
body - rest
head body box 23,22,15,13
arm body box 37,37,3,10
```

On `crates/pixfx-pixel/tests/fixtures/thunder-idle.png` with `--palette thunder` that prints
`head body 23,22,14,13 32.50,35.00`: name, parent, box, pivot.

Each part comes back with its box and its pivot (the middle of the border with its parent), the joint an effect
turns it around or tears it off at.

## When to write a script

When no step fits (a batch over many files, a new measurement), write a Rust example against the library:
`crates/pixfx-pixel/examples/split_by_colour.rs` is the pattern. Run it with
`cargo run -p pixfx-pixel --example <name> -- <args>`. Move its logic into `pixfx_pixel::tool` (tested) and make it a step only when it is needed a third time.

## Prompting an image model

Sprite Fusion's rules for agents, which hold for our models too:

- "Describe one item or subject without requesting a number of outputs."
- "Ask for the subject or variations, not for a grid, contact sheet, atlas, or spritesheet that packs several
  assets into one image."
- "State the requested change and anything important that should remain unchanged."
- "Describe the motion, energy, and whether it should loop."
- "Avoid noisy phrases such as "transparent background," "crisp pixel art," or "pixel-perfect pixel art.""
- "Do not include pixel dimensions in the prompt."
- "Omit palette by default… don't infer it from a theme, a style, or a wish for consistency."

Pose before motion: get the key pose right as its own sprite, then the frames around it.
