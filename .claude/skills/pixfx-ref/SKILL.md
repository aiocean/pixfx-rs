---
name: pixfx-ref
description: Use when drawing an effect's reference sheets with an image model and reading them back - writing or fixing a spec (parts, forms, pivots, layers, moments), the template, the OpenRouter call, extracting cells into colour and light layers, the whole-animation strip, review pages, or comparing an effect's frames with its sheet; also when a generated sheet looks wrong (re-laid page, leaked pivot marks, frame lines, drift between moments). A new effect from an idea starts at pixfx-new-effect.
---

# pixfx-ref

Effect references from an image model, in `tools/pixfx-rs`. The library is `pixfx_pixel::reference` (pure: no files, no
network); a script per step in `crates/pixfx-pixel/examples/` does the I/O. The sheet is a **reference the agent
reads** for procedural drawing. For sprite-based and hybrid effects, follow the asset workflow in
`pixfx-effect`.

## What works, and what does not

Learned making the xianxia shields (2026-11-14 to 15; the shields have since left the engine as effects that read no
sprite, and their specs stay as worked examples):

- **The model designs, the agent draws.** The model is good at the idea, the silhouette, the moments and the
  palette. It does not place pixels: copying its finished illustration into painter code came out muddy twice
  (golden bell). What worked (lotus) was asking it for the effect **taken apart** (a PARTS row: the main object
  alone, one of its pieces alone, the small effects, the light) on a **pixel grid** at the effect's resolution, then
  writing one painter function per part, building the moments from them, and comparing frame by frame.
- **Design for code, but copy the part's shading, not a habit.** Write each part as a function (a lens, a ring, a
  scalloped ellipse), and take its shading from its map before you write a line. The first lotus petal had a dark
  outline, bands across its width and a cream tip, and looked stiff beside the model's. The model's petal is
  shaded the other way:
  - a mid pink edge pixel, a white rim inside it and 2 to 3 pixels of glow outside;
  - white at the base and in a vein down the middle, pinker to the sides and deepest at the tip;
  - low contrast inside (neighbouring tones about 15 apart);
  - tones mixed pixel by pixel where they meet (an ordered Bayer dither), not hard bands.

  Read off the map which way each tone runs (along the part, across it, toward its edge) and what colour the edge
  pixel is. Where pieces overlap, a deep edge line keeps them apart: with a light edge they merge into one blob.
  A part that is a pile of detail is not a part: ask for a simpler one.
- **Opaque parts, light as light.** Translucent petals stacked over each other wash out to near white; draw the
  solid parts opaque (`blend_px` at 1.0) and put the glow in the light layer (`glow`, `add`).
- **In-betweens and entropy fills do not make frames.** `tween` morphs drawn frames for the player (motion preview);
  automatic hole filling failed. Read the drawn frames; draw the frames between yourself in code.
- **Ask for the parts in every shape they take, then one sheet per moment.** One petal on the parts sheet was not
  enough: the effect needs the petal closed, curled, cupped, open, tumbling and turned over. `forms` gives each such
  part a row of its shapes, `pivot` makes the model mark where it turns, and `layers` draws each moment as a SOLID,
  FX and LIGHT row, each moment its own sheet built from the drawn parts sheet. That kept all eleven shields'
  moments in one style and at the model's full resolution (about $0.07 a sheet, 5 sheets a shield).
- **Watch the whole animation, not the cells.** `whole` plays every moment's whole frames in order as `whole.gif`;
  reading those strips is how the leaks, the popping INIT frame and the drift below were found.
- **The model cannot keep a count or a motion across sheets; code can.** Told "three rings" in the spec, a rule
  and the prompt, it drew four rings in every moment after INIT (3 tries). Told "each rune lighting as it is
  written" and "the rings turn against each other", it drew whole rings popping in and rings standing still. Take
  the look from the sheets (a ring, a rune, the colours, how the glow is made) and write the count, the writing
  order and the spin in code: the restriction seal was written rune by rune, each ring spun exactly.
- **Match the look by reading how the model makes it, not by turning numbers up.** Restriction looked thin, then
  blurred, beside the model's until its colour and light layers were read: the light layer was almost empty and
  every halo was opaque pixels in steps (white, pale, mid, dark, a dither) round each stroke. Painting the glow that
  way (`pixfx_draw::halo`) is what made it look as good. Brighter `glow` calls only made it blurrier.

## The steps

Run from `tools/pixfx-rs`. Each script's first lines say what it takes and writes.

A spec with `layers` (the way to work: `shield/lotus.json`) is drawn in sheets, so each keeps the model's resolution:
first the **parts sheet** (`--parts-sheet`: the parts and the forms of each), then a **sheet per moment**
(`--moment HIT --parts <the drawn parts sheet>`: that moment's layer rows, built from the parts sheet sent along as
the third image). The template, generate and extract steps below run once per sheet with the same flag
(`--parts-sheet` or `--moment NAME`), each in its own folder (`target/ref/<slug>/parts`, `.../hit`). A spec without
layers is one sheet, as in the table.

| Step | Do | Writes |
|---|---|---|
| Spec | copy `crates/pixfx-pixel/examples/specs/TEMPLATE.json`; `shield/lotus.json` is the worked example | |
| Template | `cargo run -q -p pixfx-pixel --example template -- <my>.json` | `target/ref/<slug>/`: `template.png`, `sheet.json`, `prompt.txt`, `style.png` |
| Draw | `cargo run -q -p pixfx-pixel --example generate -- target/ref/<slug>` | `draw-<n>/image-01.png`, `job.json`; a line in `target/ref/spend.jsonl` |
| Extract | `cargo run -q -p pixfx-pixel --example extract -- <my>.json target/ref/<slug>/draw-<n>/image-01.png` | `target/ref/<slug>/review/`: per cell original, alpha, colour, light, rebuilt PNGs; diffs; `matrix.json`; `index.html` |
| Read the parts | `review/parts-<name>.{rebuilt,color,light}.png` and their maps in `matrix.json` | |
| Write | the effect in `pixfx-effects`, a function per part (skill `pixfx-effect`) | |
| Compare | `cargo run -q -p pixfx-pixel --example compare -- <my>.json target/ref/<slug>/review <effect-id> --times "init=0.15,0.35,...;hit=...;break=..."` | `review/compare.png`, `compare.json`; prints each row's step changes, sheet and effect |
| Fix, again | change the code, `compare` again, until the board and the steps agree | |
| Gallery | `cargo run -q -p pixfx-pixel --example gallery` | `target/ref/index.html`: every sheet, linking to its review page |

Many sheets at once (every shield): build the scripts once in release (`cargo build --release -p pixfx-pixel
--examples`, about a minute) and run `target/release/examples/<script>`. A debug `extract` spends 30 to 60 s
compressing its PNGs, a release one 3 s; 40 of them in debug, one after another, look like a hang. Run `generate`
for many sheets in parallel (each is a network wait) and long batches in the background.

Drawing spends money (about $0.07 a draft sheet) and needs `OPENROUTER_API_KEY` in the environment (the
game-factory key is in `.env`: load it with `set -a; . .env; set +a`, never print it) and `curl` on
the path. Run `generate ... --dry-run` first to read the request. Every run draws into a new `draw-<n>`; nothing is
overwritten. Read `prompt.txt` before you spend.

## The spec

A spec holds:

- `id`, `name`; `look`: what it is and looks like, a name then a sentence or two; `subject`: the thing that keeps its
  size and place in every frame (`bud`, `bell`).
- `grid: true`, `cell: 64`, `scale: 6`, `gap: 10`: every cell is a 64 x 64 grid of art pixels, drawn 6 sheet pixels
  each, and the model is asked to fill whole squares. A cell always covers 96 engine pixels round the duel's
  fighter (`pixfx_draw::RULER`, the guide figure, feet at 88), so at 64 squares **one art pixel is 1.5 engine
  pixels**: the fighter is 27 squares, its head 5, its eye still 1. The prompt says those sizes, from
  `compare::art_ruler(cell)`, so the model draws every object at the character's scale. At `cell: 96` a square is an
  engine pixel and the sizes are the ruler's own (40, 7, 1); it needs about 1.5 times the sheet pixels a cell, so
  keep `scale` and the rows a sheet holds small enough for the model's ~1536 x 1152 answer (see the Traps of
  `crates/pixfx-pixel/src/reference/AGENTS.md`). When writing painter code from a 64-square sheet, size each object from the ruler
  (a part and a ratio), not by copying the sheet's squares as engine pixels.
- `parts`: the PARTS row, first on the sheet, a cell each: `name` (upper case) and `text` (what the cell holds, one
  sentence). Ask for:
  - `MAIN`: the main object alone, whole, fully formed, "with no glow, no specks";
  - one or two pieces it is built from, alone and upright (`PETAL`, `PLATE`, `SWORD`, `BAND`);
  - anything else that has its own shape (`PAD`, `MIST`, `SMOKE`, `TRAIL`);
  - `FX`: the small effects "spread out so none overlap";
  - `LIGHT`: "only the glow of ..., with no solid shape drawn".

  A part that turns, bends or curls gets `forms`: every shape it takes in the effect, a cell each in its own row
  (`PETAL 1/6`...), so no form has to be guessed. The lotus's petal: upright and closed; leaning out, tip curled in;
  broad and cupped; wide open and flattened; tumbling edge-on; turned over. One upright petal was not enough: the
  bud, the opening and the drift each needed a form the code then made up. Draw the left-leaning form; mirror it in
  code. `pivot` ("its base") has the model mark where the part turns with one green square in every form;
  `extract` reads it into `matrix.json` (`pivot`, in the cell's pixels) and paints it over.
- `layers`: each moment row is drawn once per layer, frame for frame (`SOLID`: the shapes, opaque; `FX`: specks,
  pollen, flecks; `LIGHT`: only the glow), so each can be read and compared alone. `extract` puts them back together as the moment's whole frames (`hit-1`...: the opaque layers in order, then the layer marked `"light": true` added where no shape is); the player plays those first and `compare` reads them.
- `rows`: the moments, top to bottom: `name` (`INIT`, `IDLE`, `HIT`, `BREAK`, any you need), `caption` after the first
  cell's label, `text` (one sentence, no full stop), and optionally `frames` (its own cell count) and `notes` (what
  each frame must show, `""` for nothing).
- `frames` (cells a row), `rules` (lines for the whole effect), `figure` (the stick figure, in a 96-pixel cell's
  coordinates, scaled to `cell`; the default is the duel's fighter at the ruler, so leave it unless the effect plays
  on another character).

A spec without `grid` and `parts` keeps the first prompt the shields were drawn from (the old 96-pixel sheets).

## What the model does, and the words that stop it

| Left alone, the model... | So |
|---|---|
| draws a character over the stick figure | the prompt names the figure as a size guide only and says not to draw it |
| draws a sword, ring or talisman at a size of its own, out of scale with the character | on a grid the prompt gives the character's height, head, hand and eye in squares (`compare::art_ruler`) and asks for everything at that scale, nothing under one square |
| paints a dark backdrop that blends into every glow | the request asks for a transparent background (`gpt-image` honours it; Gemini, the `hero` tier, does not) |
| blends colours across pixels and draws its own pixel size | the grid: "fill whole squares with one flat colour each ... a small palette of about 8 colours plus 4 glow colours" |
| gives you a finished picture with nothing to take apart | the PARTS row: each part "alone, whole, at the same size as it appears in the other rows" |
| draws the first frame of a build-up already half formed | a `note` per frame with a pixel budget: "only the pad and a few faint specks" |
| changes the shape between runs | say the shape in `look` and in the IDLE `text` |
| keeps the cyan cell frames, boxes the labels, squashes rows by up to a fifth | `extract` takes the cyan out near cell edges, ignores frames and boxes, and counts each cell's squares up from its bottom |
| re-lays a page whose rows differ in length (5 parts stretched across the width of 6 forms), cells not square, its own pixel size | `extract` finds the cells from the cyan frames it kept, and on cells that are not square finds its pixel size and cuts each cell on it |
| leaks a few specks into SOLID, sparkles into LIGHT | read the layer you need; small leaks do not matter for painter code |
| on a moment sheet, copies the parts sheet's green pivot marks into frames | `extract` paints a green mark over in every cell and keeps pivots only on part cells |
| draws a cyan frame line inside a cell (golden-bell HIT frame 4, every layer) | `extract` clears a column or row of a cell that is frame cyan along 3/5 of it |
| draws an upright cyan piece (a sword) that looks like a frame side | `framed_cells` pairs the sides by the cell width that gives exactly the row's cell count; a pitch that fits below 0.15 falls back to the width over the cell count |
| uses a cracked form in IDLE (frost-ward, turtle-shell) or draws the main object at another size (thunder) | the moment prompt says the main object keeps MAIN's shape, number of pieces and size unless its row says it changes, and damaged forms belong only to rows where it is struck or ends; check every strip against MAIN and redraw the moment that drifted (one redraw fixed all three) |
| fills a tall cell with more of a repeated piece than MAIN has (restriction: 3 rings in MAIN and INIT, 4 packed rings in IDLE, HIT and BREAK, through the rule above and a spec rule "exactly three, never four") | stop redrawing after one try: the count is set in painter code from MAIN; read the drift-free things (one ring's look, its turn, the flare) from the moments |

## Reading the review page

At the top, a player runs the moments in order (the parts are not frames and stay out of it): every row in turn
or one row looped, the layer to show (rebuilt, original, colour, light), in-betweens, fps and size; space plays and
pauses, ← → step a frame. In-betweens (`pixfx_pixel::reference::tween`, 7 by default, `extract --between N`) move the drawn
pixels so the motion plays smooth; they never redraw: judge the drawing with in-betweens at 0.

Below it, each row is one cell: **original** (over black), **the model's pixels** (the busiest part blown up with
the cuts in cyan: each square one flat colour), **alpha**, **colour** (opaque pixels, exactly as drawn), **light**
(what the see-through pixels add over black), **rebuilt** (colour, else light). Rebuilt must match original; the
error is the mean per channel (0-255), about 1-5. The diffs show what moves between frames of a moment.

On a grid sheet every cell is exactly `cell` pixels a side and is never moved. On an old sheet without a grid the
model's own pixels are found (`grid` strength under each frame, doubtful below 0.15) and frames are moved to line
up (`moved`).

`matrix.json` holds every cell as a pixel matrix: a colour map and a light map as text rows (a palette letter per
pixel, `.` elsewhere) and the exact colours; read the parts' maps to write their functions. Swatches on the page are
colour pickers that recolour every frame, to try a palette before writing it.

## Writing the effect from the parts

1. Measure each part on its map: its size in art pixels (times `96 / cell` for engine pixels: 1.5 at 64), where it
   is widest, its tones from outline to highlight. Take the palette from the exact values. Then check its size
   against the ruler (skill `pixfx`, "One scale: the ruler"): name the body part it is measured against and the
   ratio, and when the sheet's size and the ruler's disagree, the ruler wins (`Ruler::size(part, ratio)`); write
   the part and the ratio in a comment by the part's function.
2. Write a function per part with the part's own parameters (a petal takes a lean, a curl, a length and a width), and
   draw every use of it through that function: the bud's petals, the drifting petals and the flecks of a blow are
   the same petal at other sizes.
3. Build the moments from the parts and a timeline (`smooth(t0, t1, t)` per piece, staggered), the back pieces on
   `Layer::Back`, the front on `Layer::Front`.
4. Render (`cargo run -p pixfx-export --example gif -- <id> --scale 4`) and run `compare` with `--times` set to when
   your effect shows each drawn frame.
5. Put one held frame of yours beside the part or whole frame it copies, both blown up by whole numbers over black,
   and name the differences you see before changing code: band thickness, how far halos reach, whether a stroke's
   shape survives its glow, how dark the gaps are, how bright the far side is. Change those, render, look again.
   Restriction took five rounds: thin and dim; brighter but blurred; solid tubes (halos too long); beads (the runes
   drowned in their own halo); then right, with each stroke drawn in its own colour and its heat only setting how
   far its halo reaches.
6. A glow the sheet paints as steps of colour: stamp each stroke into a `pixfx_draw::halo::Halo` with a heat and a
   kind, `spread(fall)`, then paint each pixel with the ramp colour of its heat's step, dithering the last step
   (`pixfx_draw::halo`). Keep additive light as a faint bloom on top. A grid per depth (behind and in
   front of the character), `Halo::none()` for the depth not being drawn.
7. A blow the effect takes: let the game say where it lands with number options in local pixels from the feet
   (`hitX`, `hitY`, defaulting to where the sheet drew it), and derive everything from that point (which piece is
   struck, where its flare starts, which way it tilts, where sparks fly, where `impact` fires its cue). Try them with
   `gif ... --num hitX=-20 --num hitY=-48`.
8. On the site, give a new effect with a blow its role and blow time in `site/assets/effect-roles.js` (`role`, and
   `impact` or `blows` at its cue times; `directional: true` flips it to face the foe), and its own options sliders
   in `site/assets/effect.js` `OWN`.

## Reading compare

The board is a line of sheet cells over a line of effect frames per moment, at the same scale, so silhouette, size,
tones and placement can be put side by side. The numbers are each step's change (`pixfx_pixel::reference::entropy`): the
conditional entropy of each frame's pixels given the last one's, 0 for nothing new, 1 for unrelated. Read them as
timing, not as a score:

- the effect's steps bunch up where the sheet's are even (sheet 0.42 0.54 0.48 0.32, effect 0.21 0.38 0.22 0.11):
  it finishes early; spread the pieces' `smooth` windows;
- a step well under the sheet's: too little happens there (lotus HIT at half the sheet's: the burst is too small);
- a step well over: too much at once.

Shapes and tones you judge on the board; timing on the numbers. The sheet's own steps run higher than a clean
effect's (its glow and specks change every frame), so compare the shape of the two rows, not their size.

## Not this skill

- A character, a pose, a sprite to redraw or split: `pixfx-pixel`, `pixfx-pose`.
- Porting an existing effect from pixelfx or Sprite Fusion: `pixfx-effect` (no image model).
