# pixfx-body: one rig for every pose, direction and animation

Date: 2026-11-16. Status: draft for approval.

## Intent

Our own tool that can generate any character sprite in any pose, any direction and any animation, instead of
buying or drawing each animation pack. The c2games animation bundle (itch.io/s/209423, six packs: platformer
hero, top-down hero, everyday, emotes, side and top-down) is the yardstick for coverage and for look. We copy no
pixel, no frame and no pose from it: every pose is our own joint angles, every pixel is painted by our code.

Two outputs, both asked for (user, 2026-11-16, "A and B"):

- **A, guide:** per-frame skeleton and body hull for an image model to draw a character onto (today's
  `pixfx-pose` job, stronger).
- **B, sprite:** the engine paints the sprite itself from the hull, with no image model.

## What the reference shows (public previews, 2026-11-16)

- The "hero" is not a costumed character: it is a **body-part mannequin**. Each part is filled with its own flat
  colour with a lighter and darker tone (head cyan; chest orange to red; upper arm, forearm, hand pink to violet;
  pelvis brown; each thigh, shin and foot its own blue, green, lime), and a **1-pixel white bone line** runs
  inside every part. No outline. About 50 px tall in a 64 cell, feet on one ground line.
- Facings E, W, S, N (side packs); 8 directions in the top-down packs. Their S and N views barely turn the body:
  limbs stay in a flat front plane.
- Clips are 7 to 12 frames, each marked LOOP or ONE-SHOT (run 8 loop, slide 7, triple jump 10, ledge climb 12,
  rope swing 12 loop). Each comes as Aseprite, PNG sheets, JSON (frame rects, timing, loop, pivots, sockets) and GIF.

So "make it like theirs" (user, 2026-11-16) is exactly the hull painted per part plus the bone overlay: A and B
are the same picture with different layers switched on.

## Goals

- Any animation in the catalog, in any direction, from one rig: side view at any yaw, top-down and isometric at
  any pitch, 4 or 8 directions as presets of the same camera.
- Layers per frame, each optional: part-coloured hull (B's look), white bone line, the hull as a body-part map
  (one id per part, for an image model and for sprite-aware effects), joints and sockets as JSON.
- Catalog coverage at least the bundle's families: grounded movement, jumps and air, landings, dash and slide,
  traversal (ledge, ladder, rope, monkey bar, vault, wall slide, roll), hurt, death, get up, everyday actions,
  emotes, plus our existing combat poses (guard, windup, slash, thrust, cast).
- Deterministic: the same clip, direction, frame and options give the same `frame_hash` natively and in wasm.

## Non-goals

- Costumed characters painted by the engine (hair, clothes). A costumed sprite comes through A (image model over
  the guide) or hand painting; B paints the mannequin look.
- Replacing the duel fighter: `pixfx-fighter` keeps its hand-painted poses.
- Physics or IK solved at run time in a game. Clips are authored as keyframes; contacts are data.

## Anti-goals

- Their S and N look: a front or back view must show real foreshortening from the 3D rig.
- Mixels or a second scale: the default figure is the ruler (`pixfx_draw::RULER`, 40 rows in a 64 cell), not
  their 50 px. A game with a taller hero fills in its own `Ruler`.
- A command-line tool. Files come from scripts in `examples/`.

## Approaches

| | What | Trade-off |
|---|---|---|
| A | Rig + hull as a guide only, image model paints | Hand-drawn look; model calls and review per character |
| B | Rig + hull painted by the engine | Any clip, any direction, instantly, deterministic; mannequin look |
| C | A first, B later | Same parts, staged |

Picked: **A and B together**, on one shared rig. Their reference look is B's look, and A's guide is B's picture
with the bone layer on, so building both costs little more than one.

## Design

### Crates

- **`crates/pixfx-body` (new, engine layer, pure, no I/O, `libm` maths).** Depends on `pixfx-draw` only
  (serde optional behind a feature, as `pixfx-fighter` does).
  - `body`: `Body` bone lengths and hull radii from a `Ruler` (moved from `pixfx-pose`).
  - `pose`: `Pose` joint angles (moved), extended with hands, feet (ankle pitch), a separate chest twist, and
    `lie` / `roll` for the whole body (rolls, tumbles, hanging upside down).
  - `rig`: forward kinematics (moved), returns joints in 3D.
  - `camera`: `Camera { yaw, pitch }` orthographic projection; presets `side(dir4)`, `topdown(dir8)`,
    `iso(dir8)`. Today's `facing` is `Camera { yaw: facing, pitch: 0 }`.
  - `clip`: `Clip { keys: [(t, Pose)], frames, fps, mode: Loop | Once, root: [(t, dx, dy)], contacts }`,
    eased between keys; `Clip::pose_at(frame)`. Phased families (start, loop, stop; rise, apex, fall) are
    separate clips sharing a name stem.
  - `catalog`: every clip as Rust data, by name (`catalog::by_name("run")`), the walk and run cycles moved here.
  - `hull`: one capsule per bone (radius from `Body`), a sphere for the head, a box for chest and pelvis,
    projected by the camera into **geometry** (`HullShape`: part id, 2D shape in floats, depth), one list per frame.
    Two renderers read the same list: `hull::pixels` rasterizes it at one game pixel per pixel, back to front, into
    a `PartMap` (part id per pixel) for B; A's smooth renderer lives in `pixfx-pose` (below).
  - `paint`: `Look` (colour ramp per part, light direction) paints a `PartMap` into a `pixfx_draw::Frame`:
    flat part colour, light and dark tone from the hull normal, optional 1-px bone line in white.
  - `socket`: named points per frame (hands, feet, head top, back, hip) for weapons and props, plus contact
    points (`ledge`, `rung`, `rope`, `wall`, `ground`) a clip pins.
- **`crates/pixfx-pose` (tool) builds on `pixfx-body`.** Keeps captions, the sheet layout and the image-model
  prompt. **A reference for an image model is never pixel art** (user, 2026-11-16: models read pixels badly): its
  cells render the same `HullShape` list and bones smooth and anti-aliased (coverage per pixel from each shape's
  signed distance) at guide resolution (about 512 px a cell), no pixel grid, no nearest-neighbour upscaling. The
  figure stays measured on the ruler, only drawn finer; the pixel sprite comes back afterwards through
  `pixfx-pixel` (`clean --height 40`). Its public names (`Body`, `Pose`, `skeleton`,
  presets) re-export from `pixfx-body` so its JSON specs keep working.
- **Scripts** in `crates/pixfx-body/examples/` (PNG through `pixfx-export` as a dev-dependency, as `pixfx-pose`
  does): `clip_sheet` writes for one clip and a camera preset the sprite sheet per layer (`hull.png`,
  `bones.png`, `parts.png`, `composite.png`), a GIF, and `clip.json` (frame rects, duration, loop, pivot,
  sockets per frame, Aseprite-compatible frame tags per direction); `catalog` writes one per catalog clip.
- **Later, own step:** `pixfx-wasm` exposure so JS games paint clips at run time, following the existing
  painter/worker pattern. Not in the first plan.

### Calls made for the builder (2026-11-16)

- **Every direction is rendered from the 3D rig, never flipped.** West is the rig turned 180°: a one-handed pose
  shows the sword hand on the far side, as a real turn does. Parts are coloured by anatomy (her right arm keeps its
  colour wherever it is), as in the reference. A game that wants a cheap flip does it itself.
- **New joints keep every existing skeleton unchanged.** The shin still ends at the sole; the foot is a stub forward
  from it along the ground; the hand is a short stub beyond today's hand point; the chest twist defaults to 0.
  `skeleton()` gives the same numbers for every existing pose, and the `pixfx-pose` tests and JSON specs stay as
  they are.
- **Contacts are kept by two-bone analytic IK at sample time.** `Clip::pose_at` eases the angles, then for each
  contact active in that frame solves the limb (hip-knee-foot, shoulder-elbow-hand) so its end stays on the pinned
  point. No IK runs in a game; it is part of making the frame.
- **Clips play in place.** The figure stays in the cell, pivot at the feet under the pelvis at rest; root motion
  (`dx` along the ground, `dy` height, in every camera) goes into the JSON per frame. The cell for a clip grows
  to the clip's bounds over every frame and direction (multiples of 8, never under the ruler's 64), and every
  frame of that clip shares it.
- **Cameras:** side pitch 0; `iso` pitch 30°; `topdown` pitch 60°. Orthographic, the same 3D size as the ruler,
  so a pitched figure is shorter on screen through foreshortening, never rescaled. The pivot is the projected ground
  point under the pelvis.
- **One part table for both outputs.** Each part (head, neck, chest, pelvis, each upper arm, forearm, hand, thigh,
  shin, foot, and the sword) has one id and one guide colour; A's legend and the part map use it. B's `Look` gives
  each part a ramp in the same hues.
- **Occluded bones:** B hides them by depth (as the reference does); A draws them fainter and thinner, so the model
  still sees where the far limb is.
- **Every clip works in every camera preset**, which is the point. Goldens pin each clip at side E and topdown S;
  the other presets are covered by behaviour tests, so the golden count stays clips × 2.
- **Clip list:** the platformer pack's named families on its page (above) are the list for step 2; the everyday and
  emote names come from those packs' pages at the start of their family.
- **The `pixfx-pose` move is agreed with the session that owns those uncommitted edits** (asked through
  `SendMessage`, waited for) before step 1 starts.

### Tests

- Rig tests move with the rig; `pixfx-pose` tests stay green unchanged.
- `tests/ruler.rs` in `pixfx-body`: the idle hull measured against `RULER` (height, feet row, head).
- Golden `frame_hash` per catalog clip and camera preset, like `pixfx-effects`.
- Behaviour tests: a loop clip's last frame flows into its first; feet on a contact do not slide; a front view
  shows the nose (`nose_visible`); a left-right symmetric pose seen from W is the mirror of E; a pitched camera
  never makes the figure taller than the ruler.

## Order of work

1. `pixfx-body` with the rig moved out of `pixfx-pose` and the camera (pitch added). `pixfx-pose` re-exports.
2. Clips and the catalog, family by family (grounded, air, land, dash and slide, traversal, hurt and death,
   everyday, emotes, combat).
3. Hull, part map, `Look`, bone overlay (B), the `clip_sheet` and `catalog` scripts.
4. Smooth hull and bones in `pixfx-pose` guide sheets (A), replacing the pixel stick cells.
5. Sockets and contacts for props (ledge, ladder, rope, wall).

## Constraints and coordination

- Another session has uncommitted edits in `crates/pixfx-pose` (`body.rs`, `sheet.rs`, `spec.rs`,
  `tests/pose.rs`) on 2026-11-16. Step 1 moves code out of those files: it waits for that session to commit, or
  agrees the move with it first.
- Commits go through a temporary index (shared tree).

## Left to the builder

- Exact key poses and timings of each clip (authored against the frame counts above: 6 to 12 frames, loop or once).
- Easing per key, hull radii per part, the exact colour ramps (bright, saturated, one hue per part, as the reference).
