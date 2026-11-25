---
name: pixfx-new-effect
description: Use when the user asks for a new pixfx effect from an idea, often one sentence ("the hero dissolves into ash when it dies", "an aura that grips the character as it powers up", "the sword leaves a smear as it swings"), wants help designing or storyboarding an effect, or says they do not want to think the design through themselves. The entry point before pixfx-ref and pixfx-effect.
---

# A new pixfx effect, from an idea to the library

The user gives an idea; **you do the design thinking**. Research, draft the storyboard, and model the effect's
geometry and motion before sketching its pixels. Bring the user only the few real forks, as choices with your
recommendation. Pick the reference workflow below, then write the painter with `pixfx-effect`.
Run from `tools/pixfx-rs`.

## 0. Choose the visual source

Decide whether the effect uses procedural drawing, authored sprite frames or both. Use actor pixels for
effects that conform to a body and a trace for effects shaped by movement. Standalone effects can play at a
point. Choose `Paint`, `Sprite` or `Hybrid` for the effect's render kind.

## The questions: propose, never interrogate

Every question goes through `AskUserQuestion`, at most 3 at a time, and each one:

- comes **after** your draft, about a fork the draft could not settle (the user cannot answer what they have not
  seen);
- has 2 to 4 options, **your pick first, marked "(Recommended)"**, each option saying what it changes in the effect;
- shows the alternatives in `preview` when they look different: a few lines of text sketch (what each moment shows);
- is a choice. "What style do you want?" makes the user design; "Gold ash (Recommended) / Grey dust / Ice
  shards" lets them pick.

A fork you can settle from the repo or from convention is not a question: decide it, and say it in one line in the
storyboard ("Category: `feedback`, next to `feedback/power-up`").

## 1. Research (no questions yet)

- `rg -n -i '<idea words>' crates/pixfx-effects/src crates/pixfx-pixel/examples/specs` and `target/ref/*/`: an effect
  or a drawn sheet may already exist. Reuse what exists; say so.
- The closest existing effects (`explosions/body-burst`, `feedback/outline`, `feedback/power-up`,
  `projectiles/trail`): their duration, size, origin, how their moments run, how they read the sprite or the trace.
  Read their files.
- How the game will use it. Start with the roles below and check the closest game's usage when available.

### Choose the reference

- **User-approved GIF or other reference.** Use it directly as the target. Inspect key frames locally and
  record their elapsed times using the GIF's frame delays, including variable delays and the loop seam when
  relevant. Use the observed shapes and motion to fit the model in 2a. Put the original and selected frames on
  the review page, skip reference generation (stops 2 and 3), and compare the Rust render with the supplied
  reference at stop 4. This route requires no AI redraw, spec, template or `pixfx-ref` call.
- **Enough information to draw directly.** When the model and existing art or primitives give the painter a
  clear target, review the wireframe and plots, then go straight to stop 4.
- **New art would resolve a specific gap.** Use `pixfx-ref` when a missing look, part or beat warrants a drawn
  reference and the user wants one. Generate only the material needed to fill the gap, after the user approves
  any paid generation. Keep the supplied reference as the target for everything it already shows.

## 2. Storyboard: draft it whole

Pick the role, then build the moments from its beats. Use key frames from the supplied reference when available.
When designing a sheet for generation, plan 4 to 6 drawn frames per moment. Give each moment a duration, caption
its key frames, and identify what makes it read, such as anticipation, impact, follow-through or settle.

| Role | Moments (beats) | Usual length |
|---|---|---|
| buff on the caster: an aura gripping its outline | GATHER, BURST (cue), AURA (loops), FADE | 1 s plus aura |
| death: the body breaks apart | SWELL (anticipation), BURST (cue), PIECES fly and settle, FADE | 2 to 3 s one-shot |
| mark: the sprite ringed, lit or tinted | one LOOP, following each frame | 1 s loop |
| trail: the way it moved, a smear of its body | follows the trace, shrinks once it stops | a loop while it moves |
| appear / disappear: the sprite materialises or dissolves | GATHER, the sprite pixel by pixel, SETTLE | 0.5 to 1.5 s one-shot |

The storyboard has these parts, in order, in your reply:

1. **The idea in one line**, and the role.
2. **Moments**: a table: moment, frames, seconds, a caption per frame.
3. **Parts**: the main object, its pieces with the `forms` each takes (a wing folded, half open, spread, swept), the
   small effects, the light; a `pivot` for each piece that turns.
4. **Math model**. Show the anchors, geometry, paths and timing curves that drive each beat, using the workflow below.
5. **Parameters and composition**. Show what the game sets, which parts play independently, and how they fit together.
6. **Look**: palette (from a `pixfx_draw::palette` preset or the nearest effect), glow painted in steps
   (`pixfx_draw::halo`), category and id.
7. **Scale**: each object's size against the ruler, as a body part and a ratio (skill `pixfx`, "One scale: the
   ruler"): "the aura reaches a head above the head: 7 px", "each shard 2 hands: 4 px", "the smear as tall as the
   body: 40 px".
   Nothing under 1 px.
8. **Reference and cost**. State which reference will drive the painter and list any proposed paid sheets.
9. **Demo use cases**. Use [pixfx-demo](../pixfx-demo/SKILL.md) to identify distinct ways to use the effect.
   Carry the selected scenarios into stop 5 when building the website demo.

### 2a. Model the effect before drawing pixels

- **Find the structure.** Break each beat into visible parts and say what drives each one: an actor anchor,
  silhouette edge, trace sample, or UI point or region. Define the local origin, axes, pivots and units. Sketch the
  underlying points, bones, centerlines, contours or masks that the painter will turn into pixels. Use the geometry
  the effect needs. A dissolve can start with a silhouette and a moving mask boundary.
- **Write the motion.** Use the simplest equations that explain what changes. Define the time variable and its
  domain, how beats join or loop, and the formulas for the paths and envelopes that matter (position, rotation,
  radius, width, opacity or emission). Spell out counts and spawn order where they affect the read. For example,
  an orbit can use `u = clamp(t / duration, 0, 1)`, `a(u) = a0 + 2π × turns × u`, and
  `p(u) = anchor(u) + r(u) × (cos(a(u)), sin(a(u)))`. Here `t` and `duration` are seconds, `a0` is radians,
  `r` is in game pixels, and the anchor comes from the sprite.
- **Plot it.** Sample the same equations for a wireframe or mask sketch with paths and marked beat times, and plot
  the governing values against time with units. Use a few samples for a simple effect. Check the start, peak and
  end, joins and loop seams where applicable, and whether paths and thin features still read on the ruler's
  whole-pixel grid. Adjust the model before adding palette, pixel clusters and additive light.
- **Name the parameters.** Give each meaningful parameter a unit (or type), default, valid range and visible
  influence. Distinguish game options from internal constants and values derived from the actor or trace. Check
  the default and representative extremes in the plots. Document a seed when randomness matters. The Rust model
  owns these rules and uses the project's deterministic math and rounding.
- **Try composing the parts.** For each candidate part, check whether it has an independent role and can reuse
  an existing effect or primitive. Specify the shared anchor or event, start offset, lifetime and layer order.
  Compose independently playable effects in the game or scene. Within one painter, reuse atomic drawing or
  simulation primitives at the lowest layer the callers share. Extract a new shared primitive only with at least
  two actual callers and no benchmark regression. Keep tightly coupled parts local and say what state they share.

Use the model to drive the Rust painter. Reference art guides the pixel treatment. Carry the formulas and
parameter choices into the painter review so the rendered motion can be checked against the storyboard.

### 2b. Sketch the reference sheets when generating art

For the sheets you plan to generate, write the spec (`crates/pixfx-pixel/examples/specs/<category>/<name>.json`, from
`TEMPLATE.json`; the moments' captions are the rows' `notes`) and run
`cargo run -q -p pixfx-pixel --example template -- <spec> --parts-sheet` and one `--moment` (it needs `--parts`: pass
any PNG until the parts sheet is drawn, and run that `template --moment` again with the drawn parts sheet before its
`generate`, or the placeholder is sent as the parts). Open the `template.png` files in Chrome: labelled cells with
the figure, the layout the model will draw into. Free, nothing sent anywhere.

Keep the reply scannable: the moments as a table, the rest as short lists; the long wording goes into the spec.
Then ask the forks (at most 3), with the storyboard and the sketch in front of the user.

## 3. Then the pipeline: one step, one review page, one answer

Use only the stops that apply to the chosen reference and requested work. With a supplied GIF or a direct
painter, go from stop 1 to stop 4. Each applicable review stop has three parts:

1. **A review page**: write `target/ref/<slug>/review-<n>-<step>.html` (`<slug>` is the id with `/` as `_`). It is
   the step's whole result on one page: a heading, a few lines saying what changed since the last page and every
   drift or doubt you see, then the pictures (`<img>` with paths relative to the page, gifs animate). Pixel art
   shows sharp: `img { image-rendering: pixelated }`, scaled up with a CSS `width`.
2. **Open it**: `open -a "Google Chrome" target/ref/<slug>/review-<n>-<step>.html`.
3. **Ask** with `AskUserQuestion` (keep going / change what you name / redraw), give the page's path in the reply,
   and end the turn there. The next step's first command runs after the answer.

| n | Step | Skill | Its review page shows |
|---|---|---|---|
| 1 | Storyboard and model | this one | the moments, parts, formulas, wireframe or mask sketch, plots, parameters and composition decision. Include the supplied reference and key frames or any proposed templates and generation cost |
| 2 | Optional parts sheet: template, `generate --dry-run`, draw, extract | `pixfx-ref` | the drawn sheet beside its template, and a link to `parts/review/index.html` (the cut cells, colour and light) |
| 3 | Optional moment sheets, extract and assemble | `pixfx-ref` | the generated frames, links to their reviews, and any differences from the design. Show `whole.gif` and `whole.png` when assembling a full reference |
| 4 | Painter code from the model | `pixfx-effect` | the effect's GIF beside the chosen reference and key frames at corresponding beats, when available. Check the paths and timing against the model on every route |
| 5 | Website demos, options, site sliders and bench | `pixfx-demo` for demos, `pixfx-effect` for options and bench | the playable use cases, the options or composition behind each, and the bench numbers |
| 6 | Goldens, CHANGELOG | `pixfx-effect` | what was blessed and the test output; bless only after the user said the look is right at stop 4 or 5 |
| 7 | Commit | | only when asked; own hunks through a temporary index |

A redraw that fails once on a count or a motion is not redrawn again: code it, and say so on the next page.

Joining a run already under way (the skill loaded after some steps were done): write the review page of the last
finished step now, open it, and stop there.

## Common mistakes

| Mistake | Instead |
|---|---|
| Asking open questions before any draft ("what style?", "where is the enemy?") | Draft the storyboard with your picks, then ask the forks as choices |
| A question with no recommendation | Your pick first, "(Recommended)", and what each option changes |
| Spending on sheets before the user saw the storyboard and the templates | The templates are free; `generate` waits for a go |
| Running several steps in one turn, with one question at the end | One step per turn: its review page, opened, then the question |
| Paths to PNGs and gifs listed in chat as the review | A `review-<n>-<step>.html` page that holds them all, opened in Chrome |
| Ten questions at once | At most 3; the rest are your calls, stated in the storyboard |
| Redrawing a supplied GIF that already shows the desired effect | Sample the GIF locally, fit and plot the model, and use that GIF for the render comparison |
| Sizing an object by eye, or copying the sheet's size at 64 squares as engine pixels | Size it against the ruler (a part and a ratio, `Ruler::size`); a 64-square sheet draws 2/3 of engine size |
| A new spec for something already drawn | Search `target/ref` and the specs first |
