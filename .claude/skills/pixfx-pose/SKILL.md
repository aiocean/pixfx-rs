---
name: pixfx-pose
description: Make a colour-coded, captioned skeleton pose sheet that an image model draws a character onto, one pose per cell, in any facing (side, three-quarter, front, back), with the pixfx-pose Rust crate. Use when a character needs consistent sprites in several poses or directions ("draw her walking, guarding, kneeling", "a turnaround of this hero", "an 8-direction sprite sheet"), when an image model must be steered with a pose skeleton (a "controlnet" pose), or when a pose needs joint positions to check a drawing against.
---

# pixfx-pose

The `pixfx_pixel::pose` module builds a figure, poses it by joint angles, sees it from any
facing, and draws a sheet: one skeleton a cell, a caption above each cell, every limb in its own colour. A model
given that sheet and a picture of the character draws the character in every pose. Tested on 2026-11-14: one call
drew side, three-quarter, front and back views and a walk, guard, hurt and kneel to the skeletons, sword in the
right hand throughout.

Why the colours and captions: with plain stick figures the model guessed which arm holds the sword and which knee is
down, and one model rearranged the grid. A caption naming each pose and a colour per limb fixed both.

## Make a sheet

Write a spec (any field but `cells` may be left out):

```json
{ "cell": [64, 64], "cols": 4, "gap": 8, "scale": 8,
  "cells": [
    { "caption": "1 IDLE, side view facing right", "preset": "idle" },
    { "caption": "2 IDLE, front view", "preset": "idle", "facing": 90 },
    { "caption": "3 WALK, right foot forward", "preset": "walk", "phase": 0.25 },
    { "caption": "4 GUARD, looking at the viewer", "preset": "guard", "pose": { "head": { "turn": -40 } } },
    { "caption": "IDLE", "preset": "idle", "turnaround": true }
  ] }
```

```
cargo run -p pixfx-pixel --example pose_sheet -- spec.json --out target/art/<name>/sheet
cargo run -p pixfx-pixel --example pose_sheet -- --presets [--facing 45] --out target/pose   # see every preset
```

The figure is the game's ruler by default (`pixfx_draw::RULER`, skill `pixfx`, "One scale: the ruler"): the duel's
fighter, 61 pixels painted crown to soles, about 5.5 heads, the head as wide as its 11-pixel face, a 26-pixel blade,
the soles on row 96 of a 104 cell. A character drawn on it comes out at the size of every sprite and effect beside
it, so leave `body` out for a character of this game. Give `"body": { "height": H, "heads": N }` only for a
character of another game or another build (a chibi, a giant boss), and fill in that game's ruler too.

It writes `sheet.png` (send it to the model), `sheet.json` (the grid `pixfx_pixel::sheet::cut` takes back),
`skeletons.json` (each cell's joints, to check the drawing against) and `legend.txt` (the colour key for the
prompt). Always `Read` the sheet before spending on it.

From Rust, build a `PoseSheet { cells: vec![Cell { caption, pose, facing }], ..PoseSheet::default() }` and call
`render()`; `presets::guard()`, `turnaround("IDLE", &presets::idle())`, `Pose::mirrored()` help.

## Pose angles (degrees, all 0 = standing, arms and legs hanging, looking ahead)

| Field | Means |
|---|---|
| `right_arm` / `left_arm` / `right_leg` / `left_leg` `{raise, spread, bend}` | raise: swing forward (90 ahead, 180 up, negative back); spread: out to her side; bend: elbow folds forward, knee folds back |
| `lean` / `tilt` / `twist` | upper body forward, toward her right, turned toward her right |
| `head {nod, turn}` | chin down, look toward her right |
| `lie` | 90 lying on her back, -90 face down |
| `lift` | pixels off the ground (a jump) |
| `sword {hand, angle}` | `"right"`/`"left"`; angle folds the blade from the forearm's line like an elbow |

`facing`: 0 side view facing right (her right side toward the viewer), 45 front-right, 90 front, 180 left, 270
back. Presets: `rest idle walk run guard windup slash thrust hurt kneel crouch jump down cast` (`walk` and `run`
take `phase`, 0 to 1).

A pose in a spec is laid over its preset field by field, so `"pose": {"lean": 30}` changes only the lean.

## Prompt

Put the sheet first and the character picture second, then:

1. One line on the sheet: boxes, a caption above each naming the pose and view, a colour-coded skeleton inside.
2. `legend.txt` verbatim.
3. One line on the character.
4. "Redraw this same character once inside every box, matching that box's skeleton exactly: each limb along its
   coloured line, each joint on its dot, the sword along the grey line, facing the way the caption and the nose dot
   say, the same height and position as the skeleton."
5. "Keep the layout exactly: same boxes, same proportions, thin cyan borders, white background. Do not draw the
   skeleton lines or the captions; leave the caption area empty."

Model choice (spike 2026-11-14): GPT image (`--tier draft`) keeps the grid and follows the skeleton. Gemini
(`--tier hero`) follows the captions more than the skeleton and copies the captions into the picture.

## Check what came back

Look at every cell against its skeleton: which arm holds the sword, which leg is forward, which knee is down, the
facing. A dress hides the legs, so a share-of-skeleton-covered score cannot tell a right pose from a wrong one;
judge by limb. Watch for skeleton colours leaking into the art (a dark-teal leg from the right shin's colour).
Then cut the cells with `pixfx-pixel` (see the `pixfx-pixel` skill), resampled to the ruler's height
(`clean --height 40`), and check each standing cell with `critique --standing`.

## Ballpoint layer: the lines to trace

The colour sheet loses the lines an agent needs to write painter code: where an arm crosses the body, where
clothes overlap, folds, the face. A second call traces the drawn sheet in ballpoint pen. The traced sheet is a
guide for the agent and never goes into a sprite or the game, so its lines do not break "no outline" in a sprite.

1. Once the colour sheet passes the check above, make a directory holding `template.png` (the colour sheet the
   model drew), `style.png` (the character picture) and `prompt.txt`:

   "The first image is a sheet of boxes, each holding the same character drawn in a different pose; the second
   image is that character. Trace every box as a ballpoint pen drawing: thin dark blue-black ink lines only, on a
   transparent background. Draw every contour and every inner line: where a limb crosses the body or another limb,
   where clothes overlap, folds, hair, eyes, nose, mouth, fingers, the sword's edge and guard. No colour, no fill,
   no shading, no hatching. Keep the layout exactly: same boxes, same proportions, thin cyan borders; each figure
   the same size and in the same place as in the first image, every line exactly on the edge it traces. Do not
   draw the captions."

2. `cargo run -p pixfx-pixel --example generate -- <dir> --tier draft` (it always asks for a transparent background,
   which ink lines need).
3. Cut the answer on the colour sheet's own `sheet.json`:
   `cargo run -p pixfx-pixel --example cut -- <dir>/draw-1/image-01.png sheet.json --out <art>/ballpoint`.
   `cell-C-R.png` there and in the colour cut is the same pose.
4. Read each ballpoint cell beside its colour cell before tracing from it. Lines off the colour edges, or a
   different pose, mean the layer drifted: draw it again rather than trace a drifted layer.

Not drawn yet (2026-11-16): the prompt is a first version. `cut` brings a cell down to one pixel a cell, so a thin
ink line may break there; read the full drawn sheet when a line seems missing.
