# pixfx-mind: a fighter's brain as a state machine

## Intent

The landing duel's fighters decide from scratch every 0.12 s (`mind()` in `site/assets/duel.js`): they score every
action and take the best. They read the foe's wind-ups and openings, but they hold no intention from one thought to
the next, so a fight has no arc: no "pressing in", no "keeping away until the qi is back", no "going for the kill".

pixfx-mind gives each fighter a brain that is a state machine. Its state (a posture) lasts, it changes because of
what the fighter perceives (its own state, the skills it holds, the foe's state, the arena), and the posture
decides how the fighter weighs every action it could take. The fight becomes smarter and more readable: a player
watching can tell what each fighter is trying to do.

## Goals

- Each fighter is a state machine with lasting postures and explicit transitions.
- The inputs are many and all count: own hp, qi, rage, statuses, the skills it holds and which are ready, the foe's
  state (what it is doing, how long it is open, its posture as the brain reads it), and the arena (walls, ceiling,
  threats in flight, own shots alive).
- Skills are data. The brain knows skill *kinds* (melee, dash, bolt, zone...), never the duel's spell names, so any
  set of skills (another school, another game) works without changing the brain.
- The brain is pure Rust, deterministic for a seed, tested by behaviour.

## Non-goals

- Porting the duel's rules (damage, shots, cooldowns, statuses) to Rust. They stay in `duel.js`.
- `bout.js` (the scripted takes on effect pages): unchanged.
- Learning across rounds or games beyond the per-round "what lands" read the JS mind already keeps.

## Anti-goals

- The brain names Thunder, Frost, or a spell id: then it is the duel's code in a crate, not a brain.
- `duel.js` keeps deciding: after the switch, JS only describes what the fighter sees and carries out the choice.
- A brain that is more predictable than today's: postures with no noise and no foe read would be.

## Approaches

| Approach | Trade-off | Picked |
|---|---|---|
| Brain crate only; `duel.js` hands it a snapshot each thought through wasm | Fastest to land; the snapshot must stay in step with the JS rules | **Yes** (user's pick) |
| Port the whole duel (rules + brain) to a Rust crate | Brain reads the real state, gpui games get the duel; ~1150 lines of rules to port first | No |
| Restructure the mind as a state machine in JS | No crate; contradicts the request | No |

The snapshot types are written so the rules could later move to Rust and feed the same structs.

## Research: what was taken and what was left

| Source | Taken | Left, and why |
|---|---|---|
| Unreal StateTree utility selectors; Infinite Axis Utility System (Lewis, Game AI Pro 3 ch. 13) | postures chosen by scored considerations with curves, veto on zero | a hand-wired transition graph: it grows as postures squared |
| Dragon Age: Inquisition (Game AI Pro 3 ch. 31) | score bands so reactions win; a record of why each choice was made | |
| Dill, dual utility (Game AI Pro 2 ch. 3) | rank, then weighted draw | argmax with noise: easy to read and exploit |
| Rabin, reaction time (Game AI Pro 2 ch. 5) | the brain sees the foe ~0.2 s late | instant reactions: players read them as cheating (Smash level-9 CPU) |
| N-grams (Game AI Pro ch. 48); Killer Instinct Shadow AI (GDC 2016) | decaying n = 1-2 habit counts, foe-relative features, lowering punished actions, a deliberate off-pick | Shadow AI's case library: it needs thousands of recorded human patterns we do not have |
| Sirlin, Playing to Win ch. 7 | three yomi levels, drawn by recent payoff | |
| FightingICE winners (Tang et al., arXiv 2003.13949) | hand structure plus light prediction wins | MCTS and rollouts: the rules stay in JS, so there is no simulator in Rust, and 8 thoughts a second on a phone has no budget for search |
| F.E.A.R. GOAP, Horizon HTN | | a plan is stale after one blow in a duel decided 8 times a second |
| Behaviour trees (bonsai-bt, beehave, Unity Behavior) | | good for scripted sequences, poor at weighing a situation; Unity has scaled its package back |
| big-brain, seldom_state | the Scorer / Thinker / Picker split as a pattern | as dependencies: the crate depends on `libm` only |

## Where it lives

`crates/pixfx-mind`, Engine layer: pure, no clock, no I/O, randomness from its own seed. Depends on `libm` only (plus
`serde`/`tsify` behind `ts`, as the other engine crates). `pixfx-wasm` exposes it behind a `mind` feature (in its
default features, like `fighter`). Not part of `pixfx-fighter`: a brain is not a body, and a game with no drawn
fighters (a top-down game's enemies) could use it.

## The brain

### Inputs, one `Sense` per thought

What the fighter could perceive this tick.

- `me`: x, height off the ground, facing, hp (0-100), qi (0-100), rage and rage needed, flying and hover height,
  statuses (frozen, shocked, silenced, sealed, chilled), barrier up, ward shield, own shots alive, whether it can act.
- `foe`: x, height, flying, hp, what it is doing (`Doing`: idle, moving with its direction, guarding, meditating,
  winding up / recovering a skill of a given kind with time left and reach), reeling, `out_for` and `open_in` (from
  its `Body`), statuses, barrier up.
- `arena`: left, right, top.
- `threats`: every shot of the foe's that can reach me, with its age, turned by the game into a shape the brain reasons about
  without knowing the spell: a `Projectile` (x, height, speed), an `Area` (x, radius, lands in, hits only the ground),
  a `Seeker` (the orb: x, height, radius), a `Field` (a fire or vortex: x, reach), a `Barrage` (strikes that follow).
  Each with damage.
- `skills`: per skill the fighter holds, whether it is ready now (cooldown, qi, rage, silence, seal).
- `time`: the duel's clock, so the brain can age what it remembers.

The game passes the true state; the brain applies its own reaction delay (below).

### When it thinks (builder's call)

The brain owns its own clock: the game calls `think(sense, dt)` every tick and the brain decides whether this tick is
a thought (every ~0.12 s with seeded spread) and how long to take a breath after its own attack (`tempo`). The
`Sense` carries `me.can_act` (false while acting, reeling, squatting, ducking, frozen, shocked or out), and a tick
when it cannot act returns `None`. `hold(seconds)` keeps the brain quiet, for tests that freeze a fighter.

### Set once, `Mind::new(profile, body, skills, seed)`

- `body`: the fighter's sizes the brain reasons with (half width, chest, body height, duck height, walk speed, jump
  speed), passed as data from the game's `Body.spec`, so the crate does not depend on `pixfx-fighter` (builder's
  call).

- `skills`: each skill's handle (the game's name, returned as is), kind, cost, windup, recover, damage, reach
  (range, distance, radius, pull reach), speed, time to land for zones, whether it is an ultimate, and what it
  inflicts (freeze, shock, silence, defence down, pull, burn, chill, rage steal).
- `profile`: the school's temperament: aggression, tempo (breath after an attack), preferred range, range when dry
  of qi, preferred height, risk weight, lead weight, reaction time, per-posture scales. Today's `SCHOOLS` and `MIND` numbers move here as data.

### Postures (the states)

| Posture | Wants | Enters when (examples) |
|---|---|---|
| `Neutral` | its range and height, probing, footsies | default; after an exchange settles |
| `Pressure` | stay close, chain blades and blasts | foe low on qi, cornered, or recovering; me ahead and with close skills ready |
| `Zone` | keep the foe out at range with bolts, orbs, zones | ranged skills ready and qi high; foe pressing with a short reach |
| `Punish` | land the hardest blow that fits the foe's opening | foe open (`out_for`, recovery, frozen, shocked) longer than one of my skills needs |
| `Defend` | block, step out, barrier, counter | a foe blow or a threat is about to land |
| `Evade` | leave the danger: blink, fly, jump, walk away, out of a corner | an area or field I stand in; cornered under pressure |
| `Recover` | get qi back: distance, then meditate | qi too low for my skills; foe far or busy |
| `Finish` | commit: ultimate, heaviest blow, chase | foe hp low, or rage full |

How the brain moves between them (after Unreal's StateTree utility selectors and the Infinite Axis Utility System,
see Research):

- **Postures are chosen by score, not by a web of edges.** Each thought, every posture scores how much it fits now
  from its *considerations*: inputs normalised to 0-1 (my qi, foe's opening, distance against my range, wall
  distance, threat urgency, foe's read posture...) each passed through a response curve (linear, quadratic,
  logistic) and multiplied. A zero vetoes the posture (`Finish` with the foe at full life). Eight postures do not
  need 56 hand-written transitions.
- **Commitment.** The current posture gets a bonus that fades over its minimum time, and a new posture must beat it
  by a margin (hysteresis), so the brain does not flicker between two close postures.
- **Interrupts.** `Defend` and `Punish` score in a band above every other posture (Dragon Age: Inquisition's score
  bands), so a blow about to land or a real opening wins at once, whatever the commitment.
- **Data, not code.** Each posture is a const table: its considerations with their curves, its minimum time and its
  weights over action categories (approach, retreat, close attack, ranged attack, area attack, defend, evade, buff,
  meditate, wait). A school's profile can scale any of them. Choosing a posture is a pure function of the
  `Reading`, tested on its own.

### Seeing late

Reaction time moves into the brain (Rabin, Game AI Pro 2: about 0.2 s for a simple reaction). The brain keeps a
short ring of past senses and sees the foe as it was `profile.reaction` seconds ago; a threat counts only once it
is that old. Its own state it sees now. The game passes the true state and stops filtering.

### Reading the foe

Each thought the brain turns the delayed `Sense` into a `Reading`: distances, corner, who is ahead, my best skill per
category and its readiness, the foe's opening, the foe's punisher on me (today's `punisher`), and the **foe's
posture as I read it** (the same set, guessed from what the foe does: closing in with a blade ready → `Pressure`,
casting from range → `Zone`, meditating → `Recover`).

**Habits** (n-grams, Game AI Pro ch. 48; Killer Instinct's Shadow AI): the brain counts what the foe does next after
its last action, per foe posture (n = 1 and 2, counts that decay over a few seconds, a fixed small table). A
prediction is blended toward a prior by how much it has seen, so a thin read barely counts (a wrong opponent model
makes a bot worse). Predictions feed considerations: against a foe that blocks after my blade, areas and throws
score higher; against one that walks in after my bolt, a blade waits for it.

**Yomi** (Sirlin, Playing to Win ch. 7): per exchange the brain plays at one of three levels: answer what the foe
is doing (0), answer what it predicts the foe will do (1), or answer the foe's usual answer to its own usual move
(2). The level is drawn from the seed with weights that move toward the level that paid off lately, so the brain
cannot be read by always doing the best thing.

### Choosing inside a posture

The candidate actions come from generators per skill kind and per threat shape, the logic of today's `attacks`,
`dangers`, `blows` and `positions` with spell names replaced by kinds. Each candidate's base score (expected damage
for its cost, less what a miss costs: today's Sirlin rules, with the "what lands" read per skill) is multiplied by the
posture's weight for its category. Then **rank and weight** (Dill, Game AI Pro 2 ch. 3): keep the candidates within a
share of the best, and draw one from the seed by weight. A small chance of the second-best keeps the brain honest.
**Punishment feedback**: `tell` reports when an attack was punished; that skill's weight falls for this foe and
recovers over time (Killer Instinct lowers an action that keeps getting punished).

### Why it chose (debug)

Every thought can return a record: the posture scores with each consideration, the top candidates and their scores,
the yomi level and the habit prediction. Plain data, replayable from the seed and the senses. `?mind` on the page
shows the posture and the winning reason.

### Output, one `Choice`

`Move(dir)`, `Jump`, `Duck(hold)`, `Guard(hold)`, `Fly(height)`, `Meditate`, `Wait`, or `Cast { skill, target }`
(the skill's handle and, for zones and blinks, an x). Plus the posture, so the page can show it (a debug caption).

## Wiring into the duel

- `duel.js` makes one `Mind` per fighter at `createDuel` from its school (skills from `SPELLS`, profile from
  `SCHOOLS`), resets it each round, and each thought builds the `Sense` (threats from `duel.shots`), calls `think`,
  and carries out the `Choice` as it does today. `tell` reports each attack's outcome.
- `dangers`, `blows`, `attacks`, `positions`, `punisher` and the `MIND`/`PRIOR` constants leave `duel.js`. They move,
  unchanged, to `site/tests/mind-baseline.js`, the yardstick the match test plays against; the page never loads it.
- `createDuel` takes an optional `minds` (one per fighter: anything with `think`, `tell`, `reset`), so a test can seat
  the baseline in one corner. The page passes none and gets the Rust brain for both.
- The posture shows as a caption over each fighter only with `?mind` in the page's URL (debug, for tuning).
- The JS duel's rng is no longer drawn by the mind; the brain draws from its own seed (the duel seed plus the
  fighter's index), so "a seed replays the same fight" still holds.

## Tests

- `pixfx-mind`: transition tests (each posture's entry and exit on a hand-made `Reading`), choice tests from a
  `Sense` ("foe recovering a slash in reach, my slash ready → `Punish`, casts slash"; "standing in an area that lands
  in 0.3 s → `Evade`"; "qi 10, foe far → `Recover`, meditates"; "a bolt at chest height → guard, duck or jump"), a
  skill set with no names of the duel's, determinism (same seed, same senses → same choices).
- `site/tests/duel.test.js` stays green: seed replay, knockouts, slot cooldowns, skill bar.
- **The match** (`site/tests/mind-match.test.js`), the proof of "smarter": the new brain against the baseline over a
  fixed set of seeds, each seed played twice with the brains swapped between Thunder and Frost so the schools'
  balance cancels out. It passes when the new brain wins at least 60% of the rounds; the test prints the win rate,
  the average round length and how often each posture was entered.
- `pixfx-mind`: posture choice tests on a `Reading` per posture; commitment holds a posture against a slightly
  better one and an interrupt breaks it; the habit table predicts a repeated foe pattern; the delayed view shows a
  foe wind-up only after the reaction time.
- On the page: watch several rounds of the landing duel with `?mind`, and see both fighters move through more than
  three postures in a round and rounds end by knockout.

## Left to the builder

- The exact numbers of each posture's weights and thresholds (tuned by watching the page).
- The size of the foe memory window.
- The wasm `Mind`'s JS shape follows `pixfx-wasm/src/fighter.rs` (strict parse at the boundary, tsify types).
