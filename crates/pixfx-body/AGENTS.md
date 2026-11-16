# pixfx-body

Layer: **Engine**, beside `pixfx-fighter`. Read the root `AGENTS.md` ("Layers and where code goes") first; this
file only says what this crate holds. Design: `docs/superpowers/specs/2026-11-16-pixfx-body-rig-design.md`.

## Job

One rig for every pose, direction and animation. A figure (`Body`: bone lengths from a height and a head count,
by default the game's ruler) is posed by joint angles (`Pose`), placed in 3D and seen from a facing.

| Module | Holds |
|---|---|
| `body` | `Body`: bone lengths; `Body::new(height, heads)`, `Body::from_ruler`; the default is `pixfx_draw::RULER` |
| `pose` | `Pose`, `Limb` (raise, spread, bend), `Head`, `Sword`, `Side`; `Pose::mirrored` |
| `rig` | 3D forward kinematics and the view: `skeleton(body, pose, facing)`, `Skeleton` (named joints, depth), `Bone`, `facing` |

## Belongs here

How a figure is posed, seen, animated and painted as a mannequin: joints, cameras, clips, the hull.

## Does not belong here

| If the code... | It goes to |
|---|---|
| draws a captioned sheet for an image model | `pixfx-pose` |
| paints the duel's hand-drawn fighter | `pixfx-fighter` |
| reads or writes files | `examples/` |

## Traps

- Facing 0 is a side view facing screen right with **her right side toward the viewer**; from the front her right
  is on the viewer's left. Tests pin both.
- An elbow folds forward and a knee folds back; `spread` is outward on each side, so `mirrored` only swaps limbs.
- Existing skeletons must not move: `pixfx-pose`'s `tests/unchanged.rs` pins every preset at every facing.

## Before you finish

```
cargo test -p pixfx-body --all-targets
cargo clippy -p pixfx-body --all-targets -- -D warnings
cargo test -p pixfx-pose --all-targets      # the guide sheets and the unchanged golden
```
