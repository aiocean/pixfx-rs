# pixfx-pixel: a pixel toolkit for agents

Status: draft for approval, 2026-11-14. Study behind it: `docs/research/2026-11-14-pixel-tool-study.md`.

## Intent

Agents make the art of every game-factory game: effects, sprites, poses. They write pixel art as painter code in
Rust (`pixfx-fighter` poses, `pixfx-effects` effects), and the engine paints it, so it recolours, flashes, freezes
and stays deterministic. Today an agent has no good way to work from a picture: an image model's drawing, an
artist's, or a customer's own sprite. It guesses pixels from a blurry view.

`pixfx-pixel` is that way. It is something like Sprite Fusion's generator and animator with no UI and no model of
its own: a Rust library, a thin `pixfx pixel` CLI for the loop that repeats, room for small Rust scripts for
everything else, and a skill file that tells an agent the loop. With it an agent can:

1. lay out a sheet for an image model (a blank grid, a pose guide, directions, frames);
2. have the image drawn through `tools/openrouter-image`;
3. cut the sheet back into cells and recover each cell's true pixel grid and colours;
4. read the result as exact text, look at it at a sensible zoom, and check it with a critique;
5. write painter code from it, and check what the engine paints against what was meant;
6. split a sprite (its own or a customer's) into parts, the ground for effects that act on the sprite itself:
   a head cut off, limbs flying, the body blown apart.

Success: an agent makes more effects and poses, faster, that look hand-made, without a person drawing pixels.

## Goals

- Every step that does not need a model is in the library, pure and deterministic: the same picture and options
  give the same bytes on every run and platform.
- A picture an image model drew comes back as a clean grid at its true size without a hand-typed pixel size, with a
  confidence the agent can act on (re-roll or not).
- What the agent reads round-trips: text from `read` goes back through `draw` unchanged.
- Every critique issue names its cells and a fix.
- Pictures shown to the agent are sized to what they hold, not to the screen.

## Non-goals

- An MCP server or a UI. Agents use the CLI, the library and scripts.
- Image generation inside pixfx-rs. Models, keys and spending stay in `tools/openrouter-image`, which already holds
  tiers (draft $0.006, standard $0.053 a 1K image, `core.ts:9-14`) and a spend ledger.
- Generated tables the engine paints from. The agent writes painter code; nothing in a game depends on this crate.
- Runtime effects that take a sprite apart. Those are engine work (`pixfx-effects`, `pixfx-draw`) that build on the
  parts this crate finds; they get their own design.
- 3D, audio, tilemap editing.

## Anti-goals (a failure even if it works)

- A pipeline that gives different pixels on two runs, or depends on hash order or an RNG.
- A CLI that grows a flag for every one-off need. New logic goes into a script first, into the library when it
  proves useful twice.
- Output the agent cannot act on: an image too large for its token budget, a critique with no cells, a confidence
  of "maybe".

## Constraints

- Rust, in the pixfx-rs workspace. Workspace lints: no `unwrap`, `expect`, `panic`, no `unsafe`. Float maths through
  `libm` (`clippy.toml` denies `f64::floor` and the rest).
- Library: pure, no I/O, no clock, no randomness. Depends on `pixfx-draw` (`Rgba`, `RgbaImage`) and `libm` only.
  PNG reading and writing stay in `pixfx-export` (`decode_png_rgba`, `write_png_rgba`, `sheet.rs:106,156`).
- CLI: argument parsing, files, printing (`crates/pixfx-cli/AGENTS.md`).
- Nothing in the engine depends on `pixfx-pixel` (root `AGENTS.md`, "Where a new piece of code goes").
- Image models reach us only by reference images and a prompt (no ControlNet), through `openrouter-image --ref`.

## Approaches considered

| | Approach | Trade-off | Picked |
|---|---|---|---|
| A | Rust library for every deterministic step; thin CLI for the repeating loop; scripts for one-off logic; image generation through the existing `openrouter-image` | Every step testable and repeatable; no keys or spending in the engine repo; the agent uses two tools, tied by a skill file | **Yes** |
| B | One CLI that also calls the image API | One command; but keys, network and spending in the engine repo, a second copy of `openrouter-image`, and a step that cannot be repeated inside the test suite | No |
| C | An MCP server with a workspace, like agent-art | The user wants CLI only; saved state and undo to build and keep running | No |
| — | CLI only, no scripts | Every new need becomes a flag | No |
| — | Scripts only, no CLI | The steps repeated every time (read, view, critique, cut) cost tokens and boilerplate each time and drift in format | No |

The user picked A, library first, on 2026-11-14.

## Design

### Library (`crates/pixfx-pixel`)

Modules, each one job. Existing ones keep their names.

| Module | Holds |
|---|---|
| `grid` (exists) | `Map` (cells of palette indices), `nearest`, `colors`, `shrink`, `rects`, `Map::paint`, `Map::empty` |
| `cut` (exists) | Cutting a picture from its background |
| `diff` (exists) | How far two pictures are apart |
| `oklab` | sRGB to OKLab through a 256-entry linear table and `libm::cbrt`; ΔE |
| `recover` | Finding a picture's pixel grid and collapsing it (below) |
| `reduce` | Cutting a picture's colours to a budget, and snapping to a palette (below) |
| `text` (`show::text` today) | `read`: a map as text; `parse`: text back to a map and palette |
| `view` (`show::zoom` today) | A picture for the agent's eyes, sized to a budget |
| `critique` | Checks with cells and fixes (below) |
| `sheet` | Layouts for an image model, and cutting a drawn sheet back into cells (below) |
| `parts` | Splitting a sprite into parts (below) |

#### Grid recovery (`recover`)

Pixel Snapper cannot find the grid of a clean upscale (study, section 3), so this is written fresh:

1. Alpha is 1-bit at 128. Colours go to OKLab, cached per distinct colour.
2. Edge energy per column and row: forward differences `ΔE(c(x-1,y), c(x,y))` summed in a fixed order, a change
   of alpha counted as a full edge. Run on the picture before colours are reduced.
3. Period per axis by autocorrelation of the edge profile, computed directly (no FFT); sub-pixel by a least-squares
   fit through the harmonic peaks. Candidates: p, p/2, p/3, and the cell count ±1.
4. Grid lines per candidate by dynamic programming over cell widths `p(1±0.35)` with a smoothness cost; the first
   and last cells may be partial.
5. Each candidate is scored by how well collapsing and repainting it rebuilds the picture (OKLab error on cell
   interiors). Lowest error wins; ties to fewer cells. When the caller knows the target size (a sheet cell is
   64x64), candidates are only those that hit it.
6. Collapse: the middle half of each cell votes. The exact colour with a clear majority wins, else the OKLab
   medoid. Alpha by majority.
7. Out: the grid lines, the collapsed picture, and a per-cell confidence (0 to 1) plus a total.

#### Colours (`reduce`)

- `reduce(picture, max_colors, threshold)`: runs after collapse, on the cell colours. Leader merge at ΔE
  `threshold` (0.03), colours ordered by (count desc, RGB key asc); then the closest pair merges until at most
  `max_colors` (default 32); each cluster is shown by its member nearest the count-weighted centre. No k-means.
- `snap(picture, palette, mode)`: `Nearest` by ΔE, or `Lightness`, which stretches the picture's 2nd-98th
  percentile of L onto the palette's and matches on `w·ΔL² + Δa² + Δb²`, so hue still counts.
- A palette is data that follows a sprite (a fighter look's colours, an effect's ramp), passed in, never guessed
  from a theme.

#### Text (`text`)

`read` is today's `text` output (head line, legend `a #rrggbb`, column tens and units, row numbers, `.` empty).
`parse` takes the same text back to a `Map` and palette, so `parse(read(m)) == m`. Characters past the legend:
the format stays one letter a cell; more than 62 colours is refused, with a hint to reduce.

#### View (`view`)

A picture for the agent: integer scale, at most 16 screen pixels per cell, the longer side at most 512 by default;
grid lines from 8x; brighter lines every 8 cells; a checker for empty cells. A picture too big for the budget comes
back as an overview with the note "zoom with --region X,Y,W,H". Options: region, onion skin of the frames before,
a cell grid of N.

#### Critique (`critique`)

Pure checks over a picture (and its frames). Each issue: `{check, severity, cells: [(x,y)], message, fix}`; a score
of 100 minus 30 an error, 12 a warning, 4 a note. An `intended` list of cells silences deliberate ones. First
checks, from agent-art (study, section 2):

- colour budget (√opaque cells × 1.2, between 8 and 32), near-duplicate colours (ΔE under a threshold),
  semi-transparent cells;
- orphan cells (no same-colour neighbour, all neighbours far), noise (cells unlike 3+ neighbours);
- weak outline (under half the edge cells dark), jaggies (L-shaped stair steps);
- pillow shading (lightness follows distance from the edge more than a light direction);
- frames: drift in volume, ground line and palette; empty or repeated frames;
- grid confidence from `recover` under a threshold.

#### Sheets (`sheet`)

A sheet is a layout of cells: size per cell, columns, rows, gap, and per cell what to draw in it.

- `blank(layout)`: an empty grid with cell borders, for an image model to draw into.
- `guide(layout, cell, guide)`: draws a guide into a cell, in colours the drawing will not use:
  - a pivot dot and a ground line;
  - a pose skeleton: named joints and bones (`head, neck, hips, shoulder_l, elbow_l, hand_l, ...`) as lines;
  - an existing sprite, for directions (the centre of a 3x3) or the first frame of a row.
- Stock layouts: `directions` (3x3, the sprite in the centre, N NE E SE S SW W NW around it), `frames(n)` (a row),
  `variants(n)` (a grid of tries to pick from).
- `cut(drawn, layout)`: each cell's picture back, guide colours removed, ready for `recover`.

Whether image models follow a drawn grid and skeleton is not known. The first task of the plan measures it (the
spike, below). My calls on it:

- **Two kinds of blank.** `blank` borders only each sprite cell; `blank --pixels` also draws a faint line round
  every pixel. The spike tries both. Guide and border colours are picked outside the palette asked for, so `cut`
  can remove them before `recover` sees the cell.
- **Pass line:** a sheet passes when at least 7 of 9 cells keep their drawing inside the cell border, and
  `recover` finds each cell's pixel size within ±10% of the size asked for; a skeleton passes when the drawn
  limbs follow at least 4 of its 5 bones (judged by the agent on `view`, recorded in the spike notes).
- **If sheets fail:** one subject per call, as Sprite Fusion does. The sheet (with its guide) goes in as a
  reference image, the model draws one picture, and `recover` runs on a free picture of unknown size. `sheet`
  stays (it still makes the reference), `cut` is kept only for a model's own grid of variants. The skill teaches
  whichever path passed.

#### Parts (`parts`)

Splitting a sprite into parts an effect can move on its own:

- `components(map)`: connected regions (8-connected), each with its box, cell count and colours.
- `split(map, spec)`: parts from a spec the agent writes after reading the sprite: each part a name and a rule
  (a box, a set of colours inside a box, a polygon, or "the rest"); every painted cell in exactly one part.
- For each part: its cells, box, a suggested pivot (where it touches its parent, the middle of the shared border),
  and its parent.
- The labels are the agent's call; the library finds regions and checks the split covers every cell once.
- **Splitting happens at build time, by an agent** (the user's decision, 2026-11-14). A customer's sprite goes
  through an agent once: it splits the sprite with `parts`, then writes painter code per part, as the fighter's
  poses are written, so the engine needs nothing new to hold parts. The spec names each part's parent (the agent
  knows a hand hangs from an arm); the library checks the parent touches the part and puts the pivot in the middle
  of their shared border. `pixfx pixel parts` writes each part's PNG, its text, and a summary (name, parent, box,
  pivot) the agent reads to write code. There is no runtime split and no part data format in the engine.

### CLI (`pixfx pixel <verb>`)

Thin: parse arguments, read and write PNG and text, print. One verb per step of the loop:

| Verb | Does |
|---|---|
| `clean <picture>` | `cut` (optional), `recover` (to `--size WxH` if known), `reduce` or `--palette`; writes the clean PNG and prints the confidence |
| `read <picture>` | Prints the text; `--rects` adds rects; `--region` |
| `draw <text>` | Text back to a PNG |
| `view <picture>` | The zoomed PNG within budget |
| `critique <picture...>` | Prints the issues; `--frames` for a row of frames |
| `sheet <layout>` | A blank or guided sheet PNG plus its layout JSON |
| `cut <drawn> <layout.json>` | The cells as PNGs, each through `clean` |
| `parts <picture> <spec>` | The parts as PNGs and their boxes and pivots |
| `diff` (exists) | Two pictures compared |

Today's `pixfx pixel <picture>` becomes `read` plus `view`.

### Scripts

`crates/pixfx-pixel/examples/` holds scripts an agent copies and changes: `cargo run -p pixfx-pixel --example
<name> -- args`. `pixfx-export` is a dev-dependency there for PNG. A script that proves useful twice moves into the
library and, if it repeats, the CLI.

### Skill

A new skill, `pixfx-pixel` (the `pixelfx` skill is about the JS reference library and stays as it is). Its source
lives in this repo, `skills/pixfx-pixel/SKILL.md`, versioned and reviewed with the crate, and
`game-factory/.claude/skills/pixfx-pixel` is a symlink to it, because `game-factory` is not a git repository (my
call). It holds the loop (sheet → `openrouter-image` → cut → clean → read →
write painter code → paint → diff and critique), when to write a script, and Sprite Fusion's prompting rules
(study, section 1) adapted: one subject, no sizes in the prompt, pose before motion, the palette passed as data.

## Testing

- `recover`: synthetic pictures with a known answer, the true grid recovered exactly:
  - nearest-neighbour upscales x2 to x12 of random sprites (where Pixel Snapper fails);
  - bilinear upscales;
  - fractional scales (7.5);
  - an offset first cell;
  - added noise and JPEG-like blur.

  Then real model pictures as acceptance tests, not smoke tests (my call): one pose cropped from the stored
  `target/art/poses/frost-gemini-r1/image-01.png` (1584x672, `hero` model) and one cell from each spike model,
  committed small under `crates/pixfx-pixel/tests/fixtures/`, each with its true grid (pixels per cell, size in
  cells) counted by hand on `view` and written next to it. The test checks the recovered grid against that
  answer, not a confidence threshold.
- `reduce`: a gradient keeps its order; the same input in a different pixel order gives the same palette.
- `text`: `parse(read(m)) == m` for random maps.
- `critique`: one fixture per check, its defect found at the right cells, and a clean sprite with no errors.
- `sheet` / `cut`: a sheet laid out, filled cell by cell with known pictures, cut back to the same pictures.
- `parts`: a known sprite split by a spec; every cell in one part; pivots where parts touch.
- Determinism: every test result checked twice in one run, and the recover fixtures' outputs pinned by hash.
- The image-model spike: the first plan task sends a guided sheet through `openrouter-image`, on both `draft`
  (`openai/gpt-image-2.5-sunburst`, $0.006) and `hero` (`google/gemini-3-pro-image`, about $0.14), each with and
  without per-pixel lines: 3 calls each, under $1 in all. It records which model follows cells and skeletons and
  draws the cleanest pixels; that model is the skill's default. (My call: the spike decides the model.)

## Left to the builder

- Exact thresholds and defaults (ΔE values, confidence cut-offs, critique limits), tuned on the fixtures.
- The layout JSON's field names and the part spec's text form.
- Module boundaries inside `recover` and `critique`.
- The order of the plan's tasks, after the spike.
