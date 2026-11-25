# pixfx_pixel::pose

Read the crate instructions in [AGENTS.md](../../AGENTS.md) before editing this module.

## Pose

Pose skeletons an image model draws a character onto. A figure ([`Body`]: bone lengths from a height and a head
count) is posed by joint angles ([`Pose`], or a preset), placed in 3D, seen from any facing, and drawn into a
captioned sheet, one skeleton a cell, every limb in its own colour.

| Module | Holds |
|---|---|
| `pixfx_core::body` (re-exported) | `Body`, `Pose`, `Limb`, `Head`, `Sword`, `Side`, `skeleton`, `Skeleton`, `Bone`, `facing`: the rig lives in `pixfx_core::body`; its names stay here so specs and scripts keep working |
| `presets` | Ready poses for a sword fighter (`NAMES`, `by_name`), walk and run cycles by phase |
| `sheet` | `PoseSheet::render` (image + `pixfx_pixel::sheet::Layout` + skeletons), `colour`, `legend`, `turnaround`; the default sheet is the ruler's 64 cell with the soles on its feet row |
| `spec` | A sheet as JSON, presets with field-by-field overrides |
| `font` | The 5x7 caption font |

`examples/pose_sheet.rs` writes a sheet to files. The skill `.claude/skills/pixfx-pose/SKILL.md` is the agent's guide.

## Belongs here

Anything that makes a pose guide clearer to an image model: new presets, props, more joints (hands, fingers,
cloth points), other figures (four legs), other guide marks.

## Does not belong here

| If the code... | It goes to |
|---|---|
| cuts or reads what the model drew | `pixfx-pixel` |
| paints a fighter the game shows | `pixfx-fighter` |
| reads or writes files | `examples/` |

## Traps

- Facing 0 is a side view facing screen right with **her right side toward the viewer**; from the front her right
  is on the viewer's left. Tests pin both.
- An elbow folds forward and a knee folds back; `spread` is outward on each side, so `mirrored` only swaps limbs.
- The cell holds the figure by its pelvis (centred) and its lowest point (on the ground); a long sword or a raised
  arm can leave the cell and is clipped there. Shorten `Body::sword` or turn the pose rather than move the figure.
- One font size serves every caption on a sheet, so one long caption shrinks them all.
