# Xianxia shields

## Intent

Games in game-factory are cultivation (tu tiên) games: the site's duel already has qi, rage, schools and barrier
spells. pixfx has one shield (`arcane/hex-shield`), and the duel's three shield spells borrow effects that are not
shields (`bell` and `mirror` use `arcane/rune-circle`, `ward` uses `holy/feather-burst`). This round adds the
shields the genre is known for (Renegade Immortal and xianxia at large), each drawn first by an image model as a
look reference, then written as a pixfx effect by hand.

## Goals

- Eleven shield effects, one per archetype, each reading at a glance as that archetype at game size.
- Each looks hand-made in pixfx's style: whole pixels, ordered dither, colour plus additive light, the same weight
  as the existing effects.
- Each shows the life a game needs from a shield: it forms, holds, takes a blow, and ends.

## Non-goals

- Swapping the duel's spells over to the new shields (`site/` belongs to another session; a follow-up).
- Runtime calls into a playing effect (a game calling `hit()` at its own moment): pixfx has no such channel yet.
- Characters, names or art from Renegade Immortal or any other work: archetypes of the genre only.

## Anti-goals

- Painting from the generated image. The image is a reference the agent reads (the review pages of `pixfx-ref`);
  the painter code is written by hand (skill `pixfx-effect`).
- Eleven variations of one dome in different colours. Each archetype has its own shape and motion.

## The shields

Ids under a new category `shield`, feature `shield-<name>`.

| Id | Archetype | Look | Blow | End |
|---|---|---|---|---|
| `shield/golden-bell` | Golden Bell (金钟罩) | a translucent golden bell drops over the body, engraved bands | it rings: a gong ripple and a ring of light | fades from the rim up |
| `shield/bagua-mirror` | bagua mirror (八卦镜) | an octagonal mirror of trigram bars turns in front of the body | the blow flashes back off it as a beam | spins down and folds away |
| `shield/frost-ward` | ice ward | faceted ice shell grows from the ground up | cracks spread from the hit | shatters into falling shards |
| `shield/restriction` | restrictions (禁制) | stacked rings of runes round the body, turning against each other | runes flare along the struck layer | rings unravel layer by layer |
| `shield/sword-ring` | flying-sword formation (剑阵) | swords fan out and orbit point-out | swords swing to the hit and spark | swords fly off outward |
| `shield/talisman-ring` | talismans (符箓) | yellow paper talismans with red glyphs orbit, joined by thin light | the struck talisman burns up | the rest burn to ash |
| `shield/lotus` | lotus throne (莲台) | petals rise and close into a bud round the body | petals shiver, pollen of light | petals open and drift away |
| `shield/soul-banner` | soul banner (魂幡) | dark spirits swirl in a vortex round the body | spirits shriek outward | spirits scatter and fade |
| `shield/turtle-shell` | black tortoise shell (玄武) | jade-dark hexagon plates slam down into a shell | a plate cracks, dust | plates crumble to stones |
| `shield/thunder` | thunder shield | a sphere of crackling arcs, bolts crawling over it | a bolt arcs out to the hit | collapses in a last flash |
| `shield/blood` | blood shield | a dark red film with flowing veins | it bulges and splatters | drains down into the ground |

## Shared shape

- Canvas about 96 x 96 with the origin at the character's feet, room for a figure 40 px tall (the duel's fighters).
- One-shot, 30 fps, about 2.4 s: forms over about 0.35 s, holds, takes one blow, ends over about 0.4 s.
- Options every shield takes: `hit` (seconds into the effect when the blow lands, 0 for none) and `feel` (the light
  shake cue on the blow, `pixfx_core::cue::impact`); `palette` recolours it. Each shield adds only what it needs.
- Behind and in front: the part of a shield behind the character draws on `Layer::Back`.
- What three or more shields share (the timeline, the blow's timing, dither helpers) goes in `src/shield/lib.rs`;
  anything all effects could use goes down to `pixfx-core` or `pixfx-draw` as usual.

## Checking

Our own effects have no source to match, so the parity rule changes: ports pass parity against their source;
effects of our own are pinned by golden frame hashes (native and wasm) and reviewed by eye against their
reference. `AGENTS.md`, `crates/pixfx-effects/AGENTS.md` and `README.md` say so. Each effect also passes the existing
suites (features, option bounds, determinism).

## References

One sheet per shield from `crates/pixfx-ref` on the `draft` tier, with a frame of `arcane/hex-shield` as the style
reference (it sets the pixel size). The specs are `crates/pixfx-ref/examples/specs/shield/*.json`; the skill
`.claude/skills/pixfx-ref` runs them. Hero tier only if the user asks.

## Approaches

- **One category, a shared module, one file per shield (picked).** Each shield is its own effect and cargo feature;
  `shield/lib.rs` holds what they share. Games build only the shields they use.
- One configurable shield effect with a `kind` option: one feature for all, but each archetype's shape and motion
  are different enough that the effect becomes eleven effects behind a switch.
- Spread over the existing categories (`holy/golden-bell`, `ice/frost-ward`): fits element folders, but a game
  looking for a shield has to search eleven places.

## Left to the builder

Exact sizes, colours, timings, how each shape is drawn, and the shared module's API, within the above.

## Progress (2026-11-14)

Done, step by step with the user reviewing each:

1. A template sheet (4 rows INIT, IDLE, HIT, BREAK, 3 frames, a stick figure a cell) and a prompt per shield; the
   model drew all eleven. First round: it drew a character over the figure.
2. Second round: the prompt names the figure as a guide only and says not to draw it. No character; the user
   approved the look.
3. Cutting the effect's pixels out of a sheet drawn on a dark background failed the user's review: the dark backdrop
   blends into every glow, so the colour layer got a dark outline and shifted hues.
4. Third round: transparent background (`background: transparent`, honoured by gpt-image). The cut is now direct:
   opaque pixels are colour as drawn, see-through pixels are light. The user approved the review pages (original,
   alpha, colour, light, rebuilt per frame, frame diffs) and the gallery.
5. The whole pipeline is Rust in `crates/pixfx-ref` (the first version was Python under `target/`): the eleven
   third-round sheets extract identically with it. Specs gained a frame count per row, a note per frame and rules;
   with notes, golden bell's INIT builds up from a few specks instead of starting half formed.
6. Mastering the pixels, step 1 of 4: grid. The model draws its own pixels (2 to 4 sheet pixels, not square,
   2s and 3s mixed), not the template's 96, and its grid drifts across the sheet. `pixfx_pixel::frames` finds the
   pitch from every frame at once, gives every frame the same pixel count and fits each frame's cuts; all twelve
   sheets get a grid (golden bell 117×132, thunder 115×128, gold-bell-with-notes 76×83). Blowing the layers back up
   through the cuts misses the original by 10 to 27% less than even cells of the same count, and by up to 37% less
   than the template's grid.
7. Step 2: origin. The model draws the subject a few pixels off in some frames, mostly a whole row (lotus HIT
   +5 to +6, turtle IDLE −2, restriction BREAK +3 to +4). `pixfx_pixel::register` matches each frame's edges with
   the median of all frames' edges; brightness was fooled by golden bell's BREAK dissolving from the bottom, edges
   are not. A best match on the search window's edge is not taken (petals flying apart). Red/cyan overlays of the
   moved frames line up after.
8. Step 3: pixel matrix. The user chose exact colour plus a palette index: every model pixel is empty, colour or
   light with the value the model drew, and points at a palette the frames share (32 colour, 16 light, built by
   `pixfx_pixel::reduce`). `matrix.json` holds it as text maps and exact values. The palette misses the exact values
   by 2.6 to 10.8 per channel (colour) and 2.9 to 7.2 (light); bagua's spark goes paler, faint glow bands. Step 4:
   a motion map (pixels that appear, vanish, move) in the review page and player.

Next, when the user says so: step 4 above, then redraw the shields with frame notes where they help, then write the eleven effects in
`pixfx-effects` from the review pages, goldens plus visual review, and update `AGENTS.md`,
`crates/pixfx-effects/AGENTS.md` and `README.md` for effects of our own.
