---
name: pixfx-body
description: Generate a character's poses, animation clips and directions with the pixfx_core::body rig - a mannequin sprite sheet (hull, bones inside it, a part map) a row per direction (side 4, iso or top-down 8) and a column per frame, with a GIF and each frame's joints and root motion as JSON. Use when a game needs a walk, run, crouch or any pose in every direction, a new animation clip, a turnaround, a skeleton or body-part map of a frame, or joint positions to place effects and hitboxes.
---

# pixfx-body

One rig for every pose, direction and animation, in `pixfx_core::body` (engine layer, pure). Read its
`crates/pixfx-core/src/body/AGENTS.md` before editing it. The mannequin follows `pixfx_draw::RULER`.
For image-model guide sheets, see the `pixfx-pose` skill.

## Make a sheet

```sh
cargo run -p pixfx-core --features body --example clip_sheet -- walk --camera topdown     # target/body/walk/
cargo run -p pixfx-core --features body --example clip_sheet -- pose/guard --camera iso --out DIR
cargo run -p pixfx-core --features body --example catalog -- --camera side                # every clip, target/body/side/<clip>/
```

| File | Holds |
|---|---|
| `composite.png` | the mannequin: hull shaded per part, white bones on top where they are not hidden |
| `hull.png`, `bones.png` | each layer alone |
| `parts.png` | each pixel in its part's guide colour (`Part::guide`): a mask per body part |
| `<clip>.gif` | the first direction playing, 4x |
| `<clip>.json` | `cell`, `pivot` (the ground point), `directions`, per frame `rect`, `duration_ms`, `root` (motion along the ground, height) and `joints` in cell pixels; a tag per direction |

Rows: side is `E S W N`; iso and top-down are `E SE S SW W NW N NE`. Every direction is turned in 3D, never
flipped. The cell is one size for the whole clip (multiples of 8, at least 64) so nothing leaves it.

## The pieces, from code

| Call | Gives |
|---|---|
| `Body::default()` | the figure on the ruler; `Body::new(height, heads)` for another |
| `Pose { .. }`, `Limb::new(raise, spread, bend)` | joint angles: lean, twist, tilt, roll, spin, lift, hands, feet, head, sword |
| `Camera::side(Dir4)`, `iso(Dir8)`, `topdown(Dir8)`, `Camera { yaw, pitch }` | how she is seen |
| `skeleton_seen(&body, &pose, cam)` | named joints with depth, and `ground` |
| `Clip { keys, root, contacts, .. }`, `clip.pose_at(&body, f)` | key poses eased (`Ease`), root motion, a foot or hand planted by IK while its `Contact` lasts |
| `catalog::by_name("run")`, `catalog::NAMES` | the ready clips: ground moves and every preset as `pose/<name>` |
| `hull(&body, &skel)` | the body as shapes per part, back to front |
| `sprite(&body, &clip, f, cam, cell, &look, layers)` | one frame painted, its part map and skeleton |
| `clip_sheet(&body, &clip, view, &look)` | the whole sheet and its `SheetMeta` |

## A new clip

Add it to the catalog (`crates/pixfx-core/src/body/catalog/`, its family's file, its name in `NAMES`), keyed in poses on the ruler. A
moving clip sets root motion and plants each foot with a `Contact` so it does not slide; `catalog::foot_at` places a
foot flat at a point. Then bless the goldens and look at the sheet:

```sh
PIXFX_BLESS=1 cargo test -p pixfx-core --features body --test body_goldens
cargo run -p pixfx-core --features body --example clip_sheet -- <clip>
```

Read the composite by eye in every direction before you call it done.

## Before you finish

```sh
cargo test -p pixfx-core -p pixfx-pixel --features pixfx-core/body --all-targets
cargo clippy -p pixfx-core -p pixfx-pixel --features pixfx-core/body --all-targets -- -D warnings
```
