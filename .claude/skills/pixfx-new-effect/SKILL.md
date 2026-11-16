---
name: pixfx-new-effect
description: Use when the user asks for a new pixfx effect from an idea, often one sentence ("make a phoenix that dives at the enemy", "a sword formation round the character", "another shield like the lotus"), wants help designing or storyboarding an effect, or says they do not want to think the design through themselves. The entry point before pixfx-ref and pixfx-effect.
---

# A new pixfx effect, from an idea to the library

The user gives an idea; **you do the design thinking**. You research, draft the whole storyboard with your own
picks, sketch it, and bring the user only the few real forks, as choices with your recommendation. Then you take it
through the reference sheets (skill `pixfx-ref`) and the painter code (skill `pixfx-effect`), stopping at the
checkpoints below. Run from `tools/pixfx-rs`.

## The questions: propose, never interrogate

Every question goes through `AskUserQuestion`, at most 3 at a time, and each one:

- comes **after** your draft, about a fork the draft could not settle (the user cannot answer what they have not
  seen);
- has 2 to 4 options, **your pick first, marked "(Recommended)"**, each option saying what it changes in the effect;
- shows the alternatives in `preview` when they look different: a few lines of text sketch (what each moment shows);
- is a choice. "What style do you want?" makes the user design; "Gold spirit phoenix (Recommended) / Red fire
  phoenix / Ice phoenix" lets them pick.

A fork you can settle from the repo or from convention is not a question: decide it, and say it in one line in the
storyboard ("Category: `fire`, next to `fire/brazier`").

## 1. Research (no questions yet)

- `rg -n -i '<idea words>' crates/pixfx-effects/src crates/pixfx-ref/examples/specs` and `target/ref/*/`: an effect
  or a drawn sheet may already exist (`shield/sword-ring` was drawn and never written). Reuse what exists; say so.
- The closest existing effects: their duration, size, origin, how their moments run. Read their files.
- How the game will use it: the role table in `site/assets/effect-roles.js` (strike, projectile, ward, buff,
  volley, ...).
- Does it reach a target that may be far off (swords, arrows, talismans, a barrage)? Then the far part is not drawn
  on the effect's own canvas: the effect takes an order and the scene flies its shots (the volley row below;
  `arcane/sword-formation` is the worked example).

## 2. Storyboard: draft it whole

Pick the role, then build the moments from its beats. Each moment gets 4 to 6 drawn frames, a caption per frame, a
duration, and what makes it read: anticipation before the action, an impact frame with a cue (hit-stop, shake),
follow-through, a settle.

| Role | Moments (beats) | Usual length |
|---|---|---|
| ward / shield | INIT (forms), IDLE (holds, loops), HIT (takes a blow), BREAK (ends) | 3 to 4 s one-shot |
| strike on the foe | CAST (gathers, anticipation), TRAVEL or APPEAR, IMPACT (cue), AFTER (debris, fade) | 1 to 2 s |
| projectile | SPAWN, FLY (loops), HIT | 0.5 s plus flight |
| buff on the caster | GATHER, BURST (cue), AURA (loops), FADE | 1 s plus aura |
| ambient | one LOOP | 1 to 4 s loop |
| volley: hits a point at any distance when the game orders it | GATHER, HOLD (loops until ordered), then each SHOT flies (its own effect, the scene flies it), STRIKE (a hit effect where it lands, with the cue), SCATTER or spent | opening once, then a loop; the flight is the scene's |

A volley is three effects: the formation that holds and is ordered, the piece that flies (drawn pointing along +x,
no streak of its own: its trail is `projectiles/trail` swept along its real path, in its colours), and the strike
played where a piece meets a body. Storyboard all three; the parts sheet holds the
shot and the strike as parts.

The storyboard has these parts, in order, in your reply:

1. **The idea in one line**, and the role.
2. **Moments**: a table: moment, frames, seconds, a caption per frame.
3. **Parts**: the main object, its pieces with the `forms` each takes (a wing folded, half open, spread, swept), the
   small effects, the light; a `pivot` for each piece that turns.
4. **Exact in code**: counts, order, paths, spin ("8 swords, launched 0.1 s apart"). The image model will not keep
   these across sheets; the code will.
5. **What the game sets**: where a blow lands (`hitX`, `hitY`), size, colour as options; a target that may be far off
   as an **order** (`effect.attack(x, y, "converge" | "pierce")`), never a target option sized to the canvas.
6. **Look**: palette (from a `pixfx_draw::palette` preset or the nearest effect), glow painted in steps
   (`pixfx_draw::halo`), category and id.
7. **Scale**: each object's size against the ruler, as a body part and a ratio (skill `pixfx`, "One scale: the
   ruler"): "the bell's rim 3 heads wide: 21 px", "each sword one blade: 16 px", "a talisman 2 hands tall: 4 px".
   Nothing under 1 px.
8. **Cost**: sheets to draw (a parts sheet and one per moment, about $0.07 each).

Then **sketch it**: write the spec (`crates/pixfx-ref/examples/specs/<category>/<name>.json`, from
`TEMPLATE.json`; the moments' captions are the rows' `notes`) and run
`cargo run -q -p pixfx-ref --example template -- <spec> --parts-sheet` and one `--moment` (it needs `--parts`: pass
any PNG until the parts sheet is drawn, and run that `template --moment` again with the drawn parts sheet before its
`generate`, or the placeholder is sent as the parts). Open the `template.png` files in Chrome: labelled cells with
the figure, the layout the model will draw into. Free, nothing sent anywhere.

Keep the reply scannable: the moments as a table, the rest as short lists; the long wording goes into the spec.
Then ask the forks (at most 3), with the storyboard and the sketch in front of the user.

## 3. Then the pipeline: one step, one review page, one answer

The work runs one step at a time. A step ends at its stop, and the user's answer starts the next step; this holds
even when the user's request names only some stops, or none. Every stop is the same three things, in this order:

1. **A review page**: write `target/ref/<slug>/review-<n>-<step>.html` (`<slug>` is the id with `/` as `_`). It is
   the step's whole result on one page: a heading, a few lines saying what changed since the last page and every
   drift or doubt you see, then the pictures (`<img>` with paths relative to the page, gifs animate). Pixel art
   shows sharp: `img { image-rendering: pixelated }`, scaled up with a CSS `width`.
2. **Open it**: `open -a "Google Chrome" target/ref/<slug>/review-<n>-<step>.html`.
3. **Ask** with `AskUserQuestion` (keep going / change what you name / redraw), give the page's path in the reply,
   and end the turn there. The next step's first command runs after the answer.

| n | Step | Skill | Its review page shows |
|---|---|---|---|
| 1 | Storyboard and sketch | this one | the moments table, parts, exact-in-code list, cost, and every `template.png`. **Nothing is spent before this answer** |
| 2 | Parts sheet: template, `generate --dry-run`, draw, extract | `pixfx-ref` | the drawn sheet beside its template, and a link to `parts/review/index.html` (the cut cells, colour and light) |
| 3 | Moment sheets, in parallel; extract; `whole` | `pixfx-ref` | `whole.gif`, `whole.png`, a link to each `<moment>/review/index.html`, and every drift from MAIN named (count, size, a cracked form too early) |
| 4 | Painter code, a function per part; a held frame beside the sheet, a round per difference | `pixfx-effect` | the effect's gif beside `whole.gif`, and held frames beside the sheet frames they copy |
| 5 | Options (`hitX`/`hitY`, targets), site role and sliders, bench | `pixfx-effect` | the gif at a few option values (`gif ... --num hitX=-20 --out target/ref/<slug>/opt-a`), the bench numbers |
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
| Taking a count or a motion from the sheets | Write it in code; say so in "Exact in code" |
| Sizing an object by eye, or copying the sheet's size at 64 squares as engine pixels | Size it against the ruler (a part and a ratio, `Ruler::size`); a 64-square sheet draws 2/3 of engine size |
| A new spec for something already drawn | Search `target/ref` and the specs first |
| A `targetX`/`targetY` option for a foe that may stand far off (the canvas caps the reach) | An effect that takes orders: the game calls `attack(x, y, aim)` and the scene flies the shots (skill `pixfx-effect`, "Hit at any distance") |
