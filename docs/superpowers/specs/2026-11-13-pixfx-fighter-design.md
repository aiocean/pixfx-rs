# pixfx-fighter — design

Date: 2026-11-13 · Status: scope chosen by owner 2026-11-13 ("reusable parts", "all of it now, separately")

## Intent

The site's duel (`site/assets/duel*.js`) grew a fighting game's body: a drawn pixel fighter with ~20 poses, a body
that takes hits the way fighting games do (hitstun, launch, one juggle, knockdown, getting up, quick roll, ducking,
landing recovery), the screen's answer to a blow (hit-stop by weight, shake, damage numbers, dust, knockout slow
motion) and marks for what hangs on a fighter. `pixfx-fighter` moves those parts into the engine as Rust, so any
game built on pixfx can use them, and the site's duel becomes their first user.

What stays in the site: the duel's rules and AI (spells, damage, qi, rage, schools' spell lists, barriers,
mind/attacks/positions), the HUD (bars, damage-number text, qi streaks) and the scene.

## Relation to plan 3

Plan 3 (pixfx-rs-b8, after plan 2) ports pixelfx acting and game-factory cues. It builds on this crate's
hit-stop, shake and status pieces instead of adding its own (agreed with b8 2026-11-13).

## Crate

`crates/pixfx-fighter`, pure like `pixfx-core` (no I/O, no clock, no browser), depends only on `pixfx-core`'s
public API. Lints as the workspace; `#![forbid(unsafe_code)]`.

| Module | Holds | From |
|---|---|---|
| `body` | `Body`: position, velocity, facing, reaction state (`stun`, `down`, `wake`, `land`, `squat`, `duck`, `flash`, hit clock). `hit(Blow)`, `knock_out(dir)`, `step(dt, Drive, Arena, roll)` → events (`Land`, `Bump`), queries `reeling`, `floored`, `untouchable`, `out_for`, `open_in`, `pose(Doing)` | `duel.js` body/launch/thrownLands/downed/stand/outFor/openIn/poseOf |
| `pose` | `Pose` (idle, walk, jump, fly, land, crouch, duck, draw, slash, channel, cast, guard, meditate, hurt, frozen, shocked, launched, tumble, fall, down, roll, sit, kneel, win, …), `Gather` (summon, palm, lunge, prayer, cross) | `duel-art.js` |
| `look` | `Look`: the palette a fighter is drawn in; two stock looks, `thunder` and `frost` | `duel-art.js` LOOKS |
| `paint` | `draw(frame, look, &Appearance)`: the pose into a 64×64 sprite, outlined, turned in quarter turns, its gathering aura, a dodge's afterimage; into a `pixfx_core::Frame` | `duel-art.js` paint/blit/outline |
| `status` | `Marks` on a fighter and how they are drawn: burning (flames up the body, colour + light), frozen (ice), shocked (sparks, light), chill, defence down, silence, fury pips, shield, barrier dome | `duel-art.js` drawStatus/dome, new flames and sparks |
| `feel` | `Feel`: `react(&[Cue])`, `advance(dt) -> fight dt`, `shake(roll)`, `jolts()`, `dust()` + drawing dust into a frame, `numbers()`, `streaks()`, `dusk()` | `duel-feel.js` |

Randomness comes in from the caller (`roll: &mut dyn FnMut() -> f64`) so the duel keeps one seeded stream.

## Browser

`pixfx-canvas` binds the crate for JS: `Body` (its fields as properties; `tick`, `step`, `hit`, `knockOut`,
`settle`, `updatePose`, `Body.separate`, `Body.heavy`), `Feel` (takes the duel's event objects as they are) and
`Sprites` (fighters with their marks and status layers, dust and portraits, drawn colour then light like a Stage's
effects, between `begin()` and `end()`). The site's `duel.js` is given `Body` by `createDuel` and keeps only its
rules on the fighter object, the body's fields being accessors onto the wasm `Body`; `duel-art.js` keeps the HUD and
tells `Sprites` what each fighter is doing.

## Verification

- **Same fight**: the duel with the engine body plays the same fights as before, step for step: the 80-seed
  probe gives the same wins (104 to 118) and per-step traces of x, air, vx, vy, pose match.
- **Same pictures**: every pose of both looks, at several times, drawn by the old JS and by Rust in Chrome, match
  pixel for pixel on the sprite (blended aura and dome compared with a ±1 per channel tolerance).
- **Same feel**: the site's former feel tests run against the Rust `Feel` (`crates/pixfx-fighter/tests/feel.rs`).
- Chrome on the live site: 60 fps, no errors.
