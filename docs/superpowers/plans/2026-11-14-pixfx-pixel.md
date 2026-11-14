# pixfx-pixel Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A pixel toolkit for agents: recover any picture's true pixel grid, reduce and snap its colours, read it
as round-trip text, view it within a token budget, critique it, lay out sheets, split sprites into parts, and drive
all of it from `pixfx pixel <verb>`.

**Architecture:** Pure functions in `crates/pixfx-pixel` (one module per job, each with its own test file); a thin
CLI in `crates/pixfx-cli/src/pixel.rs` (parsing, files, printing); scripts in `crates/pixfx-pixel/examples/`; a
skill in `skills/pixfx-pixel/SKILL.md`.

**Tech Stack:** Rust 2024, `libm`, `pixfx-draw` (`Rgba`, `RgbaImage`, `to_hex`, `hex`), `pixfx-export` (PNG, in the
CLI and as a dev-dependency only).

**Spec:** `docs/superpowers/specs/2026-11-14-pixfx-pixel-design.md`

## Global Constraints

- No `unwrap`, `expect`, `panic`, `unsafe` (workspace lints). Float maths through `libm`; rounding through
  `pixfx_draw::math::js_round` or explicit `libm::floor(x + 0.5)`.
- Library: no I/O, no clock, no RNG, no `HashMap` iteration reaching output. Every tie-break is a total order on
  integer keys. Sums in a fixed order.
- Alpha is 1-bit at 128 (`OPAQUE` in `lib.rs`).
- Nothing in the engine depends on `pixfx-pixel`.
- Crate tests: `cargo test -p pixfx-pixel`; CLI: `cargo test -p pixfx-cli`; clippy clean with `--all-targets`.
- Commits: only this plan's paths, through a temporary index (`GIT_INDEX_FILE`), never a plain `git commit -a`;
  other sessions share the tree.

## Review Focus

1. A picture with a transparent background and no grid at all (a hand-drawn 1x sprite): `recover` returns pitch 1
   and the picture unchanged, not a made-up grid. Test in Task 2.
2. A picture whose colours number in the thousands (an image model's): `clean` without `--palette` reduces to at
   most 32 colours and `read` works. Test in Task 3 and Task 9.
3. A drawn sheet whose size differs from the layout (a model returns 1024x1024 for a 840x840 layout): `cut` scales
   the cell rectangles. Test in Task 7.
4. Text an agent edited with a letter not in its legend: `parse` refuses it naming the row and column. Test in
   Task 4.
5. A part spec whose parent does not touch the part: `split` refuses it, naming both. Test in Task 8.

---

### Task 1: OKLab and the real-model fixtures

**Files:**
- Create: `crates/pixfx-pixel/src/oklab.rs`, `crates/pixfx-pixel/tests/oklab.rs`
- Create: `crates/pixfx-pixel/tests/fixtures/` (spike figures, cropped, each `<name>.png` with `<name>.txt` holding
  `pitch <px>` and `cells <w>x<h>`)
- Modify: `crates/pixfx-pixel/src/lib.rs` (module list for every later task, each module a stub until its task)

**Interfaces:**
- Produces: `pub struct Lab { pub l: f64, pub a: f64, pub b: f64 }`, `pub fn lab(c: Rgba) -> Lab`,
  `pub fn delta_e(a: Lab, b: Lab) -> f64` (Euclidean).

- [ ] Tests: `lab(#ffffff)` has `l` within 1e-6 of 1.0 and `a`,`b` within 1e-6 of 0; `lab(#000000).l == 0`;
  `delta_e(x, x) == 0`; red→orange ΔE < red→blue ΔE; the result for 4096 colours is the same on two calls.
- [ ] Implement with Ottosson's matrices, sRGB→linear by a 256-entry table built once per call site (a `const fn`
  or a computed array), `libm::cbrt`.
- [ ] Fixtures: crop each spike picture `target/art/spike-wanglin/{draft-style,draft-grid,hero-base,hero-style,
  hero-grid}/image-01.png` to its figure box plus one pitch of margin; record pitch and cells from the spike
  (`11.9 27x40`, `8.0 29x46`, `10.22 50x76`, `10.22 34x60`, `14.65 41x57`); check each by eye with `pixfx pixel
  view` once Task 5 lands and correct the `.txt` if the count is off.
- [ ] Run `cargo test -p pixfx-pixel --test oklab` → pass. Commit.

### Task 2: Grid recovery (`recover`)

**Files:** Create `crates/pixfx-pixel/src/recover.rs`, `crates/pixfx-pixel/tests/recover.rs`

**Interfaces:**
- Consumes: `oklab::{lab, delta_e}`.
- Produces:
  - `pub struct Grid { pub cols: Vec<u32>, pub rows: Vec<u32> }`: cut positions in source pixels, first 0, last
    the width/height; cell `i` spans `cols[i]..cols[i+1]`.
  - `pub struct Recovered { pub grid: Grid, pub pitch: f64, pub picture: RgbaImage, pub confidence: Vec<f64>,
    pub total: f64 }`: `picture` is one pixel per cell; `confidence` row by row, 0..1; `total` their mean.
  - `pub fn recover(img: &RgbaImage) -> Recovered`
  - `pub fn collapse(img: &RgbaImage, grid: &Grid) -> (RgbaImage, Vec<f64>)`
  - `pub fn fit_height(img: &RgbaImage, height: u32) -> RgbaImage`: resample cell by cell to `height` rows, width
    in proportion; each target cell takes the majority source colour of its block (ties: lowest RGBA key).

- [ ] Tests, each on a seeded-by-index synthetic 20x24 sprite (colours from a fixed list, a transparent margin):
  - `nearest_upscales_x2_to_x12_recover_exactly`: `recover(upscale(s, k)).picture == s` for k in 2..=12.
  - `fractional_scale_7_5_recovers_size`: 20x24 at 7.5 → picture 20x24, ≥ 95% of cells equal.
  - `offset_first_cell`: a k=6 upscale with 3 columns and 2 rows cropped off the left and top → picture 20x24
    with the first column partial, ≥ 95% equal.
  - `bilinear_and_noise`: k=8 upscale blurred by a 3x3 box and ±6 noise from a fixed sequence → ≥ 90% equal.
  - `no_grid_is_pitch_1`: the 20x24 sprite itself → `pitch == 1.0`, picture unchanged.
  - `fixtures_recover_their_counted_cells`: each fixture's picture within ±1 cell of its `.txt` count in each
    axis.
  - `deterministic`: `recover` twice on each fixture gives equal `Recovered`.
  - `fit_height_halves`: a 2x upscale `fit_height(.., 24)` equals the original.
- [ ] Implement per the spec's stages:
  1. Forward-difference ΔE edge profile per column and row, alpha change counted as 1.0, summed in index order.
  2. Autocorrelation of each profile minus its mean, lags 2..len/3; first local max above 0.2× lag 0; refine as
     `Σ k·lag_k / Σ k²` over harmonics k=1..5 (each harmonic's lag the argmax within ±2 of `k·p`).
  3. Candidates p, p/2, p/3 (those ≥ 1.5), and 1.0.
  4. Per candidate and axis, dynamic programming over cut positions: widths in `[floor(p·0.65), ceil(p·1.35)]`,
     score `Σ profile[cut] − 0.5·mean(profile)·((w−p)/p)²` per cell; the first and last cell may be any width
     ≤ the upper bound. Ties: the earlier cut.
  5. Score each candidate by `collapse` then repaint: mean ΔE between source and repaint over the middle half of
     each cell; lowest wins; ties to the larger pitch.
  6. `collapse`: the middle half of each cell votes; the exact RGBA with > 50% wins, else the OKLab medoid; alpha
     by majority at 128. Confidence per cell = share of voters within ΔE 0.04 of the winner.
- [ ] Run `cargo test -p pixfx-pixel --test recover` → pass. Commit.

### Task 3: Colours (`reduce`, `snap`)

**Files:** Create `crates/pixfx-pixel/src/reduce.rs`, `crates/pixfx-pixel/tests/reduce.rs`

**Interfaces:**
- Consumes: `oklab`.
- Produces: `pub fn reduce(img: &RgbaImage, max_colors: usize, threshold: f64) -> RgbaImage` (defaults the CLI
  passes: 32, 0.03); `pub enum SnapMode { Nearest, Lightness }`;
  `pub fn snap(img: &RgbaImage, palette: &[Rgba], mode: SnapMode) -> RgbaImage`.

- [ ] Tests: 4000 distinct colours → `colors(reduce(..)).len() <= 32`; output colours are a subset of the input's;
  the same picture with its pixels permuted (a fixed permutation) gives the same set of output colours; a
  10-step grey ramp reduced to 4 keeps dark-to-light order; `snap` Nearest maps each pixel to its closest palette
  colour by ΔE; `snap` Lightness on a dark-only picture uses the palette's darkest to lightest range; transparency
  kept.
- [ ] Implement as the spec says: count colours (alpha ≥ 128) keyed by packed RGB; order (count desc, key asc);
  leader merge at `threshold`; then merge the closest pair of clusters (ties: lower keys) until ≤ `max_colors`;
  each cluster shown by the member nearest its count-weighted OKLab centre. Lightness: 2nd-98th percentile of L
  weighted by count, stretched onto the palette's L range; distance `4·ΔL² + Δa² + Δb²`.
- [ ] Run the crate tests → pass. Commit.

### Task 4: Text round trip and named legends

**Files:** Modify `crates/pixfx-pixel/src/show.rs` (split: `text` stays, `zoom` moves to `view.rs` in Task 5);
Test `crates/pixfx-pixel/tests/text.rs`

**Interfaces:**
- Produces: `pub fn text_named(map: &Map, palette: &[Rgba], names: &[&str]) -> Result<String, TooManyColors>`
  (legend line `a robe0 #f4efe2` when a name is given, else `a #f4efe2`; `text` calls it with `&[]`);
  `pub fn parse(text: &str) -> Result<(Map, Vec<Rgba>), ParseError>`;
  `pub struct ParseError { pub line: usize, pub message: String }` with `Display`.

- [ ] Tests: `parse(text(m, p)) == (m, p)` for 50 maps from a fixed sequence (sizes 1x1 to 70x12, up to 62
  colours); same with names; `parse` ignores the column-number lines and row numbers; a letter outside the
  legend → `Err` whose message names the row and column (Review Focus 4); a row of the wrong width → `Err`.
- [ ] Implement `parse`: head line `WxH, ...`; legend lines `<letter> [name] #rrggbb`; skip number lines (only
  digits and spaces); grid rows `<row> <cells>`.
- [ ] Run the crate tests → pass. Commit.

### Task 5: View within a budget

**Files:** Create `crates/pixfx-pixel/src/view.rs` (with `zoom` moved from `show.rs`), `tests/view.rs`

**Interfaces:**
- Produces: `pub struct ViewOptions { pub max_side: u32, pub max_scale: u32, pub region: Option<[u32; 4]>,
  pub every: u32 }` with `Default` (512, 16, None, 8); `pub struct View { pub image: RgbaImage, pub scale: u32,
  pub overview: bool }`; `pub fn view(map: &Map, palette: &[Rgba], opts: &ViewOptions) -> View`; `zoom` keeps its
  signature and is what `view` calls when scale ≥ 2.

- [ ] Tests: a 16x16 map → scale 16 and image ≤ 512 a side with grid lines; a 64x64 map → scale 7 (largest
  integer with `64·(s+1)+1 ≤ 512`); a 900x900 map → `overview == true`, image ≤ 512 a side, area-averaged, no grid
  lines; `region` crops before scaling; grid lines only when scale ≥ 8 (below, cells touch).
- [ ] Implement; scale = min(max_scale, largest fitting), overview when even scale 1 does not fit.
- [ ] Run the crate tests → pass. Commit.

### Task 6: Critique

**Files:** Create `crates/pixfx-pixel/src/critique.rs`, `tests/critique.rs`

**Interfaces:**
- Produces: `pub enum Severity { Error, Warning, Note }`; `pub struct Issue { pub check: &'static str,
  pub severity: Severity, pub cells: Vec<(u32, u32)>, pub message: String, pub fix: String }`;
  `pub struct Report { pub issues: Vec<Issue>, pub score: u32 }` (100 − 30/error − 12/warning − 4/note, floor 0);
  `pub fn critique(img: &RgbaImage, intended: &[(u32, u32)]) -> Report`;
  `pub fn critique_frames(frames: &[RgbaImage]) -> Report`.

- [ ] Tests, one fixture each built in code, the defect found at the right cells: `colour_budget` (limit
  `clamp(√opaque·1.2, 8, 32)`), `near_duplicate` (ΔE < 0.02), `semi_transparent` (alpha 1..127 on > 5% of
  painted), `orphan` (no same-colour 4-neighbour and every neighbour ΔE > 0.15; `intended` silences it), `noise`
  (> 30% of cells differ from ≥ 3 of 4 neighbours), `weak_outline` (< 50% of edge cells darker than L 0.35),
  `jaggies` (L-shaped stair steps > 8), `pillow_shading` (correlation of L with distance-to-edge > 0.55 and with a
  top-left direction < 0.25 in absolute value); frames: `volume_drift` (> 22%), `ground_drift` (bottom row moves
  > 2), `palette_drift`, `empty_frame`, `repeated_frame`. A fighter pose from `pixfx poses thunder` (copied into
  `tests/fixtures/thunder-idle.png`) has no `Error`.
- [ ] Implement; each check a private function returning `Vec<Issue>`; issues ordered by (severity, check name,
  first cell).
- [ ] Run the crate tests → pass. Commit.

### Task 7: Sheets

**Files:** Create `crates/pixfx-pixel/src/sheet.rs`, `tests/sheet.rs`

**Interfaces:**
- Produces: `pub struct Layout { pub cell_w: u32, pub cell_h: u32, pub cols: u32, pub rows: u32, pub gap: u32,
  pub scale: u32 }`; `pub fn size(&Layout) -> (u32, u32)`; `pub fn cell_rect(&Layout, col: u32, row: u32) ->
  [u32; 4]`; `pub const GUIDE: Rgba` (#ff00ff) and `pub const BORDER: Rgba` (#00ffff);
  `pub fn blank(layout: &Layout, pixel_lines: bool) -> RgbaImage`;
  `pub enum Guide { Pivot { x: u32, y: u32 }, Ground { y: u32 }, Bones(Vec<[(u32, u32); 2]>),
  Sprite(RgbaImage) }`; `pub fn guide(sheet: &mut RgbaImage, layout: &Layout, col: u32, row: u32, g: &Guide)`;
  `pub fn directions(sprite: &RgbaImage, scale: u32) -> (Layout, RgbaImage)` (3x3, sprite in the centre);
  `pub fn frames(n: u32, cell_w: u32, cell_h: u32, scale: u32) -> Layout`;
  `pub fn cut(drawn: &RgbaImage, layout: &Layout) -> Vec<RgbaImage>` (row by row; cell rects scaled by
  `drawn.w / size.w` and `drawn.h / size.h`; `GUIDE` and `BORDER` pixels within ΔE 0.1 made transparent).
- [ ] Tests: `blank` borders sit on cell edges; `guide` Bones draws only in `GUIDE` inside its cell; a sheet filled
  with 6 known pictures cuts back to them exactly; the same sheet scaled to 1.2× by nearest-neighbour cuts back to
  pictures that `recover` brings back to the originals (Review Focus 3); `directions` puts the sprite in cell (1,1).
- [ ] Run the crate tests → pass. Commit.

### Task 8: Parts

**Files:** Create `crates/pixfx-pixel/src/parts.rs`, `tests/parts.rs`

**Interfaces:**
- Produces: `pub struct Component { pub cells: usize, pub bbox: [u32; 4], pub colors: Vec<usize> }`;
  `pub fn components(map: &Map) -> Vec<Component>` (8-connected, ordered by first cell row by row);
  `pub enum Rule { Box([u32; 4]), Colors { colors: Vec<usize>, within: Option<[u32; 4]> }, Rest }`;
  `pub struct PartSpec { pub name: String, pub parent: Option<String>, pub rule: Rule }`;
  `pub struct Part { pub name: String, pub parent: Option<String>, pub map: Map, pub bbox: [u32; 4],
  pub pivot: Option<(f64, f64)> }`; `pub fn split(map: &Map, spec: &[PartSpec]) -> Result<Vec<Part>, SplitError>`;
  `pub struct SplitError(pub String)` with `Display`;
  `pub fn parse_spec(text: &str) -> Result<Vec<PartSpec>, SplitError>`, lines
  `name parent|- box X,Y,W,H | colors a,b[,..] [in X,Y,W,H] | rest`.
- [ ] Tests: a figure split into head/body/arm by boxes: every painted cell in exactly one part, in spec order
  (first rule that matches wins); pivot of head = middle of its border with body; an unknown parent and a parent
  that does not touch → `Err` naming both (Review Focus 5); cells no rule matches and no `rest` → `Err` with the
  count; `parse_spec` round trips the three rule forms.
- [ ] Run the crate tests → pass. Commit.

### Task 9: CLI verbs

**Files:** Modify `crates/pixfx-cli/src/pixel.rs`, `crates/pixfx-cli/src/main.rs`, `crates/pixfx-cli/src/poses.rs`;
Test `crates/pixfx-cli/tests/pixel.rs`

**Interfaces:**
- Consumes: every library function above.
- Produces: `pixfx pixel clean|read|draw|view|critique|sheet|cut|parts`, `pixfx diff --anchor feet`,
  `pixfx poses <look> --pose P --states`, `look_names(look) -> Vec<&'static str>` beside `look_palette`.

- [ ] Verbs (each writes into `--out`, default `target/pixel`, and prints what it wrote):
  - `clean <png> [--cut TOL] [--height N] [--palette ...] [--max-colors 32]` → `<name>.clean.png`; prints
    `pitch P, cells WxH, confidence C`.
  - `read <png> [--palette ...] [--rects] [--region X,Y,W,H]` → prints text (named legend with a look palette).
  - `draw <txt>` → `<name>.png`.
  - `view <png> [--region] [--max-side N]` → `<name>.view.png`.
  - `critique <png>... [--frames] [--intended x,y;x,y]` → prints issues and score; exit 1 when any `Error`.
  - `sheet --cell WxH --grid CxR [--scale N] [--pixel-lines] [--directions <png>] [--bones <file>]` →
    `sheet.png` + `sheet.json` (the `Layout`).
  - `cut <drawn.png> <sheet.json>` → `cell-<c>-<r>.png`, each through `recover`.
  - `parts <png> <spec.txt>` → `<part>.png`, `<part>.txt`, and a summary line per part
    `name parent bbox pivot`.
  - `diff --anchor feet`: align both pictures by the bottom painted row and the middle of the painted columns.
  - `poses --pose P --states`: one row per state (out false/true × face ±1 × flash/ice/shock/own) at three
    moments of the pose.
- [ ] Tests (`tests/pixel.rs`, through `pixfx_cli::pixel` functions, no process spawn): parse each verb's
  arguments; `clean` on the `hero-style` fixture gives 34x60 ±1; `read` then `draw` round trips a sprite; a 4000
  colour picture `clean` without palette reduces to ≤ 32 (Review Focus 2); `poses --states` sheet has one cell per
  state, each painted.
- [ ] Run `cargo test -p pixfx-cli` → pass. Commit.

### Task 10: Scripts, skill, docs

**Files:** Create `crates/pixfx-pixel/examples/split_by_colour.rs` (a 15-line script: load a PNG, recover, print
components); `skills/pixfx-pixel/SKILL.md`; symlink `game-factory/.claude/skills/pixfx-pixel` → it. Modify
`crates/pixfx-pixel/AGENTS.md`, `crates/pixfx-cli/AGENTS.md`, `README.md`, `CHANGELOG.md`; `crates/pixfx-pixel/
Cargo.toml` (`[dev-dependencies] pixfx-export`).

- [ ] Skill: when to use it; the loop (style reference = the game's sprites, one subject per call on `draft` via
  `openrouter-image --ref`; `clean --height --palette`; `read`; write painter code; `poses --states` / paint;
  `diff --anchor feet`; `critique`); when to write a script; the prompting rules from the study verbatim.
- [ ] `cargo run -p pixfx-pixel --example split_by_colour -- crates/pixfx-pixel/tests/fixtures/hero-style.png`
  prints components.
- [ ] Whole gate once: `cargo test --workspace`, `cargo clippy --workspace --all-targets`, the determinism goldens
  (`cargo test --release -p pixfx-effects --test determinism -- --ignored`). Commit.
