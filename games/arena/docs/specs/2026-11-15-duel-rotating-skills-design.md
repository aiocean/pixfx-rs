# The duel's rotating skill bar, statuses and counters

## Intent

The landing duel plays the same four skills every round. The user wants more skills and more chance in a fight: a
bar of at most 3 skills that turns over as they are spent, each skill with its own energy and damage, a random new
one taking a spent skill's place, and combat that reads deeper (elements, counters). The research behind the
mechanics is `docs/superpowers/research/2026-11-15-survivors-combat-research.md`.

## Decided by the user

- A slot skill has **its own energy**, rolled when drawn; each cast drains it by the skill's cost (qi is still
  spent). When it cannot pay another cast, the slot empties, counts down, and **draws a new random skill**.
- **A pool per school**, each grown with new spells. **Fixed**: slash, dive, blink and the ultimate (its own fourth
  place, charged by rage). Only the 3 slots rotate.
- The rotating bar is built in `site/assets/duel.js` first; the move of the rules to Rust carries it over later.
- From the research, all eight: **stacking statuses, anti-lock control, energy roll sets power, draw rules, block by
  hit size, ward as hit charges, less damage variance, evolution and cleanse** (the last as the final phase).

Every number below is the builder's call unless the line says otherwise; all are constants at the top of `duel.js`.

## 1. The bar

- A slot holds one skill: `{ spell, casts, left, power, evolved }`, or `{ empty: t }` counting down.
- On a draw, `casts` is rolled 1 to 4 (equal odds). The skill's energy is `casts × cost`; each cast drains `cost`.
  A skill drawn with fewer casts hits harder: **power ×1.4 / ×1.2 / ×1.1 / ×1.0** for 1 / 2 / 3 / 4 casts.
- Casting uses the skill's own cooldown, as today. At 0 casts left the slot empties and counts down `3 + 2 × roll`
  seconds.
- **Draw rules**: never a skill already on the bar; the round's first draw is the school's signature kind (Thunder a
  blast, Frost a zone); while the foe carries a status, one draw in three comes from the skills that pay it off.
  Otherwise equal odds. A round starts with 3 fresh draws. All rolls use the duel's seeded dice.
- A skill with no qi cost never enters a pool.

## 2. New spells (effects already in the engine, played the way an existing kind plays)

| School | Spell | Kind | Effect | Role |
|---|---|---|---|---|
| Thunder | spark | bolt | projectiles/laser-bolt | cheap, fast, light; 1 shock stack |
| Thunder | burst | blast | explosions/plasma-explosion | heavy at arm's length; pays off shock |
| Thunder | quake | zone | explosions/small-explosion | quick blast under the foe, grounded |
| Thunder | hex | ward | arcane/hex-shield | hit charges; cleanses (phase 4) |
| Frost | shard | bolt | projectiles/laser-bolt | cheap, fast, light; 1 chill stack |
| Frost | jet | blast | water/water-jet | a cold jet at middle reach; 1 chill stack |
| Frost | mist | void | nature/poison-cloud | a cloud under the foe; a chill stack a tick |

Pools: Thunder bolt, palm, step, seal, orb, bell, heal, spark, burst, quake, hex (11). Frost bolt, lotus, shatter,
icicle, collapse, frostfire, void, mirror, ward, shard, jet, mist (12). Each spell gets its own icon (a new generated
sheet through `site/art/skill-icons.sh`, indexed by spell name).

## 3. Statuses that stack (Halls of Torment)

- **Chill**: 0 to 3 stacks, each hit that chills adds one and refreshes 2.5 s; each stack slows walking ×0.85. The
  third stack freezes the target (through section 4) and clears the stacks. Sources: shard, jet, mist, frostfire.
- **Shock**: 0 to 5 stacks, a shocking hit adds one (no chance roll). A shocked fighter twitches: every
  `0.6 × 0.854^(stacks−1)` s it spends a stack, takes 3% of the caster's attack (no defence) and is held 0.12 s.
  Sources: bolt, spark, seal, palm, orb.
- **Payoffs**: Thunder's palm and burst deal +12% per shock stack on the foe and spend them; Frost's shatter and
  lotus deal +15% per chill stack and spend them.
- Burn, defence-down and silence stay as they are.

## 4. Control that gets harder to repeat (Vampire Survivors, Valhalla)

- Hard control is freeze, the shock hold and the bell's stun. Each one the target takes adds 1 **grit**; grit falls
  0.4 a second.
- A control lasts `duration / (1 + grit)`. At grit 2 or more, a freeze becomes one chill stack instead.

## 5. Guard, ward and damage

- **Guard by hit size** (Halls of Torment): a guarded blow is turned aside with chance `min(0.5r, 0.5√r)`, capped at
  1, where `r = GUARD / power` and `GUARD = 20`: slash (power 5) always, dive (11) about 2 in 3, burst (24) about 2 in
  5. A failed block lets the whole blow through; the qi cost is paid either way.
- **Ward as hit charges** (Valhalla): Frost's ward and Thunder's hex hold `1 + floor(3 × roll)` charges for 6 s; each
  erases one whole blow (not burn, shock or field ticks).
- **Less variance**: spread ±5% → ±1.5%. Reductions of damage taken (the guard's chip) multiply and stop at 80%: chip
  becomes 20%.

## 6. Evolution and cleanse (final phase)

- **Evolution** (Vampire Survivors, Valhalla): pairs per school (Thunder bolt–seal, palm–step, quake–burst; Frost
  bolt–icicle, shatter–mist, lotus–jet). A spent skill whose partner is on the bar refills at once as its evolved
  form: power ×1.5, one more stack of what it inflicts, a bright frame on its icon. Its countdown is skipped.
- **Cleanse**: Thunder's heal and hex, Frost's ward clear burn, silence, defence-down and chill and shock stacks.

## The brain

Unchanged in what it decides with. Each brain is given its school's whole pool once (`Mind::new` and `meet`); a
spell not on the bar is not ready. Spells are read by kind. The sense carries the foe's frozen and shocked hold
as now; stacks reach it only through what they cause. A spell's damage in its `Skill` is its power on the bar.

## Tests (site/tests)

- The bar: never more than 3 slot skills; basics and the ultimate always there; a spent skill empties its slot and
  it refills after its countdown with a skill not on the bar; casts 1 to 4 and their power; the same seed plays the
  same bar; the first draw is the signature kind.
- Every pool spell is cast in some seed's fight and its effect is in the engine.
- Chill: 3 stacks freeze; shock: stacks tick and spend; payoffs spend stacks and hit harder.
- Grit: a second freeze at once is shorter; at grit 2 a freeze is a chill.
- Guard: slash always blocked, a heavy blow sometimes not; ward charges erase whole blows; spread within ±1.5%.
- Phase 4: a spent skill with its partner on the bar refills evolved; cleanse clears the statuses.
- Fights still end by knockout and both schools win rounds; the brain fights with any bar.

## Not in scope

The Rust port and lookahead (their own spec, next). Balance beyond "fights end and both schools win rounds".
