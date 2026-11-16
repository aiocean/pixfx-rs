# Rules

How a fight works, for a bot author. The rules are `crates/pixfx-duel`; every number lives in
`crates/pixfx-duel/src/data.rs` and is pinned by `tests/fixtures/js_numbers.json`, so this doc names numbers only
where they explain a mechanic. When a number here and `data.rs` disagree, `data.rs` is right.

## A fight

- Two fighters on a flat arena with walls: Thunder in the left corner (0), Frost in the right (1). They walk, jump,
  duck, guard, fly and meditate, and cast skills.
- A round ends when one fighter's hp reaches 0. A round that passes the time limit is **cut**: in training and in the
  ladder a cut round counts as lost for the learner, so stalling never pays.
- The simulation runs at 60 ticks a second (`duel::TICK`). A seed decides every roll, so the same seed and the same
  two bots give the same fight on every machine.

## The two schools

| | Thunder | Frost |
|---|---|---|
| Stats | less hp and attack (2500 hp, 345 atk) | more hp and attack (3300 hp, 520 atk) |
| Traits | `fury`: each blow it lands raises its attack, stacking for the round; `unyielding`: once a round, a killing blow leaves it on its last sliver of life | `attuned`: every second paid spell it casts earns a rage point |
| Basics (always in reach) | slash, dive, blink | slash, dive, blink |
| Pool (the bar draws from it) | bolt, palm, step, seal, orb, bell, heal, spark, burst, quake, hex | lotus, shatter, icicle, frostfire, collapse, shard, jet, flurry, void, mirror, ward |
| Ultimate (full rage) | tribulation: three thunderclaps fall one after another where the foe will be | prison: a great collapse drags the foe in from far off and freezes it |

Frost's bigger stats are balanced by Thunder's faster, more varied kit. With both sides played by the same trained
AI, Thunder wins 49.1% of the mirror over 150 seeds (the target is 45-55%).

## Skills

Each skill has a kind (melee, dive, dash, blast, bolt, seeker, area, field, heal, ward, barrier, blink), a qi cost, a
cooldown, a reach, a damage, and two timings:

- **windup**: from the cast until the blow comes out;
- **recover**: after the blow, before the fighter can act again.

Areas and blinks can be aimed: at the foe, ahead of it or short of it. Everything else aims at the foe.

## The bar

A fighter does not hold its whole pool. Each fight draws 8 of its school's 11 pool skills (`POOL_SIZE`), and three
bar slots show one skill each. A cast empties its slot, which refills from the pool after 1.5 to 2.5 s. A skill left
ready and uncast for 6 s leaves for another draw (`STALE`). Some pairs evolve when cast one after the other (Thunder:
bolt + seal, palm + step, quake + burst; Frost: shard + icicle, shatter + flurry, lotus + jet), and a lingering skill
grows stacks.

So a bot has seven castable slots: the three basics, the three bar slots and the ultimate.

## Qi and rage

- **Qi** pays for casts and for flying. It refills slowly on its own and fast while meditating (`QI_REGEN`,
  `MEDITATE_REGEN`), but a meditating fighter is open.
- **Rage** is a small whole-number pool: a point for each blow landed. When it is full, the ultimate is castable.

## Statuses

What a hit can leave behind (`data.rs` and `status.rs`): **chill** (stacks that slow walking; the last one freezes),
**freeze**, **shock** (stacks; a shocked fighter twitches, held and hurt), **burn** (a share of the caster's attack
every tick, ignoring defence), **stun**, **silence**, **seal**, **defence down**, a **pull**, and rage stolen. A bot
sees, for each of its slots, what that skill inflicts (`docs/bot.md`).

## Defence

- **Guard** holds a blow, at a little chip damage. Casting drops the guard.
- **Barriers** (bell, mirror) turn blows aside for a moment; **wards** (ward, hex) erase one; **dodge** is a stat roll.
- Two blades meeting **clash**.

## Frame data

The design is `docs/specs/2026-11-16-duel-frames-design.md`; the numbers are `data::FRAMES`.

- **Counter:** a blow on a fighter in its windup does 1.2x and staggers 0.10 s longer.
- **Punish:** a blow on a fighter in its recover does 1.2x and staggers 0.20 s longer.
- **Unsafe on block:** dive, step, palm, burst and lotus recover 0.15 s longer when their blow is guarded.
- **Ultimate opening:** an ultimate cannot be hit in the first 0.3 s of its windup; a blow passes through as a dodge.

A slow recover is an opening, and a bot sees every phase and its seconds left (`docs/bot.md`), so it can learn to
punish.

## Styles

Each fighter is drawn one of three styles, aggressive, balanced or defensive (`style.rs`), on dice of their own, so
the style never moves a fight's rolls. The hand-written brain fights differently in each. A bot reads its style in
its tag (`docs/bot.md`) and may play it or ignore it; the shipped AI was trained in all three. Every bot reacts and
thinks at the same speed whatever its style (`docs/bot.md`, "When it decides").
