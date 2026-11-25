# pixfx_core::body

Layer: **Engine**, beside `pixfx-fighter`. Read the root `AGENTS.md` ("Layers and where code goes") first; this
file only says what this module holds. Design: `docs/superpowers/specs/2026-11-16-pixfx-body-rig-design.md`.

## Job

One rig for every pose, direction and animation. A figure (`Body`: bone lengths from a height and a head count,
by default the game's ruler) is posed by joint angles (`Pose`), placed in 3D and seen from a facing.

| Module | Holds |
|---|---|
| `dimensions` | `Body`: bone lengths; `Body::new(height, heads)`, `Body::from_ruler`; the default is `pixfx_draw::RULER` |
| `pose` | `Pose`, `Limb` (raise, spread, bend), `Head`, `Sword`, `Side`; `Pose::mirrored` |
| `camera` | `Camera` (yaw, pitch), `Dir4`, `Dir8`, the side, iso and top-down presets |
| `rig` | 3D forward kinematics and the view: `skeleton_seen(body, pose, camera)`, `skeleton(body, pose, facing)`, `Skeleton` (named joints, depth, ground), `Bone`, `facing`; two-bone IK (`reach`) |
| `clip` | `Clip`: keys, easing, loop or once, root motion, planted contacts; `pose_at` |
| `catalog` | the ready clips by name (`ground`: idle, walk, run...; every preset as `pose/<name>`) |
| `presets` | the named still poses `pixfx_pixel::pose` re-exports |
| `part`, `hull` | `Part` (17 body parts, guide colours); `hull`: the body as shapes per part, back to front |
| `paint`, `sprite` | the mannequin on the ruler: `rasterize` to a `PartMap`, `paint` with a `Look` and `Layers`; `sprite` places a clip frame in a `Cell` |
| `sheet` | `clip_sheet`: a row per direction of a `View`, a column per frame, composite, hull, bones and part images and `SheetMeta` |

## Belongs here

How a figure is posed, seen, animated and painted as a mannequin: joints, cameras, clips, the hull.

## Does not belong here

| If the code... | It goes to |
|---|---|
| draws a captioned sheet for an image model | `pixfx_pixel::pose` |
| paints the duel's hand-drawn fighter | `pixfx-fighter` |
| reads or writes files | `examples/` (`clip_sheet.rs`, `catalog.rs`; shared code in `examples/common/mod.rs`) |

## Traps

- Facing 0 is a side view facing screen right with **her right side toward the viewer**; from the front her right
  is on the viewer's left. Tests pin both.
- An elbow folds forward and a knee folds back; `spread` is outward on each side, so `mirrored` only swaps limbs.
- Existing skeletons must not move: `pixfx-pixel` tests check the rendered pose sheets and presets.

## Before you finish

```
cargo test -p pixfx-core --features body --all-targets
cargo clippy -p pixfx-core --features body --all-targets -- -D warnings
cargo test -p pixfx-pixel --all-targets      # the guide sheets and the unchanged golden
```
