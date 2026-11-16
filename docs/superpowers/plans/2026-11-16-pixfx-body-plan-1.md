# pixfx-body, plan 1: the rig, the camera, clips and the painted sprite

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** One rig in a new engine crate, `pixfx-body`, that poses a figure, sees it from any camera (side, iso, top-down, any direction), plays timed clips without foot sliding, and paints the part-coloured mannequin with its white bones as a pixel sprite on the ruler, written out by a script as layered sheets, a GIF and JSON.

**Architecture:** `pixfx-body` (engine layer, depends only on `pixfx-draw` and `libm`) takes over `Body`, `Pose` and the forward kinematics from `pixfx-pose`, which re-exports them unchanged. New modules add the camera, hands and feet, clips with two-bone IK for contacts, a starter catalog, the hull as projected geometry, and a pixel painter. A script in `crates/pixfx-body/examples/` writes files.

**Tech Stack:** Rust 2024, `libm`, `serde` (feature), `pixfx-draw` (`Frame`, `Rgba`, `RULER`, `frame_hash`); dev: `pixfx-export` (`write_png_rgba`, `gif::write_gif`), `pixfx-core` (`render::Rendered`), `serde_json`.

**Spec:** `docs/superpowers/specs/2026-11-16-pixfx-body-rig-design.md`

This is plan 1 of 4. Plan 2: the rest of the catalog, family by family. Plan 3: smooth anti-aliased guide sheets in `pixfx-pose` (A). Plan 4: props and contacts (ledge, ladder, rope, wall).

## Global Constraints

- Engine purity: no I/O, no clock, no `f64::sin`/`sqrt`/`round` (clippy `disallowed_methods`): `libm::*`, `pixfx_draw::math::js_round`.
- clippy `unwrap_used`, `expect_used`, `panic` denied; `unsafe_code` forbidden (workspace lints, `[lints] workspace = true`).
- Default figure is the ruler: `Body::default() == Body::from_ruler(&pixfx_draw::RULER)`, 64 cell, feet row 60. Never their 50 px.
- One game pixel per pixel in B. No mixels.
- Every direction is rendered from the 3D rig, never flipped.
- `skeleton()` gives the same numbers for every existing pose after every task.
- Cameras: side pitch 0; iso pitch 30; topdown pitch 60; orthographic.
- Clips play in place; root motion only in data. Clip cell: bounds over every frame and direction, rounded up to a multiple of 8, never under 64.
- Occluded bones hidden by depth in B.
- Goldens: each catalog clip at `side(E)` and `topdown(S)`.
- Shared tree: commit only your own files, through a temporary index (`GIT_INDEX_FILE` + `read-tree HEAD` + `add` + `write-tree` + `commit-tree` + `update-ref`), never `git commit -a`. End messages with `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.
- Tests per crate: `cargo test -p pixfx-body --all-targets`, `cargo clippy -p pixfx-body --all-targets -- -D warnings`, and the same for `pixfx-pose` when it changes. Never `--workspace`.

## Review Focus

1. A NaN or infinite angle, ease, time or camera value from a bad JSON clip must still give a finite skeleton and a paintable frame (`fin()` everywhere). Test in Task 4 and Task 6.
2. An IK target out of the limb's reach (contact too far) must straighten the limb toward it, never NaN. Test in Task 4.
3. A clip with one key, or a frame index past the end, must still sample (once: hold the last; loop: wrap). Test in Task 4.
4. A pose that throws a limb straight at the camera (zero-length projected bone) must still paint the joint as a dot, no gap and no division by zero. Test in Task 6.
5. Lying and tumbling poses (`lie` 90/-90, `roll`) under the top-down camera must keep the figure inside its cell and on the ground pivot. Test in Task 7.

---

### Task 0: Gate on the uncommitted `pixfx-pose` edits

`crates/pixfx-pose` has uncommitted edits from a session that is no longer running (`body.rs`: `Body::from_ruler`, `Default` = ruler; `sheet.rs`, `spec.rs`, `tests/pose.rs`, `AGENTS.md`; 52 lines). Task 1 moves `body.rs`.

- [ ] **Step 1:** `ListAgents`. If a session owns `crates/pixfx-pose`, `SendMessage` it: "moving body/pose/rig out of pixfx-pose into a new pixfx-body; please commit or tell me to build on your edits" and wait.
- [ ] **Step 2:** With no owner reachable, ask the user once: commit those edits as they stand (they make the default figure the ruler, which the spec requires) and build on them, or wait. Do not start Task 1 without an answer.
- [ ] **Step 3:** Run `cargo test -p pixfx-pose`; expected PASS on the baseline Task 1 starts from.

### Task 1: `pixfx-body` takes the rig; `pixfx-pose` re-exports it

**Files:**
- Create: `crates/pixfx-body/Cargo.toml`, `crates/pixfx-body/src/lib.rs`, `crates/pixfx-body/AGENTS.md` (+ `CLAUDE.md` symlink or copy as other crates do)
- Move (`git mv`): `crates/pixfx-pose/src/{body,pose,rig}.rs` → `crates/pixfx-body/src/`; `crates/pixfx-pose/tests/pose.rs` → `crates/pixfx-body/tests/rig.rs`
- Modify: `crates/pixfx-pose/Cargo.toml` (dep `pixfx-body`), `crates/pixfx-pose/src/lib.rs` (`pub use pixfx_body::{Body, Head, Limb, Pose, Side, Sword, Bone, LimbPoints, Point, Skeleton, facing, skeleton};`), `crates/pixfx-pose/AGENTS.md`, root `CLAUDE.md` and `README.md` crate tables, `.claude/skills/pixfx/SKILL.md` layer table.
- Create: `crates/pixfx-body/tests/goldens/skeletons.json`, `crates/pixfx-body/tests/unchanged.rs`

**Interfaces:**
- Produces: `pixfx_body::{Body, Pose, Limb, Head, Sword, Side, Skeleton, Point, LimbPoints, Bone, skeleton, facing}` with today's signatures; `presets` stays in `pixfx-pose` until Task 5.

`Cargo.toml` mirrors `pixfx-fighter`'s: deps `libm`, `pixfx-draw`, `serde` (required for now: `Pose` derives it and `pixfx-pose`'s JSON specs need it); dev-deps `pixfx-export`, `pixfx-core`, `serde_json`.

- [ ] **Step 1: Pin today's numbers before moving anything.** A test `every_preset_at_every_facing_keeps_its_joints` in `crates/pixfx-pose/tests/unchanged.rs` that, when `PIXFX_BLESS=1`, writes `skeleton(&Body::default(), &presets::by_name(n, 0.25)?, f)` for every `presets::NAMES` × `facing::ALL` to `skeletons.json`, and otherwise asserts every joint equal within 1e-9. Run once with `PIXFX_BLESS=1`, then without: PASS.
- [ ] **Step 2:** Move the files and wire the re-exports. `unchanged.rs` and its golden stay in `pixfx-pose` until Task 5 moves the presets; they run against the re-exported rig.
- [ ] **Step 3:** `cargo test -p pixfx-body --all-targets && cargo test -p pixfx-pose --all-targets`: every existing test and `unchanged` PASS. Clippy clean on both.
- [ ] **Step 4:** Commit "body: pixfx-body takes the rig from pixfx-pose".

### Task 2: The camera

**Files:** Create `crates/pixfx-body/src/camera.rs`; Modify `rig.rs`; Test `crates/pixfx-body/tests/camera.rs`

**Interfaces:**
- Produces:
  - `pub struct Camera { pub yaw: f64, pub pitch: f64 }` (degrees; yaw as today's facing; pitch 0 side, 90 straight down), `Serialize/Deserialize`, `Copy`.
  - `impl Camera { pub fn side(dir: Dir4) -> Camera; pub fn iso(dir: Dir8) -> Camera; pub fn topdown(dir: Dir8) -> Camera }` with pitches 0, 30, 60.
  - `pub enum Dir4 { E, S, W, N }`, `pub enum Dir8 { E, SE, S, SW, W, NW, N, NE }`, each with `fn yaw(self) -> f64`: E 0, SE 45, S 90 (faces the viewer), SW 135, W 180, NW 225, N 270, NE 315; `fn all() -> [Self; n]`.
  - `pub fn skeleton_seen(body: &Body, pose: &Pose, camera: Camera) -> Skeleton`; `skeleton(body, pose, f)` becomes `skeleton_seen(body, pose, Camera { yaw: f, pitch: 0.0 })`.
  - `Skeleton` gains `pub ground: Point`: the projected ground point under the pelvis (the pivot). `pixfx-pose`'s `skeletons.json` output gains this field; the `unchanged` test compares joints only.

Pitch tilts the view frame about the screen x axis after yaw: screen y = up·cos(p) − forward-into-screen·sin(p) projection; depth adjusted the same way. Exact maths left to the builder; tests pin behaviour.

- [ ] **Step 1: Failing tests:**
  - `pitch_zero_is_todays_skeleton`: for every preset × facing, `skeleton_seen(.., Camera{yaw:f,pitch:0})` == `skeleton(.., f)`.
  - `looking_down_shortens_the_standing_figure`: idle at `topdown(S)`: height (head top to ground point) < 38 and > 38·cos(60°)−1; at `iso(S)`: between that and 38.
  - `top_down_south_sees_the_face`: `topdown(Dir8::S)` → `nose_visible`; `topdown(Dir8::N)` → not.
  - `a_symmetric_pose_from_west_mirrors_east`: rest pose, `side(W)` joints == `side(E)` joints with x negated about the pelvis, and right/left swapped.
  - `a_bad_camera_is_finite`: yaw/pitch NaN and ∞ → every joint finite.
- [ ] **Step 2:** Run; FAIL (no `camera` module).
- [ ] **Step 3:** Implement.
- [ ] **Step 4:** Run `pixfx-body` and `pixfx-pose` tests; PASS, `unchanged` still PASS.
- [ ] **Step 5:** Commit "body: a camera with pitch: side, iso, top-down in 4 and 8 directions".

### Task 3: Hands, feet and a chest twist

**Files:** Modify `pose.rs`, `body.rs`, `rig.rs`; Test `crates/pixfx-body/tests/extremities.rs`

**Interfaces:**
- Produces:
  - `Body` gains `hand: f64` (stub beyond the hand point; default `RULER.hand` = 2) and `foot: f64` (forward stub from the sole; default 1.5 × `RULER.limb`), both `#[serde(default)]` so old JSON loads.
  - `Pose` gains `chest: f64` (shoulders twist on top of `twist`, default 0), `roll: f64` (whole body turned about her forward axis, cartwheel, default 0), `right_foot`/`left_foot: f64` (ankle pitch, toes up positive, default 0), `right_hand`/`left_hand: f64` (wrist bend, default 0); all `#[serde(default)]`.
  - `LimbPoints` gains `tip: Point` (fingertips or toes); `Bone` gains `RightHand, LeftHand, RightFoot, LeftFoot`, emitted by `bones()` after today's bones and before `Sword`.
- [ ] **Step 1: Failing tests:** `new_fields_at_zero_change_no_existing_joint` (`unchanged` already covers; assert also in this file for idle); `a_foot_points_forward_along_the_ground` (idle side E: `right_leg.tip.x > right_leg.end.x`, same y within 0.5); `toes_up_lifts_the_tip`; `a_roll_of_180_stands_her_on_her_head` (head below pelvis); `chest_turns_the_shoulders_not_the_hips`.
- [ ] **Step 2–4:** Run (FAIL), implement, run (PASS incl. `pixfx-pose` and `unchanged`).
- [ ] **Step 5:** Commit "body: hands, feet, chest twist and roll".

### Task 4: Clips, with two-bone IK for contacts

**Files:** Create `crates/pixfx-body/src/clip.rs`, `crates/pixfx-body/src/ik.rs`; Test `crates/pixfx-body/tests/clip.rs`

**Interfaces:**
- Produces:
  - `pub enum Ease { Linear, InOut, In, Out, Hold }` (`Hold` keeps the earlier key until the next).
  - `pub struct Key { pub at: u32 /* frame */, pub pose: Pose, pub ease: Ease }`
  - `pub enum Mode { Loop, Once }`
  - `pub enum Limb4 { RightArm, LeftArm, RightLeg, LeftLeg }`
  - `pub struct Contact { pub limb: Limb4, pub from: u32, pub to: u32 }`: the limb's end stays where it was at frame `from` (in body space, root motion removed) through frame `to`.
  - `pub struct Clip { pub name: String, pub frames: u32, pub fps: u32, pub mode: Mode, pub keys: Vec<Key>, pub root: Vec<(u32, f64, f64)> /* frame, dx along ground, dy height */, pub contacts: Vec<Contact> }`, serde.
  - `impl Clip { pub fn pose_at(&self, body: &Body, frame: u32) -> Pose; pub fn root_at(&self, frame: u32) -> (f64, f64) }`
  - `ik::reach(root: V3, upper: f64, lower: f64, target: V3, pole: V3) -> (V3, V3)` (mid, end), analytic law of cosines; out of reach → straight toward target. `pose_at` turns the solved points back into `Limb` angles (`raise`, `spread`, `bend`) so the result is still a `Pose`.
- Interpolation: angle fields lerped with the ease of the earlier key; `sword`, `lift` lerped; in `Loop`, the last key eases into the first at `frames`; frame past the end wraps (Loop) or holds (Once).
- [ ] **Step 1: Failing tests:** `a_frame_on_a_key_is_that_keys_pose`; `halfway_with_linear_is_the_mean`; `a_loop_flows_from_its_last_frame_into_its_first` (pose at `frames` == pose at 0, and the step from frames−1 to 0 is no larger than the largest step inside); `once_holds_the_last_key`; `one_key_is_a_still_clip`; `a_planted_foot_does_not_slide` (walk-like clip with a right-leg contact frames 0..3: `right_leg.end` of `skeleton(.., pose_at(f), 0)` within 0.5 px across those frames); `an_unreachable_target_straightens_the_leg_and_stays_finite`; `a_bad_number_in_a_clip_stays_finite` (NaN key angle, NaN root).
- [ ] **Step 2–4:** FAIL, implement, PASS.
- [ ] **Step 5:** Commit "body: clips with keys, easing, loops, root motion and planted contacts".

### Task 5: The catalog, with the grounded-movement family

**Files:** Create `crates/pixfx-body/src/catalog/mod.rs`, `crates/pixfx-body/src/catalog/ground.rs`, `crates/pixfx-body/src/catalog/combat.rs`; Move `crates/pixfx-pose/src/presets.rs` content into `catalog/combat.rs` (single poses as one-key clips) and the walk and run cycles into `ground.rs`; `pixfx-pose` keeps `pub mod presets` as a thin re-export (`by_name`, `NAMES`, each fn) so its specs work. Move `unchanged.rs` + golden into `pixfx-body/tests/`. Test `crates/pixfx-body/tests/catalog.rs`.

**Interfaces:**
- Produces: `pub fn catalog::by_name(name: &str) -> Option<Clip>`, `pub const catalog::NAMES: &[&str]`, `pub fn catalog::family(name: &str) -> Option<&'static str>`.
- This plan's clips (our own key poses; frames and modes as the spec's reference range, 6 to 12 frames): `idle` (8, loop), `walk` (8, loop), `run` (8, loop), `run_start` (4, once), `run_stop` (5, once), `sprint` (6, loop), `skid` (5, once), `turn` (6, once), `crouch` (6, loop), `crouch_walk` (8, loop), plus every current preset (`rest idle walk run guard windup slash thrust hurt kneel crouch jump down cast`) as one-key clips under `pose/<name>` so names do not clash. Walk and run carry foot contacts. 12 fps default.
- [ ] **Step 1: Failing tests:** `every_name_gives_a_clip`; `every_clip_is_finite_at_every_frame_from_every_camera` (all `Dir8` × side/iso/topdown); `every_loop_flows_into_its_first_frame`; `feet_do_not_slide_in_walk_and_run`; `presets_through_pixfx_pose_still_match` (`unchanged` passes from its new home).
- [ ] **Step 2–4:** FAIL, implement, PASS on both crates.
- [ ] **Step 5:** Commit "body: the catalog, grounded movement and the combat poses as clips".

### Task 6: The hull as geometry

**Files:** Create `crates/pixfx-body/src/hull.rs`, `crates/pixfx-body/src/part.rs`; Test `crates/pixfx-body/tests/hull.rs`

**Interfaces:**
- Produces:
  - `pub enum Part { Head, Neck, Chest, Pelvis, RightUpperArm, RightForearm, RightHand, LeftUpperArm, LeftForearm, LeftHand, RightThigh, RightShin, RightFoot, LeftThigh, LeftShin, LeftFoot, Sword }` with `fn id(self) -> u8` (1-based, 0 = empty) and `fn guide(self) -> Rgba` (one hue per part, the reference's scheme: head cyan, chest orange to red, arms pink to violet, pelvis brown, legs blue/green/lime by segment; right and left distinguishable). `ALL: [Part; 17]`.
  - `pub enum Shape { Capsule { a: (f64, f64), b: (f64, f64), ra: f64, rb: f64 }, Disc { c: (f64, f64), r: f64 }, Quad([(f64, f64); 4]) }`
  - `pub struct HullShape { pub part: Part, pub shape: Shape, pub depth: f64 }`
  - `Body` gains radii: `limb_r` (default `RULER.limb`/2 = 1.5), `torso_w` (default `RULER.width`·0.35), `pelvis_w`; serde default.
  - `pub fn hull(body: &Body, skel: &Skeleton) -> Vec<HullShape>`, sorted back to front by `depth`. Chest and pelvis are quads from shoulders/hips projected; head a disc; limbs capsules.
- [ ] **Step 1: Failing tests:** `every_part_but_the_sword_is_in_the_hull_of_idle`; `the_far_arm_is_drawn_before_the_chest_from_the_side` (side E: `LeftUpperArm` index < `Chest` index); `from_behind_the_order_turns` (side W: right arm before chest); `an_arm_pointed_at_the_camera_is_a_disc_sized_capsule` (zero-length capsule allowed, finite); `a_bad_skeleton_gives_finite_shapes`.
- [ ] **Step 2–4:** FAIL, implement, PASS.
- [ ] **Step 5:** Commit "body: the hull, one shape per body part, back to front".

### Task 7: Painting the sprite (B)

**Files:** Create `crates/pixfx-body/src/paint.rs`, `crates/pixfx-body/src/look.rs`; Test `crates/pixfx-body/tests/paint.rs`, `crates/pixfx-body/tests/ruler.rs`, `crates/pixfx-body/tests/goldens.rs` + `tests/goldens/frame_hashes.json`

**Interfaces:**
- Produces:
  - `pub struct PartMap { pub w: u32, pub h: u32, pub ids: Vec<u8>, pub shade: Vec<i8> /* -1 dark, 0 base, 1 light */ }`
  - `pub fn rasterize(shapes: &[HullShape], w: u32, h: u32) -> PartMap`: one game pixel per pixel, a pixel belongs to a shape when its centre is inside it, later (nearer) shapes overwrite; every shape paints at least its centre pixel. Shade: light on the side facing the light direction (up-left), dark on the opposite edge, one pixel wide.
  - `pub struct Look { pub ramps: [[Rgba; 3]; 17] /* dark, base, light per Part */, pub bone: Option<Rgba> /* default white */ }`, `Look::mannequin()` from `Part::guide`.
  - `pub struct Layers { pub hull: bool, pub bones: bool }`
  - `pub fn paint(map: &PartMap, skel: &Skeleton, look: &Look, layers: Layers) -> Frame`: hull colours into `color`; bones as 1-px lines (`pixfx_draw` line primitive) drawn per part only where that pixel's id is that part (occluded bones hidden). No outline. `light` stays empty.
  - `pub struct Sprite { pub frame: Frame, pub parts: PartMap, pub pivot: (i32, i32) }`; `pub fn sprite(body: &Body, clip: &Clip, frame: u32, camera: Camera, cell: (u32, u32), look: &Look, layers: Layers) -> Sprite`: places the skeleton with the ground point at `(cell.0/2, cell.1 - 4)` (the ruler's feet row 60 in a 64 cell).
  - `pub fn clip_cell(body: &Body, clip: &Clip, cameras: &[Camera]) -> (u32, u32)`: bounds over every frame × camera, rounded up to 8, min 64.
- [ ] **Step 1: Failing tests:**
  - `ruler.rs`: idle `side(E)` frame 0 in 64 cell: painted rows 38 ± 1, lowest painted row 60, head width 7 ± 1, nothing outside the cell.
  - `paint.rs`: `every_pixel_of_a_part_has_a_colour_of_its_ramp`; `a_hidden_bone_is_not_drawn` (side E: no pixel of the left-arm bone colour over a chest id); `bones_off_leaves_only_hull_colours`; `west_is_rendered_not_flipped` (`walk` frame 2, side W != mirror of side E pixels, since the far leg differs); `a_tumble_from_above_stays_in_its_cell` (`pose/down` and a `roll: 180` pose at `topdown` every `Dir8`: no pixel clipped); `a_bad_pose_still_paints`.
  - `goldens.rs`: for every catalog clip, `frame_hash` of every frame at `side(E)` and `topdown(S)` equals the JSON; `PIXFX_BLESS=1` writes it.
- [ ] **Step 2–4:** FAIL, implement, bless once, PASS. Read 3 sprites as PNG by eye (Task 8 script) before blessing.
- [ ] **Step 5:** Commit "body: paint the mannequin with its bones, on the ruler".

### Task 8: The `clip_sheet` and `catalog` scripts, docs and skill

**Files:** Create `crates/pixfx-body/examples/clip_sheet.rs`, `crates/pixfx-body/examples/catalog.rs`, `crates/pixfx-body/examples/common/mod.rs`; Create `.claude/skills/pixfx-body/SKILL.md`; Modify `.claude/skills/pixfx/SKILL.md` (which-skill row), `.claude/skills/pixfx-render/SKILL.md` (point to the scripts), `README.md` scripts table, `CHANGELOG.md`.

**Interfaces:**
- `cargo run -p pixfx-body --example clip_sheet -- <clip> [--camera side|iso|topdown] [--layers hull,bones] [--out DIR]` writes, per direction row (4 for side, 8 otherwise) × frame column: `composite.png`, `hull.png` (hull only), `bones.png` (bones only on transparent), `parts.png` (each pixel `Part::guide` of its id, no shading), `<clip>.gif` (first direction, through `pixfx_core::render::Rendered { w, h, fps, origin: pivot, margin: 0, frames }` and `pixfx_export::gif::write_gif(path, &r, 4)`), and `<clip>.json`: `{ name, fps, mode, cell: [w,h], pivot: [x,y], directions: [..], frames: [{ dir, index, rect: [x,y,w,h], duration_ms, root: [dx,dy], joints: {..}, sockets: {} }] }` plus Aseprite `meta.frameTags` per direction.
- `catalog` runs `clip_sheet`'s library path for every `catalog::NAMES` into `target/body/<name>/`.
- [ ] **Step 1:** Write the scripts; `cargo run -p pixfx-body --example clip_sheet -- run --camera side` and `-- walk --camera topdown`; `Read` both `composite.png` files: a part-coloured mannequin with white bones, 4 and 8 rows, feet on one line, W and N turned in 3D.
- [ ] **Step 2:** Write the skill (`pixfx-body`: when to use it, cameras, clips, layers, the scripts, how to add a clip), update the other docs.
- [ ] **Step 3:** `cargo test -p pixfx-body --all-targets`, `cargo clippy -p pixfx-body --all-targets -- -D warnings`, same for `pixfx-pose`: PASS.
- [ ] **Step 4:** Commit "body: clip sheets with hull, bones and part map, GIF and JSON".
