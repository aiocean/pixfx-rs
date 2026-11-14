# Plan 3: both sides of a hit — design

Date: 2026-11-13 · Status: direction chosen by owner 2026-11-13 ("both sides", "porting is only a part; after porting we
keep developing, so if what we do is development, just do it")

## Intent

A game built on pixfx should get a whole hit, not just its pixels: the attacker winds up, strikes and recovers on a
timing the game can read; the one struck reacts to the side it was hit from and to how many hits it has taken, moving
through states (hurt, swelling, bursting, remains); and the game answers on the exact frame of contact with hitstop,
shake, a ripple and a flash.

pixelfx (044e12a) has the attacker's timing (`src/core/act.js`, `actTiming`) and the melee and move effects. The
modified copy in game-factory (`game-factory/references/pixelfx`, branch `bloater-states`) has the target's side
(effect states with `meta.state {next, carry}` and the options `from`, `hits`, `dir`) and the game's side (effect cues,
`src/core/cues.js`), with the Bloater as its worked case (`demo/bloater.js`). Plan 3 brings these into pixfx and builds
on them: porting is the means, pixfx's own engine is the goal. Where the port and pixfx's existing pieces disagree,
pixfx's pieces win and the port adapts.

## Goals

1. **Cues.** An effect can say what the game should do and when: shake (trauma), hitstop (seconds), ripple, flash and
   sound, timed in the effect's own clock, delivered in world space as playback passes them.
2. **States.** An effect can be one state of a creature: which effect follows it when it ends and which options carry
   over. A small, pure state player runs a creature through its states and takes hits (`from`, `dir`, `hits`).
3. **One answer per game.** pixfx-fighter's `Feel` is the one place a game's hitstop and shake live. It takes effect
   cues as well as fight cues, and gains the ripple and the flash, so a game never runs two shakes or two freezes.
4. **Acting.** An attack's timing is data: wind-up, contact moments (each with its freeze, shake and push), dust,
   recovery. A performance clock plays it, holds still during a freeze and reports contacts. The attacker is drawn by
   pixfx-fighter's body and poses; there is one character in pixfx, not two.
5. **Effects.** The Bloater's five states (idle, hurt, swell, burst, remains), the melee category (12) and the moves
   category (12), each drawing the same pixels as its JS source.
6. **A place to see it.** A site page where a fighter performs a melee or move at a Bloater, the player can strike it
   from either side, and the Bloater goes hurt, hurt, then swell, burst, remains, with the game's answer on every
   contact.

## Non-goals

- Sound synthesis (`sfx.js`). Sound cues are carried as data and reach the host; making the sound is a later step.
- pixelfx's stick hero (`rig.js`, `actor.js`) and straw training dummy. The attacker is pixfx-fighter's body; the
  target is a state-driven creature or a fighter.
- The status category. pixfx-fighter already draws burning, frozen and shocked on a fighter; pixelfx's 13 status
  effects get their own plan that merges them with fighter `Marks` instead of adding a second set.
- The site's duel using pixelfx moves. That is the next step once this lands.

## Anti-goals

- Two shake or hitstop systems in one game.
- An effect whose picture differs from its JS source with no reason written down next to it.
- Game rules (how many hits kill a Bloater) inside the engine. The engine plays states and delivers cues; the game
  decides.

## Approaches

1. **Port pixelfx as it is.** Cues, states, acting with pixelfx's own stick hero and dummy, every piece checked against
   its JS. Cheapest to verify, but pixfx ends with two character rigs and two answers to a hit.
2. **Build on pixfx's own pieces (chosen).** Port what pixfx lacks (cues, states, effect pictures), check
   those against their JS, and route everything a game sees through pixfx-fighter's `Feel` and body. The attacker's
   timing drives the fighter's poses, so melee and moves are no longer compared with pixelfx as whole scenes, only as
   effect pictures. The owner chose development over a faithful port.
3. **Leave the attacker out.** Cues, states and the Bloater only. Smallest, but drops half of what was asked.

## Design

### Cues (`pixfx-core`, new module `cue`)

- `Cue` (enum): `Shake { trauma }`, `Hitstop { dur }`, `Ripple { x, y, r0, r1, dur, amp }`, `Flash { k, dur }`,
  `Sound { name, gain, pitch, len }`; `TimedCue { t, cue, pan }`.
- `Effect::cues(&self, opts: &Options) -> Vec<TimedCue>`, default empty; sorted by `t`, entries with `t < 0` dropped
  (game-factory `cuesOf`).
- `cues_between(list, t0, t1, meta, opts, screen_w) -> Vec<TimedCue>` as game-factory `cuesBetween`: a loop's cues
  repeat each period, x mirrored by flip and offset by the effect's position, `pan` from x across the screen width.
- `Instance` collects the cues its playback passes; the host drains them each tick.
- Naming: pixfx-fighter already has a `Cue` (fight events). The core type keeps the name `Cue`; inside pixfx-fighter
  the fight type is referred to by path where both meet. Builder may rename the fighter's to `Event` only if the
  fighter's JS bindings stay unchanged.

### States (`pixfx-core`, new module `state`)

- `Meta` gains `state: Option<StateMeta>`; `StateMeta { set, name, order, next: Option<&'static str>, carry: &'static
  [&'static str] }`.
- `from`, `hits`, `dir` are ordinary effect options (numbers), as in the JS.
- `Creature` (pure, no clock of its own): current effect id, its options and time; `enter(id, opts)`, `step(dt)` which
  follows `next` when a one-shot ends and copies the `carry` options, and the cues passed. Which state a hit leads to
  is the caller's rule (the Bloater's three-hit rule lives in the site page, as it lives in `demo/bloater.js`).

### The game's answer (`pixfx-fighter::Feel`)

- `Feel::take(&[TimedCue])`: shake adds trauma, hitstop extends the freeze, under the caps `Feel` already applies.
  Ripples and flash are held in `Feel` and age with real time (they keep going during a hitstop, as in the demo).
- `pixfx-core` gains the two screen passes the demo does (`demo/bloater.js:155-167`): `ripple` (bend the picture
  around growing rings) and `flash` (lift toward white by k). Pure functions over a `Frame`.
- Touching pixfx-fighter: check `ListAgents` for a session working in that crate first and coordinate if one is.

### Acting

Not in the engine until a game plays an attack effect. A pixelfx `act` has two halves: the hit timing tied to the
effect's own picture (when the blow lands, its freeze, shake, push and dust), which every game playing the effect
shares, and the caster's pose track, which is each game's own sprite. When a game needs it, the timing comes with the
effect it times, and the game draws its caster.

### Effects (`pixfx-effects`)

- `critters/bloater-{idle,hurt,swell,burst,remains}` and their shared `_bloater` drawing, with `cues` and `state`,
  checked against game-factory's copy.
- `melee/*` (12) and `moves/*` (12), pictures, checked against pixelfx 044e12a. A move that needs an
  effect not yet ported (a projectile, an explosion) gets it in the same task.

### Browser (`pixfx-canvas`) and the site

- Bindings: cues drained from a `Stage` effect as plain JS objects; `Creature`; `Feel.take`.
- A site page, "hit lab": a fighter with a chosen melee or move, a Bloater, strike buttons for each side; Bloater rules
  as in `demo/bloater.js` (two hurts, the third sets it swelling).

## Verification

- **Pictures:** each new effect ≤ 0.5% differing pixels against its JS source with the existing parity tool
  (`pixfx-cli parity <id> --pixelfx <path>`): game-factory's copy for the Bloater, `/tmp/fxref/pixelfx` (044e12a) for
  melee and moves.
- **Cues:** each Bloater state's cue list, for several `hits` and `from`, equals game-factory's `cuesOf` output
  (golden JSON written by a node script, as plan 2's goldens).
- **States:** a test drives a Bloater `Creature` through hurt, hurt, swell, burst, remains, checking `next`, the
  carried `hits` and mirrored `from`.
- **Acting:** a test checks contacts arrive at their `t`, the clock holds during `freeze`, and phases change at
  wind-up and recovery bounds; values equal pixelfx `actTiming` for every melee and move.
- **One answer:** a test feeds `Feel` fight cues and effect cues together and checks one trauma and one freeze, capped.
- **The page:** in Chrome, the hit lab runs at 60 fps with no console errors, and a recorded strike sequence shows the
  freeze on the contact frame and the five states in order.

## Left to the builder

- Ripple and flash drawing details beyond the demo's formulas.
- Which fighter poses map to each move family, within the existing `Pose` set; new poses only if none fits, added in
  pixfx-fighter's style.
- File split inside each new module.
