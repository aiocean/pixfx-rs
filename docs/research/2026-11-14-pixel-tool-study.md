# Study: Sprite Fusion, agent-art, Pixel and Palette Snapper

What these tools do and how they are built, to write our own agent-only pixel tool (`pixfx pixel`, crate
`pixfx-pixel`) from understanding, not by porting. Sources were read on 2026-11-14:

- Sprite Fusion: its public docs and page bundles (scratch copies in `/tmp/sf-re*`, not kept). Model and provider
  are not disclosed anywhere public.
- agent-art: `game-factory/tools/agent-art` at `f2e23cd` (JS, MIT).
- Pixel Snapper: `game-factory/tools/spritefusion-pixel-snapper` at `02f9315` (Rust, MIT). Palette Snapper:
  `Hugo-Dz/spritefusion-palette-snapper` at `ea64782` (Rust, MIT).
- Ideas only: theamusing/perfectPixel, ventz/pixel-perfect, alexkorol/pixel-respecter.

Confidence: **H** read in code or docs, **M** inferred from several facts, **L** a guess.

## 1. Sprite Fusion (a paid web app with an API and MCP)

| Feature | Takes | Gives | How it is built | Conf |
|---|---|---|---|---|
| Generate | prompt, size 16-128 (9 sizes) | 12 sprites (4 from 96 px), streamed one by one | Their own model. Results can be a few pixels off (117 for 112), so it likely draws big and snaps to a grid | H / M |
| Edit | prompt + 1-9 images (first is the sprite) | variants at the sprite's size | Other images are "supporting visual sources" | H |
| Style reference | prompt + 1-20 references | 9 sprites | 4-8 references work best; theme words can override the style | H |
| 8 directions | 1 image, no prompt | N NE E SE S SW W NW | Shown as a 3x3 with an empty centre; how it is generated is not known | H / L |
| Animate | 1 sprite + motion prompt, even frames 2-16, colours 2-256 | one-row sheet, WebP preview, `metadata.json` | The app always sends 8 frames and 24 colours. A frame is the sprite's canvas, so motion past it is cut off. Pose the sprite first (edit), then animate | H |
| Palettes | colours, a saved palette, or a preset | every result in only those colours | The palette travels with the asset as metadata, never re-detected from pixels | H |
| Palette snap / reduce (free) | image + palette (`nearest` / `lightness`) or `max_colors` (32) | PNG, transparency kept | Palette Snapper (section 3) | H |
| Normal maps (free) | sprite or sheet, strength, OpenGL/DirectX | normal map PNG | A dome from the silhouette's distance field plus detail from lightness, then Sobel. Each 8-connected island is handled on its own | H |
| Pixel editor | | | Brush 1-16, eyedropper, canvas size, undo, pixel-perfect lines | H |
| Tilemap editor | tileset | maps for 14 engines | Manual, 3x3 autotile rules; no AI | H |
| VFX library | slug | a "recipe" (timeline, layers, palette) + one TypeScript file | 25 effects, as code an agent can port | H |
| API / MCP | `POST /api/v1/generate` with `operation`; 19 MCP tools | SSE: `started` (credits held), `output` per asset, `completed` | Refund when nothing is saved; never auto-retry a broken stream | H |

Patterns worth learning:

- One entry point per operation, with exact input counts (generate 0, edit 1-9, style 1-20, directions 1,
  animate 1).
- A batch of variations by default; the agent or person picks one. Every request is recorded locally (id,
  operation, prompt, each output's path).
- Pose before motion: make the key pose as its own sprite, then animate from it.
- The palette is data that follows the asset, not something guessed from pixels.
- The deterministic tools (snap, normals) are small Rust programs that run the same as a CLI, in wasm and on the
  server.

Their prompting rules for agents (verbatim):

- "Describe one item or subject without requesting a number of outputs."
- "Ask for the subject or variations, not for a grid, contact sheet, atlas, or spritesheet that packs several
  assets into one image."
- "State the requested change and anything important that should remain unchanged."
- "Describe the motion, energy, and whether it should loop."
- "Avoid noisy phrases such as "transparent background," "crisp pixel art," or "pixel-perfect pixel art.""
- "Do not include pixel dimensions in the prompt."
- "Omit palette by default… don't infer it from a theme, a style, or a wish for consistency."

## 2. agent-art (an MCP server that lets an agent draw pixel art itself)

How it is built:

- **One engine, thin tools.** 29 tools map onto engine ops; the same list runs from a CLI. A canvas is
  `{width, height, frameHeight, layers, palette, meta, animations}`, saved as JSON with PNG layers. Animation frames
  are rows of one canvas. One draw call is one atomic batch, rolled back on any error.
- **Drawing.** Lines, shapes, fills and `grid`: rows of characters through a legend.
  - `shade` gives automatic form shading: distance to the edge → dome → normals → light → ramp, optional Bayer
    dither.
  - `cells` (Voronoi stones and scales), `noise` (shares of pixels exact by rank), `outline`.
  - Lighting and shadows stay banded and dithered so they remain pixel art.
  - `damage`: cracks and holes in colours the region already has, placed from a seed in box-relative space so they
    match across frames.
- **Rig.** Parts are drawn once per view in cells with a red pivot dot. A pose is `{x, y, rot, sx, sy, variant,
  hide, z}` per part. It also has:
  - two-bone IK, foot planting and springs for capes;
  - rotation through Scale2x 8x + 3x3 supersampling + a 5-of-9 majority vote + stray-pixel clean-up, so rotated
    parts gain no new colours;
  - 2D skinning (a lattice mesh, heat-diffusion weights) to bend a sprite instead of rotating it;
  - an animation library that retargets clips by hip height.
- **Seeing and checking its own work** (the part that matters most for us):
  - **Pictures sized to their token cost.** At most about 512 px and 16 screen pixels per art pixel; grid lines
    from 8x; a checkerboard for transparency. A large canvas comes back as an overview with "zoom with x,y,w,h".
  - **`art_read` round-trips.** It returns a character grid plus a legend in exactly the format `grid` draws, so the
    agent reads, edits and writes back with nothing lost.
  - **`art_critique`** scores 100 minus 30 per error, 12 per warning and 4 per info. Every issue carries
    coordinates and a fix written as a tool call. It checks:
    - colour budget (√opaque px × 1.2, 8-32), near-duplicate colours, semi-transparency;
    - orphan pixels, with an `intended` list for deliberate ones;
    - noise, low contrast, desaturation, weak outline, jaggies;
    - tiling seams;
    - pillow shading: brightness follows distance from the edge, not a light direction;
    - frame drift: volume, ground line, palette, empty or duplicate frames.
  - **Rig `consistency`.** Bone lengths, palette and joints across views; `animate` refuses frames that break the
    rig.
  - **Token discipline.** Short tool descriptions, help served by section with token counts, compact replies.

agent-art never splits an imported sprite into parts. Parsing a customer's sprite (for heads and limbs coming off)
is new ground.

## 3. Pixel Snapper and Palette Snapper (grid recovery and colour reduction)

Pixel Snapper (`src/lib.rs`):
1. k-means to 16 colours, seeded `ChaCha8(42)` (721-859).
2. Edge profiles, a grey central difference `|g(x+1)-g(x-1)|` summed per column and row (939-979).
3. Step = the median gap between strict peaks above 0.2·max (981-1023).
4. Square cells, averaged across the axes (1025-1058).
5. A greedy walker that cuts at the strongest edge within ±0.35·step (1133-1184).
6. Stabilising, the most common colour per cell (1345-1398).

**It cannot find the grid of a clean nearest-neighbour upscale** (H, run on a ×7 sprite: 80x80 instead of 20x20).
A hard edge gives the central difference two equal values side by side, and the strict-peak test (`>` both
neighbours, 993-997) finds none, so the fallback `size/64` takes over. Other weaknesses:

- cuts land one column early;
- cells are forced square;
- errors carry forward through the greedy walk, with no global fit;
- no confidence output;
- colours are reduced before grid detection, so anti-aliased tones take palette slots.

Palette Snapper:
- Converts to OKLab with its own cbrt, so results are the same on every platform.
- Sorts colours by count, then leader clustering at ΔE 0.03: leaders never move.
- Then merges the closest pair until at most `max_colors` (32) are left; the more frequent colour wins.
- `lightness` mode stretches the image's 2nd-98th percentile of L onto the palette's range and matches on L only.
- Deterministic. Order-dependent, O(c³) on many colours, ignores hue in `lightness`.

Better ideas from the alternatives:
- Autocorrelation for the period, with harmonic sub-pixel refinement (ventz).
- A dynamic-programming fit of all grid lines at once, with partial first and last cells (ventz).
- Choosing between grid candidates by reconstruction error, which also gives a per-cell confidence map (ventz).
- Only the middle of each cell votes (ventz, pixel-respecter).
- Reduce colours after cells are collapsed, so anti-aliasing never claims a slot (ventz).
- One score for every (size, offset) candidate: contrast between cells / √(variance inside cells) (pixel-respecter).

A design for our own, deterministic with no RNG:

| Stage | Choice |
|---|---|
| Alpha | 1-bit, at alpha 128 |
| Colour | OKLab through `libm::cbrt`, cached per colour |
| Edges | Forward difference `ΔE(c(x-1), c(x))` on the unreduced picture, alpha counted as an edge |
| Period | Autocorrelation per axis in a fixed order, harmonic refinement; try p, p/2, p/3 and the count ±1 |
| Lines | Dynamic programming over widths p(1±0.35) with a smoothness cost; partial first and last cells |
| Choice | Reconstruction error on cell interiors; ties to fewer cells; a confidence map out |
| Collapse | The middle half of each cell votes: the exact colour with a clear majority, else the OKLab medoid |
| Reduce | After collapse: leader merge at ΔE 0.03 ordered by (count, RGB key), then merge to `max_colors`; a real member colour represents each cluster |
| Snap | Nearest by OKLab ΔE; `lightness` stretches L but keeps a hue term |

## 4. What this means for our tool

Our edge is that the agent writes painter code, so effects can act on the sprite itself. A list of picture files
can't do that. What the three tools teach, as candidate features:

1. **Read and clean any picture:** automatic grid recovery and colour reduction (section 3), with a confidence
   map.
2. **Round-trip text:** `read` gives the grid and legend, `draw` takes the same text back.
3. **Critique** with coordinates and a fix per issue (section 2).
4. **Pictures sized to their token cost:** zoom, grid lines, an overview plus zoom for big ones.
5. **Sheets for an image model:** a blank grid, a 3x3 for directions, a row for frames, a pose guide inside cells,
   then cut each cell back. Pose before motion; a batch of variations to pick from.
6. **Parts:** split a sprite into named parts with pivots (head, arms, legs, body), the ground for effects that
   take a sprite apart. Rotate parts without new colours (Scale2x + majority vote).
7. **Palette as data** that follows a sprite; snap to a palette (`nearest`, `lightness`).
8. **Export** a one-row sheet + `metadata.json` (`frame_w`, `frame_h`, `frame_count`, `fps`).
9. Later: `shade`, `damage` in existing colours, normal maps, 2D skinning.
