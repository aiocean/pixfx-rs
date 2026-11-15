---
name: pixfx-pose
description: Make a colour-coded, captioned skeleton pose sheet that an image model draws a character onto, one pose per cell, in any facing (side, three-quarter, front, back), with the pixfx-pose Rust crate. Use when a character needs consistent sprites in several poses or directions ("draw her walking, guarding, kneeling", "a turnaround of this hero", "an 8-direction sprite sheet"), when an image model must be steered with a pose skeleton (a "controlnet" pose), or when a pose needs joint positions to check a drawing against.
---

# pixfx-pose

A crate in `tools/pixfx-rs/crates/pixfx-pose`. It builds a figure, poses it by joint angles, sees it from any
facing, and draws a sheet: one skeleton a cell, a caption above each cell, every limb in its own colour. A model
given that sheet and a picture of the character draws the character in every pose. Tested on 2026-11-14: one call
drew side, three-quarter, front and back views and a walk, guard, hurt and kneel to the skeletons, sword in the
right hand throughout.

Why the colours and captions: with plain stick figures the model guessed which arm holds the sword and which knee is
down, and one model rearranged the grid. A caption naming each pose and a colour per limb fixed both.

## Make a sheet

Write a spec (any field but `cells` may be left out):

```json
{ "body": { "height": 56, "heads": 6 }, "cell": [64, 64], "cols": 4, "gap": 8, "scale": 8,
  "cells": [
    { "caption": "1 IDLE, side view facing right", "preset": "idle" },
    { "caption": "2 IDLE, front view", "preset": "idle", "facing": 90 },
    { "caption": "3 WALK, right foot forward", "preset": "walk", "phase": 0.25 },
    { "caption": "4 GUARD, looking at the viewer", "preset": "guard", "pose": { "head": { "turn": -40 } } },
    { "caption": "IDLE", "preset": "idle", "turnaround": true }
  ] }
```

```
cargo run -p pixfx-pose --example pose_sheet -- spec.json --out target/art/<name>/sheet
cargo run -p pixfx-pose --example pose_sheet -- --presets [--facing 45] --out target/pose   # see every preset
```

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
Then cut the cells with `pixfx-pixel` (see the `pixfx-pixel` skill).
