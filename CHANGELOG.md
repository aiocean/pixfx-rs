# Changelog

## Unreleased

- Effects: `shadow/winter-ordeal`, the winter shadow of heaven, made here. Over the foe the
  night sky darkens in and cracks like glass: jagged cracks run out from a keystone where an eye opens, and rings of
  cracks cut the sky into slabs. Then the sky falls: ring by ring the slabs tremble, drop and turn, showing a lit top
  edge and a dark underside, and smash on the ground into shards that crumble to dust; heavy ones shake the screen a
  little. Last, the keystone carries the open eye down onto the foe's head at 1.78 s (the blow cue). The crack pattern
  (37 slabs, their shards, and each crack cell with the time it appears) is baked once by
  `examples/bake_winter_ordeal.rs` into `shadow/winter_ordeal_sky.rs`, so nothing about the cracks is worked out in a
  game. On the site it plays as a strike landing at 1.78 s.
- Drawing: `pixfx_draw::shatter` breaks a shape like glass. Bolts of cracks (`growth::dbm`) strike one after another,
  and every region the cracks wall in becomes a piece. `Hold` decides which pieces fall: `Hold::Rim` lets only pieces
  walled in by cracks fall, and `Hold::At` lets every piece fall except the one holding that point. `Cracks` sets the
  strikes, `eta`, `noise`, `reach` and the smallest piece. Each piece knows the strike that freed it, and the part
  still standing is `rest`. `examples/shatter.rs` draws it frame by frame.
- Drawing: `growth::dbm` runs about 10x faster (a 152-node bolt: 625 ms to about 60 ms) with the same output, bit for
  bit. Neighbours are looked up from a precomputed table, and each candidate keeps its noise.
- Body: `pixfx-body`, one rig for every pose, direction and animation. A figure on the ruler is posed by joint
  angles with hands, feet, roll and spin, and seen by any camera: side (4 directions), iso and top-down (8), each
  turned in 3D, never flipped. Clips key poses with easing, loop or play once, carry root motion and plant a foot or
  hand by IK while it is in contact. The catalog has the ground clips (idle, walk, run, run start and stop, sprint,
  skid, turn, crouch, crouch walk) and every preset as a still. The mannequin is painted on the ruler from a hull of
  shapes per body part with the bones inside it, plus a part map. `clip_sheet` lays a clip out a row per direction
  and a column per frame with the frames' joints and root motion as JSON; `examples/clip_sheet.rs` and
  `examples/catalog.rs` write the PNGs, a GIF and the JSON. `pixfx-pose` builds on the same rig.
- Effects: `shadow/blood-shield`, a dome of blood with veins (dark cultivation fantasy), made from `pixfx-ref` sheets.
  A pool spreads under the feet, veins grow out of it up round the character with glowing tips, and a see-through
  film fills in behind them. The dome is lit from the upper left, has a Fresnel rim and a highlight crescent, and holds
  with a heartbeat that pulses out along the veins. A blow (`hitX`, `hitY`) flashes, dents, ripples and splatters.
  At the end the bubble pops: the rim breaks into shards and drops, then the veins draw back into the pool and the pool
  shrinks away. On the site it plays as a ward with its blow at 1.6 s, with hit sliders.
- Drawing: `pixfx_draw::growth`, branching growth for veins, lightning, cracks and roots. Every setting is a field.
  `dbm` is the dielectric breakdown model (a Laplace field relaxed on a grid: `eta`, grounded or insulating walls,
  `drift`, `noise`, an optional target). `colonize` and `colonize_from` are space colonization (`attractors`,
  `influence`, `kill`, `bias`, `jitter`, `branch_chance`, `max_turn`, `fork_angle`). `Tree::path` lays a path you
  set, so growth branches off it. `jag` wanders a line by midpoint displacement. `Tree::prune` drops stubs, and
  `Tree::murray` sets fork angles by Murray's law from branch thickness. `Tree::radii` and `stroke` taper the strokes
  smoothly from trunk to tip and pass each pixel's normal for shading. `examples/growth.rs` draws a panel of each.

- Duel: frame data (`docs/superpowers/specs/2026-11-16-duel-frames-design.md`, numbers in `pixfx_duel::data::FRAMES`).
  A blow on a fighter gathering a cast is a COUNTER, on one recovering a PUNISH: 1.2x power and a longer stagger
  (+0.10 s, +0.20 s), and the hit event says which (`how`). Dive, step, palm, burst and lotus recover longer and an
  extra 0.15 s when guarded; jet, heal and the ultimates recover longer too. Casting drops a guard. An ultimate's first
  0.30 s cannot be hit (the blow passes through as a dodge). A counter or a punish stops the fight 2 or 3 ticks
  longer and floats its word over the damage (`pixfx_fighter::feel`). The policy sees each act's phase, the seconds
  left in it and each skill's windup and recover (`OBS` 250 -> 354), and is retrained on these rules (3000 updates):
  it beats the brain 94.8% and the first mind 95.3% over 150 seeds, with no round cut.
- Duel: `examples/ladder.rs --telemetry` prints, for each mirror, what each school's skills did (casts, landed,
  damage, guarded, broken, punished, final blows) and each school's totals.
- Train: long runs go on a training machine as a launchd job (`train/long-run.sh`, which resumes from the last checkpoint
  when the trainer dies); `train/README.md` "Training on a remote host".
- Duel: the two schools win about even in the policy mirror (Thunder 49.1% over 150 seeds, from 40.2%) with no rule
  changed. The rules were not what held Thunder back: the brain's mirror stays at 50.9%. The policy played Thunder
  worse, so it is fine-tuned (600 updates) with the learner on Thunder in 90% of its fights. It beats the brain
  95.9% (95.1% as Thunder, 96.7% as Frost) and the first mind 95.4%. `ppo.py --thunder` sets that share (0.5 by
  default), and the training log gives each opponent's win rate per school (`win/latest/thunder`, ...).

- Fighter: the duck, launched, tumble and fall poses are redrawn by hand from reference drawings
  (`pixfx-pixel` clean and read), each with the whole costume as the standing poses wear it: the crown and its
  ribbon, the crossed collar's gold edge over the red inner robe, the sash with its buckle, jade pendant and streaming
  tail, the sword in its scabbard with hilt and tassel, gold cuffs, the hem, the boots. The duck is a deep squat with
  an arm over the head instead of a low block; launched and fall are a size with the other poses (50x39 and 49x34
  instead of 56x46 and 59x35) and keep their details on the slant. Diagonal bands of one colour are drawn with the
  new `Pen::line4`, whose pixels touch by an edge, so they read as one stroke: the poses' single loose pixels drop
  from 44 to 12 (launched) and from 36 to 6 (fall). `Pen::quad` fills a shape with any number of corners.
  The limbs no longer swing every few frames in these poses (`flail` is gone, with `Pen::disc` and `Pen::leg`,
  which only they used).

- The duel's trained policy is a file of its own: the wasm no longer carries `policy.bin` (1.35 MB to 660 KB gzip),
  and the painting workers no longer hold a copy of it. Each corner seats a policy of its own. `pixfx-wasm`:
  `new Policy(bytes)` reads a weights file once; `Duel.withPolicies(config, seats?, left, right)` seats one per corner
  (the same one twice for a mirror), and `new Duel(config, seats?)` seats the brain (`"brain" | "first"`). The site's
  `duel.js` takes them with `usePolicies([left, right])`; the landing page names each corner's file in
  `POLICY_FILES` (both `policy.bin` today), fetches each file once after the engine is in, so the first frame never
  waits for them, and seats the brain until they arrive. `site/build.sh` copies every `crates/pixfx-duel/weights/*.bin`.

- `arcane/sword-slash` (new): a spirit sword appears beside the foe, winds back and cuts down through it in one fast
  arc. The blade sweeps a crescent of its own pixels (each blade row's edge colour, `pixfx_draw::sweep` with the brush
  turned with the blade), white at the blade and cooling to steel behind it; the impact cue fires as the blade points
  through `hitX`, `hitY`; it overshoots and crumbles from the tip. `size` (1 to 3, 2 by default) draws the spirit
  sword that many times its size. On the site it is a strike: the caster summons it on the foe.

- Motion and sweep: every streak is one brush swept along a path, as a camera sees a bright thing waved in a long
  exposure. `pixfx_draw::sweep` paints a `Brush` (a column of cells, each a colour and a light; `Brush::column`
  from a palette and a width for a thin mover, `Brush::edge` for a broad body's back edge (`inset` steps in, past a sprite's outline), or
  `Brush::from_column` from a sprite's own column) along poses, square to the way
  it goes or turned with the thing's angle (a turning bar sweeps a fan), each pixel showing the cell that passed it
  last, its colour, width, wave, dither and light set by how long ago (`Look`); no holes however fast it turns.
  `pixfx_core::motion` reads a `Trace` (now with each point's angle) once for everyone: `at`, `velocity`,
  `speed`, `heading`, `turn`, `spin`, `acceleration`, `poses`. `projectiles/trail` (new) is a column swept
  along its mover's trace (`Scene::trail`): straight or curving with it, longer the faster it went, waving and
  breaking up toward its end, behind the character; played on its own and moved by the game it trails itself, and
  played on a character it wraps the sprite and smears the body itself (its back edge, upright, in its own colours: a
  fighter gliding leaves an afterimage), and `turn` 1 turns it with its mover's angle. `fire/campfire`, `fire/brazier`,
  `fire/candle` and `scifi/jetpack` take `carried` 1: carried along, their flames and plumes bend back against the
  motion (and the brazier's and the candle's smoke drift back); not carried or standing still, their pixels are
  unchanged. `explosions/body-burst` smears each flying chunk's back edge along its arc. The `blood_spray` script
  (`crates/pixfx-export/examples`) throws drops in arcs, each a `projectiles/trail` in blood colours that it moves as
  a game moves any mover, and `carry` carries any effect back and forth with `carried` 1. On the site the trail
  rides the two fighters as they glide, with sliders for its seconds, width and wave; a fighter thrown back by a heavy
  blow leaves an afterimage as it flies; the jetpack is played carried, so its plumes lean as it flies.

- `arcane/bagua-mirror` and `arcane/bagua-mirror-strike` (new): the bagua mirror, an octagonal bronze mirror ringed
  with the eight trigrams round a jade face, in two forms. Both wrap the sprite they play on: the mirror is sized and
  placed from its body, spins up out of a point of light behind the character's back, and a jade line washes down its
  outline. The shield form sways and lights its trigrams in turn; a blow (`hitX`, `hitY`) flashes its face, flares
  the outline, knocks the mirror away so it trembles, squashes and springs back, fires an impact cue (shake,
  hit-stop) and is thrown back as a beam; it ends by spinning down to a point. The strike form holds the same way
  until the game orders it (`Scene::order`): `arcane/bagua-mirror-flying` leaves from behind the caster spinning
  like a thrown coin, its jade rim smeared behind it along the way it flies (`pixfx_draw::sweep` over the trace the
  scene hands it), shrinking to half, and bursts on the target as `arcane/bagua-mirror-burst`
  (white flash, octagonal shock ring, the trigrams flung out, shards, a heavy impact cue). The mirror painter and the
  outline are shared in `arcane/bagua.rs`, with colours from the drawn parts sheet. Made from `pixfx-ref` sheets
  (`examples/specs/shield/bagua-mirror.json`). On the site the shield is a `ward` with hit sliders and the strike a
  `volley`.
- `pixfx_core::render_frames_on_actor` and `gif --show-actor` draw the sprite too, between the effect's back and
  front layers, so a sprite-aware effect can be previewed on its character. The `stage` script
  (`crates/pixfx-export/examples/stage.rs`) plays an effect on a stage between two characters, orders it at the
  target (`--order-at`) and answers its cues as a game does (`--feel`: hit-stop and screen shake).
- Effects that take orders, for any distance (`pixfx_core::volley`): an effect can answer `Effect::order` (a point,
  an aim, `converge` or `pierce`, and the caster's body) with a volley of pieces, each another effect; `Volley::aim`
  plans one for any like pieces (nearest the target first, a gap apart). The point gives the direction, not where a
  piece bursts: each piece hangs where it was (`held`), turned toward its way, then flies along its path and on past
  the point, and the scene strikes it on the first body it meets (`Scene::set_body`, the caster's spared): a
  `strike` event naming the body and the hit effect played there; a piercing piece flies through every body, and
  the game can burst a piece itself (`Scene::burst`). A piece that meets nothing fades out past the point. Pieces
  keep their caster's mirroring and turn to their heading (32 headings, so frames are shared); each can carry a
  trail (`tail`). The ordered effect ends with its last piece. A looping effect can
  open once and then loop only what follows (`Effect::loop_from`). JS: `effect.attack(x, y, aim, caster)`, `pixfx.setBody`
  and `clearBody` on every shell, `effect.burst()` and the effect's `"strike"` event (`body`: the one struck);
  gpui: `FxScene::order`, `set_body`, `burst`, `launched` and `events`. Workers paint any effect the scene plays,
  its spec made from its clip (`options_json`), so `scene.play` no longer takes `frames`.
- `arcane/sword-formation` (new): eight flying swords gather one by one behind the caster, fan out together into a
  ring and turn there until ordered; then each hangs where it is, turning to its way (`arcane/held-sword`, new), and
  they fly one after another, 0.1 s apart, at 300 pixels a second, curving in on the point and on past it, bursting
  on the first body they meet. Its pieces are `arcane/flying-sword` (new: opens with a flash and specks) trailing
  `projectiles/trail`, and, where each strikes, `arcane/sword-strike` (new). Made here from `pixfx-ref`
  sheets (`examples/specs/arcane/sword-formation.json`); the count, order and paths are in code. On the site it
  plays the `volley` role: it gathers at the caster, holds, then is ordered at the foe 150 px away.
- The landing duel plays a rotating skill bar: three slots, each a skill drawn at random from the school's pool
  (now 11 for Thunder and 12 for Frost, seven of them new) with 1 to 4 casts (fewer hit harder); a spent slot counts
  down and draws again, and a skill whose partner is on the bar evolves. Chill and shock stack, payoff skills spend
  them, repeated control wears off (grit), a guard turns heavy blows aside less often, wards hold charges, heals and
  wards cleanse, and every spell has its own icon.
- The landing duel's bar reads at a glance (casts left, the seconds to the next skill, rarity in the frame, trait marks,
  rage lighting the ultimate) and plays with more chance: each fight a random 8 skills per school, a rarity and traits
  on every draw, and lucky casts (double, free, again). Thunder and Frost now win about even, and pixfx-mind's `Sense`
  takes a `power` per skill so a stronger skill is valued higher.
- `crates/pixfx-ref` (new): a new effect's reference from an image model, in Rust. A spec (look, moments as rows, a
  frame count per row, a note per frame, rules) becomes a labelled template sheet and a prompt (`template`); the
  OpenRouter request goes out through curl (`generate`, transparent background, spend logged to
  `target/ref/spend.jsonl`); the drawn sheet is cut back into frames as colour and light layers with a review page
  and a gallery (`extract`, `gallery`). The eleven xianxia shields are its worked specs; `TEMPLATE.json` shows every
  field. Skill `.claude/skills/pixfx-ref`. `pixfx_pose::captions` writes captions over a sheet's cells.
- `pixfx_pixel::frames::recover_frames`: the pixel grid shared by a sheet's frames. The pitch comes from the gaps
  between edges of every frame at once, every frame gets the same pixel count, and each frame fits its own cuts, with
  a strength that is 0 for noise. `pixfx-ref`'s `extract` uses it: its layers are the model's pixels, the same size in
  every frame, and the review page shows each frame's cuts.
- `pixfx_pixel::register`: the frames of one drawing lined up on one origin, a whole-pixel shift each from their
  edges, a frame that changes rather than moves left in place. `extract` moves each frame's layers by it and the
  review page says how far.
- `pixfx_ref::tween`: frames between the drawn ones of a row, made by moving their pixels. Pixels pair with the
  pixel they become in the next frame, every cell follows a dense field of their moves, pixels with no partner grow
  or erode from the other frame's shape, and every cell of an in-between is gathered, so none is left empty; unlike
  colours swap at a time of their own instead of mixing to grey. The review page's player plays 1, 3 or 7 of them
  between drawn frames (`extract --between N`, default 7).
- `pixfx_ref::matrix`: every frame as a pixel matrix, each pixel empty, colour or light with its exact value and an
  entry of a palette the frames share (32 colour, 16 light). `extract` writes it as `matrix.json` (text maps per
  frame, exact values, palettes) and the review page shows the palette and each frame through it; its
  swatches are colour pickers with a hue slider, and the player's recoloured layer repaints every frame live.
- `pixfx-ref` sheets are drawn for redrawing: with `grid` every cell shows its pixel grid (64 art pixels a side for
  the shields) and the prompt asks for whole squares in flat colour from a small shared palette; with `parts` a
  PARTS row comes first, the effect taken apart (the main object, its pieces, the small effects, the light), each
  drawn alone and whole. `extract` cuts a grid sheet on the template's columns and the label lines, so boxed labels,
  cell frames and squashed rows do not move it, and names the parts `parts-<name>`, leaving them out of diffs,
  in-betweens and the player. `pixfx_ref::entropy` measures each step's change as conditional entropy, and
  `pixfx_ref::compare` with its `compare` script puts an effect's frames beside the sheet's cells at the same scale
  with the steps of both. The eleven shield specs are grid sheets with parts; the skill teaches the whole loop.
  A part can have `forms` (each shape it takes, a row of its own) and a `pivot` the model marks in green and
  `extract` reads; a spec with `layers` is drawn as a parts sheet, then a sheet per moment built from it, each
  moment once per layer (`SOLID`, `FX`, `LIGHT`). `extract` finds cells from the cyan frames the model keeps and,
  on cells it drew not square, its own pixel size; a pivot per green mark, marks painted over in every cell, and
  frame lines the model draws through a cell cleared. A moment's prompt keeps the main object at MAIN's shape, number
  of pieces and size, with damaged forms only where it is struck or ends. The `whole` script plays every moment's
  whole frames in order as `whole.gif` and `whole.png`.
- `holy/lotus` (new, feature `holy-lotus`): a pink lotus bud rises from its pad, breathes, shivers open under a blow
  and drifts apart in petals; written from the parts of its grid sheet. Made here, no parity source.
- `arcane/restriction` (new, feature `arcane-restriction`): three rune rings write themselves round the character one
  character at a time from the ground up, each ring spinning on its own; a blow lands anywhere (`hitX`, `hitY`) and
  the nearest ring flares from that point, tilts away from it and throws sparks; the rings unravel from the top, rune
  by rune. Its glow is painted in steps of its ramp round every stroke, as on the reference sheet, with
  `pixfx_draw::halo` (new: strokes stamped with a heat and a kind, spread a fixed fall a pixel as a rounded octagon,
  walked only over the box they cover). Written from the
  parts sheet of its `pixfx-ref` spec. Made here, no parity source. The gif, sheet, bake and mp4 scripts take
  `--num NAME=V` for an effect's own number options.
- `holy/golden-bell` redrawn in flat bands of five golds with a one-pixel outline, a waist band and a white flash.
- Skills for an agent working in the repo, in `.claude/skills/`: `pixfx` (where to start), `pixfx-effect`,
  `pixfx-play`, `pixfx-render`, `pixfx-perf`; `pixfx-pixel` moves there from `skills/`. The export scripts take
  `--seed N`, and `pixfx_export::write_video` with its `mp4` script writes a video through ffmpeg.
- Sprite awareness: `pixfx_core::Silhouette` reads the sprite an effect wraps into cells of the effect's space
  (`Area`): the body (`cells`, `covers`, `bounds`, `centroid`), its `edge`, an outline `ring`, the cells `around` it
  with their distance, and `pick` for a random cell of it; `Actor::opaque` gives the sprite's pixels with their colour.
  `feedback/power-up` is built on it, its frames unchanged. `.claude/skills/pixfx-sprite-aware` teaches writing such an effect.
- `feedback/outline` (new, feature `feedback-outline`): a ring round the sprite it wraps, following each frame; `width`
  1 to 3, `corners`, `palette` for its colour, `w`/`h` for the box the sprite fits in. Made here: it has no source to
  check parity against.
- The `outline` option is gone from every effect, with `outlinePresets()`, the export examples' `--outline` and
  `pixfx_draw::outline`: an outline is now the effect above, played on the sprite to ring. Effects whose art has an
  outline (rocks, clods, the coin) draw it as designed. `ClipKey` is at version 2: clips baked before are refused.

- Each crate and package tests itself; there is no repo-wide gate. One effect builds, tests and benches alone behind
  its feature (`cargo test -p pixfx-effects --no-default-features --features fire-candle --all-targets`): the tests
  about named effects run when those effects are built, the golden check compares each built effect with its own
  entry (bless keeps the others), and `bench` measures every effect the build has by default.
- pixfx ships as source, with no command-line tool: `pixfx-cli` and its `pixfx` binary are removed. You clone the
  repo and your agent imports the crates and JS packages, builds the wasm (`bun run build:wasm`), and runs or edits
  scripts. Each command became a cargo example in the crate whose library it calls: `pixfx-export` `sprite_sheet`
  (was `pixfx sheet`), `gif` (`pixfx render --gif`), `bake`, `parity`; `pixfx-effects` `bench`; `pixfx-fighter`
  `poses`; `pixfx-pixel` `clean`, `read`, `draw`, `view`, `critique`, `sheet`, `cut`, `parts`, `diff`. Flags and
  outputs are unchanged. The logic moved into the libraries: `pixfx_export::parity` (`compare`, `read_reference`,
  `write_side_gif`, `ParityOpts`, `Actors`), `pixfx_fighter::poses`, `pixfx_pixel::tool`, and `Look::colors`.
- `arcane/hex-shield` (new, feature `arcane-hex-shield`): a hemisphere of hex cells forms over the character, four
  bolts strike it, spark off and ripple over its surface, then it breaks up into shards. `radius` (16 to 40) sizes
  the dome, `hold` (0.8 to 2.75 s) sets when it breaks, `palette` recolours it, and each blow fires a light shake
  cue (`feel`). It is a port of Sprite Fusion's shield, matched pixel for pixel by `pixfx parity` against the
  vendored source.
- `pixfx parity` checks an effect ported from Sprite Fusion against its source vendored at
  `tools/spritefusion/<id>.ts` (through `tools/spritefusion-ref.mjs`, Node 22.6+), stepped as pixelfx steps its
  effects; pixelfx ports are checked as before.
- `feedback/power-up` (new, feature `feedback-power-up`): a buff in its stat's colour. Streaks are drawn in, the
  character flashes, a shock races along the floor, then an aura grips the body and licks upward with speed
  streaks and motes. `size` (0.25 to 2) scales it, `palette` recolours it, and the burst fires a shake and a flash
  cue (`feel`). It is a port of Sprite Fusion's power up, matched pixel for pixel by `pixfx parity` against the
  vendored source.
- An effect can play on a character: `effect.wrap(source)` hands it the sprite's current frame every update, and an
  effect that wraps its actor (`Effect::wraps_actor`; `feedback/power-up` does) grips that outline and follows each
  pose. Shells read the sprite for you: `this.pixfx.actorOf(sprite)` (Phaser), `pixfx.actorOf(sprite)` (Pixi),
  `imageActor(() => ({image, cut, x, y}))` (Canvas 2D, for the sprites a game draws), `FxScene::set_actor`
  (gpui). Rust places the sprite (`Scene::set_actor`), scales and flips it, and paints a wrapped effect's frames
  for that instance alone; workers and gpui threads get each sprite frame once. `pixfx parity --actor A.png,B.png
  [--actor-every N]` checks such an effect against its source on the same sprites.
- Attack timing (`Act`, `Performance`, `pose_for`) and `Effect::act` are removed: nothing used them. An attack's hit
  timing returns with the first attack effect a game plays; the caster's poses are the game's own sprite.
- `pixfx-web`, `pixfx-phaser`, `pixfx-pixi`, `pixfx-canvas2d`: `setDrawBudget(n)` caps the effect layers drawn per
  frame (`Scene::set_draw_budget`); 0, the default, draws every one that changed.
- `pixfx-gpui`: `FxScene::with_threads(n)` paints on native threads: each effect is stepped, drawn and turned into
  images on one thread, and the window's thread only paints images (100 fires: 1.4 ms a frame instead of 81 ms).
  `FxScene::default()` still paints on the spot.
- `crates/pixfx-canvas` is gone: `packages/pixfx-canvas2d` is the Canvas 2D shell, and the site's docs (Canvas 2D
  shell, getting started, the landing page's sample) teach it.
- `pixfx-phaser` runs on `pixfx-web` (its copy of the runtime is gone) and draws only what the main camera sees: an
  effect off camera plays on unseen and takes no paint or texture until the camera reaches it, and one with a
  `scrollFactor` (a HUD effect at `0`, parallax between) is culled where the camera shows it. A game's Vite config
  adds `optimizeDeps: { exclude: ["pixfx-web"] }`.
- `pixfx-scene`: `View` carries the camera's scroll (`scroll_x`, `scroll_y`), and `Scene::set_scroll_factor(handle,
  f)` says how far an effect moves with it. JS: `scene.setView(x, y, w, h, scrollX, scrollY)` and
  `scene.setScrollFactor(handle, f)`. `pixfx-web` exports `wasmEngine`.

- `pixfx-pixel` (new): pixels for agents, never shipped in a game. `pixfx pixel <verb>` runs it:
  - `clean`: an image model's picture down to one pixel a cell. The grid is found by autocorrelation and dynamic
    programming, with a confidence per cell; `--height` resamples to the game's sprite height, and colours are
    snapped to `--palette` or merged to 32 in OKLab.
  - `read` / `draw`: a sprite as text (a letter per cell, a legend naming a look's colours) and back.
  - `view`: a picture sized to what it costs an agent to look at.
  - `critique`: issues with cells and a fix each, and a score.
  - `sheet` / `cut`: blank sheets for an image model, with bones or a sprite as a guide, and the drawn cells cut back.
  - `parts`: a sprite split into named parts with parents and pivots, the ground for effects that take a sprite
    apart.

  Also `pixfx diff --anchor feet`, `pixfx poses <look> --pose P --states`, and the skill
  `skills/pixfx-pixel/SKILL.md`. The cut, match, shrink, rects and diff steps move here from `pixfx-draw` (`trace`
  is gone; `RgbaImage` stays, in `pixfx_draw::image`).
- `pixfx-fighter`: no pose is painted from a traced table any more; launched, tumble, fall and down are painted by
  their painter code again, as every other pose. `traced`, `paint_traced` and `pixfx trace` are gone.

- The site draws with `pixfx-canvas2d`: the landing duel, the effect pages, the gallery and the pointer fx paint on
  workers and load one engine (pixfx-web's wasm). Landing duel at 4x CPU throttling: 13-15 fps before, 58-60 after;
  at 6x: 11-15 before, 47-56 after.
- `pixfx-wasm`: the duel's bindings move here from `pixfx-canvas`, behind the `fighter` and `light` features (both
  default; +290 KB, a game builds without them): `Body`, `FightFeel`, `lookColors`, `Sprites` (now `new Sprites(w,
  h)` and `picture()`), and for light `stepLights(lights, dt)`, `putOutLight(light, within)` and `new Lighting(w,
  h).paint(lights, glows)`: the game keeps its own list of lights (`Lights` is gone). Pictures (`Picture`: the lit
  box, colour and light RGBA) are drawn by a shell. `scene.bend(src, w, h, unit)` bends a picture of the screen by
  the live ripples.
- `pixfx-canvas2d`: `drawPicture(picture)`, `warp()`; re-exports the engine's `effects`, `meta`, `Fx` and the fighter
  and light bindings. `pixfx-web`: `PixfxFeel.bend(src, w, h, unit)`.
- `pixfx-core`: a held frame counts at least one byte, so an unlit frame also goes past the budget (it was held
  forever, and which one stayed depended on hash order).
- `pixfx-canvas2d` (new): pixfx on a Canvas 2D context over `pixfx-web`, painting on workers. `createPixfx(ctx, {
  scale })`, `play(id, x, y, opts)`, and each frame `update(dt)` then `drawLayer("back")` / `drawLayer("front")` (or
  `draw()`) in the context's transform. `bun run build` writes `dist/` for pages without a bundler.
- `pixfx-web`: `Stage.live` is the Rust scene until `clear()`, then null; a feel built on it reads still after its
  shell is destroyed instead of throwing.
- `pixfx-pixi` (new): pixfx in a Pixi 8 application. `createPixfx(app, { container, camera })`, then `play(id, x, y,
  opts)` with the same effects, events and feel as Phaser; the shake moves and tilts a camera container, the flash
  covers the screen. On WebGL frames are regions of atlas pages filled with `texSubImage2D`.
- `pixfx-web` (new): the JS runtime every JS shell shares (wasm, painting workers, game-wide frames, a stage per
  host scene, effect and feel handles), host-neutral on eventemitter3.
- `pixfx-canvas`: a shared frame is held cut to the box that holds anything, so it takes the bytes the frame budget
  counts for it; what the stage shows is unchanged.
- `pixfx-phaser`: a pure shell over the wasm `Scene`. Placement, timing, which frame shows, fading, follow-stop,
  compositing and the frame cache come from Rust; workers simulate, paint, crop and convert, and transfer the bytes;
  on WebGL every surface is a region of a few atlas pages, filled with `texSubImage2D`. Removed: `createLayer` and
  `PixfxLayer` (no game needs layer groups; one atlas batch per layer costs less than re-merging a region whenever one
  effect changes), `PALETTE_PRESETS` (use `(await loadPixfx()).palettePresets()`), and the `textureBudget` load
  option. `loadBaked(url)` needs a scene with the plugin started. `pixfx-wasm`: `Frames.loadBaked(json, pages)`
  replaces `bakedFrames`. `pixfx-scene`: an effect has at most one frame in flight per layer with a painter
  elsewhere, and a frame that comes back late still shows once; a frame painted on the spot counts against the
  budget as its lit box (what shells upload), not its whole canvas.
- `pixfx-wasm`: the scene for JS shells (`src/scene.rs`). `new Frames(workers)` holds a game's shared frames;
  `frames.scene(scale)` makes a `Scene` per stage (`play`, `moveTo`, `set`, `stop`, `remove`, `update`, `events`,
  the feel, `setView`, `setDrawBudget`); `scene.draw(frames)` and `frames.endFrame()` return a draw list (an
  `Int32Array` of make, refill, show and free ops), each surface cropped to what it holds. With `new Frames(true)`
  frames are painted on workers: `frames.takeJobs()`, a worker's `Painting.paint(job)` paints, crops and turns the
  frame into RGBA, the worker transfers the bytes, and `scene.deliver(frames, job, shape)` takes back only their
  `Shape`, so pixels never pass through wasm on the main thread; they equal those painted on the spot. Painted on the
  spot (`new Frames(false)`), the bytes are at `frames.colorPtr(id)` and `lightPtr(id)` until the next call.
  `pixfx-scene`: `Frames::deliver` and `Scene::deliver_own` take a `Frame` or what the shell takes through
  `Receive<D>`. `palettePresets()` and `outlinePresets()` read the engine's presets.
  `Squash.step(dt, x, y, vx?, vy?)` scales a body for its velocity or, without one, for how far it moved, holding its
  scale through a hit-stop (`pixfx-core`: `squash::Moving`).
- `pixfx-scene`: painting leaves `Scene`. `Frames` holds the shared frames and a `Painter` for every scene of a game
  (`Frames::scene`), so a frame one scene painted shows in all of them; hosts end each frame with
  `Frames::end_frame`, and draw calls take the frames (`scene.draw(&mut frames, &mut shell)`). `InlinePainter`
  paints on the spot; a painter on another thread hands frames back with `Frames::deliver` and
  `Scene::deliver_own`, and an effect shows its last frame until then. `Scene::play` takes a `Clip`;
  `Scene::load`, `set_frame_budget` and `frame_bytes` are `Frames::deliver`, `set_budget` and `bytes`.
  `pixfx-core`: `Instance` is a `Clock` (time, stop, cues, fade, end) and a `Sim` (the effect's state, brought to a
  frame on demand); `Clip` is what an effect draws and how big. Every `frame_hash` is unchanged.
- `pixfx-scene`: `Scene::set_draw_budget(Some(n))` draws at most `n` layers per host frame (time-slicing); effects
  left over show their last frame and are drawn in a later host frame, the one waiting longest first, skipping to the
  frame their clock is on. `None` (the default) draws every layer whose frame changed, as before.
  `pixfx-gpui`: `FxScene::set_draw_budget`; `pixfx-canvas`: `Stage.setDrawBudget(layers)` (0 draws every one).
- `pixfx-scene` (new): `Scene` and the `Shell` contract move out of `pixfx-canvas` (`pixfx_canvas::stage`) so every
  shell shares them; `Shell` gains `free(surface)`, called for every surface the scene drops (past the frame budget,
  or when its effect ends), and `Scene::clear` frees all of them for a host shutting down.
- `pixfx-scene`: `Scene::draw_handle` shows one effect within a host frame and `Scene::end_frame` ends it, for a host
  that paints each effect on its own. `pixfx-gpui`: `FxScene` (`play`, `tick`, `stop`, `remove`, `paint`, `end_frame`)
  replaces `FxView`, `FxFrames` and `paint_fx`; it is a shell over `Scene`.
- `pixfx-draw` (new): frames, colour, painter, shapes, blending, palettes, outline rings and `math` move out of
  `pixfx-core`; import them from `pixfx_draw` (`pixfx_draw::Frame`, `pixfx_draw::color::Rgba`, ...). `pixfx-core`
  keeps effects, particles, cues, feel and the player. `pixfx-light` depends on `pixfx-draw` only.
- `pixfx-core`, `pixfx-wasm`, `pixfx-cli`, `pixfx-phaser`: every effect takes an `outline` (off, a preset, a hex colour
  or `{color, width, corners}`) and a `palette` by preset name (`fire`, `ice`, `poison`, `holy`, `shadow`, `arcane`,
  `blood`, `electric`, `nature`, `gold`); the CLI takes `--outline` and `--palette`.
- `pixfx-canvas`: the stage's own loop bends the view by the ripples alive; `Stage.warp()` does it for a host that
  owns the loop. `pixfx-core`: `screen::ripple_rgba` bends RGBA pixels at any scale.
- `pixfx-core`, `pixfx-wasm`: `HitFlash` fills a struck sprite for a few frames on play time, held through a hit-stop.
- `site/`: the hero's duel shows each fighter's skill bar on the dais floor: slots 1, 2 and 3 and the ultimate. The
  spells in a slot share one cooldown, which darkens the slot and sweeps back as it runs out; the ultimate's slot
  fills with rage. The icons are generated (OpenRouter `bytedance-seed/seedream-5-5`, with a frame of the duel as
  reference) and pixelated to 32 and 24 px atlases by `site/art/skill-icons.sh`.
- `pixfx-fighter`: the launched, tumble, fall and down poses of both looks are traced from generated art
  (`site/art/fighter-fall.prompt.txt`, Gemini 3 Pro filling the empty cells of a pose sheet) into rect tables in
  `src/traced/`, painted in the look's colours by role (`traced::pose`); these four poses are now still pictures.
- `pixfx-draw`: `trace` cuts a picture from its background, matches it to a palette, shrinks it to sprite pixels,
  merges the pixels into rects and diffs two images. `pixfx-cli`: `pixfx trace` turns a crop of art into a pose
  table, `pixfx diff` counts the pixels two images differ by and writes a mask, and `pixfx poses --blank` leaves
  chosen cells empty for an image model to fill. `pixfx-export`: `write_png_rgba`, `decode_png_rgba` and
  `pack_cells` are public, the one PNG encoder and decoder every file goes through.
- `site/`: the effect page picks a palette preset, an outline and (for impact effects) the feel, and copies the call.
- `pixfx-core`: the trauma shake follows the usual model on every axis: `Feel::roll(strength)` tilts the screen
  (radians, `FeelSpec::roll`) next to `shake`'s x and y, each axis is the most it can move times trauma squared times
  smooth Perlin noise (`math::perlin`, gradient noise that never comes to rest), `cue::impact` adds 0.3 to 0.6 trauma
  per hit, trauma drains 1 a second, and the default shake is 16 game pixels and 0.05 radians at full trauma (a heavy
  hit moves the screen 2 to 4 px, a light one 1 px; it was 1 px for both).
- `pixfx-wasm`, `pixfx-canvas`: `Feel.roll(strength)` and `Stage.roll(strength)`; the Canvas stage's own loop leaves
  roll out to keep the pixel grid. `pixfx-phaser`: `feel.roll`, and the plugin tilts the main camera by it for the
  render, keeping the game's own rotation.
- `pixfx-effects`: each effect is a cargo feature (`fire-candle`), each category a feature that turns on its effects
  (`fire`), and `all` (the default) every one. `pixfx-wasm` and `pixfx-canvas` build with
  `--no-default-features --features pixfx-effects/<effect or category>` to ship only the effects a game plays;
  `EffectId` then lists only those.
- Release builds favour play speed: fat LTO in one codegen unit (opt-level 3), and wasm-pack's `wasm-opt` pass at
  `-O3` for `pixfx-wasm` and `pixfx-canvas`. The 25-effect wasm is 235 KB gzip instead of 246 KB.
- `pixfx-canvas`: `Stage.move(handle, x, y)` moves a playing effect without restarting it, for effects that follow
  the pointer or a sprite.
- `site/`: landing page and docs, with an fx layer (`site/assets/fx.js`) that plays engine effects for the cursor,
  clicks, button presses and page changes as soon as each effect is in the registry. With fx on, pages change in
  place (`site/assets/nav.js`), so the old page burns open from the click point over the new one.
- `pixfx-core`: the simulation core the effects are built on, plus `Painter::clip_local`, `clip_box` and
  `dim_light`.
- `pixfx-effects`: 25 effects, listed in `pixfx_effects::EFFECTS`, each ported from pixelfx and checked by
  `pixfx parity`; the frame-hash determinism test covers all of them and is run on demand
  (`-- --ignored`).
- `pixfx-canvas`: `light_rgba` skips unlit pixels.
- `pixfx-core`: `Painter::disc`, `circle`, `ring` and `light`; `OptValue::Color` and `Options::color_or` for colour
  options; `Options::num_in`, `math::js_sign`, `math::or_one`; `color::step_at` is public.
- `pixfx-wasm`: an extra option may be a hex colour string (`{ stone: "#7a6a58" }`), read as `OptValue::Color`.
- `pixfx-effects`: the number options that size loops (`water-jet` and `laser-bolt` `length`, `small-explosion`
  `size`, `poison-cloud` `spread`) are held to a documented maximum so a huge value cannot hang an effect.
- `pixfx-core`: `Effect::canvas(&Options) -> Canvas` is the canvas an instance draws on (`Meta::canvas` unless the
  effect sizes it from its options); `render_frames`, `Rendered.origin`, the wasm `Player` (`Player::canvas`) and
  `Fx`, the canvas `Stage` and `pixfx-gpui` size and place every instance by it.
- `pixfx-effects`: `weather/fireflies` and `weather/snow` take `width` and `height` options (whole pixels, 1 to
  2048) that size their canvas, as densely filled as their own 320×180, which is still the default and renders
  as pixelfx's. `weather::lib` helpers take a `Canvas`; `sized`, `count` and `MAX_SIDE` are new.
- `pixfx-export`: a sheet's `pivot` is the origin on the canvas the options give (`Rendered.origin + margin`).
- `pixfx-cli`: `read_reference` takes the origin the reference was rendered at.
- `site/`: one fireflies instance the size of the landing scene lights it, so glows no longer stop in straight
  lines where 320-pixel tiles met.

## 0.1.0

- `pixfx-core`: frame with colour and additive light, painter, `Effect`/`Instance`, `render_frames`,
  `composite`, `frame_hash`.
- `pixfx-effects`: `fire/candle`, `fire/campfire`, `fire/brazier`, matching pixelfx with 0 differing pixels.
  Frame-hash determinism test runs natively and in wasm.
- `pixfx-cli`: `pixfx parity`, `pixfx sheet`, `pixfx render --gif`.
- `pixfx-export`: sprite sheets (composite, colour, light PNG), beanfx-layout JSON, GIF.
- `pixfx-wasm`: JS bindings (`effects`, `meta`, `Fx`) and a browser demo.
- `pixfx-canvas`: a Canvas 2D renderer with its own loop (`Stage`), `update` + `drawLayer("back"|"front")` for hosts that draw sprites between an effect's layers, and a browser demo.
- `pixfx-gpui` and `examples/gpui-web`: live playback inside gpui on the web.
- `pixfx-phaser`: Phaser 4 scene plugin playing effects as per-effect images or in scene-wide layers, with a demo.
- `parity/LEDGER.md`: record of approved parity misses (empty).
