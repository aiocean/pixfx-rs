# Rules

How a fight works, for a bot author. The rules are `crates/pixfx-duel`; every number lives in
`crates/pixfx-duel/src/data.rs` and is pinned by `tests/fixtures/js_numbers.json`, so this doc names numbers only
where they explain a mechanic. When a number here and `data.rs` disagree, `data.rs` is right.

## A fight

- Two fighters on a flat arena with walls: Thunder in the left corner (0), Frost in the right (1). They walk, jump,
  duck, guard, fly and meditate, and cast skills.
- A round ends when one fighter's hp reaches 0. A round that passes the time limit is **cut**. In training a cut
  round counts as lost for the learner, so stalling never pays. In the ladder a cut ends that fight: it shows in the
  `cut` column, and its unplayed rounds count neither way.
- The simulation runs at 60 ticks a second (`duel::TICK`). A seed decides every roll, so the same seed and the same
  two bots give the same fight on every machine.

## The two schools

| | Thunder | Frost |
|---|---|---|
| Stats | less hp and attack (2500 hp, 345 atk) | more hp and attack (3300 hp, 520 atk) |
| Traits | `fury`: each blow it lands raises its attack 4%, up to 5 stacks for the round; `unyielding`: once a round, a killing blow leaves it on its last sliver of life | `attuned`: every second paid spell it casts earns a rage point |
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

Areas and blinks can be aimed (`docs/bot.md` gives where each aim lands). Everything else aims at the foe.

## The bar

A fighter does not hold its whole pool. Each fight draws 8 of its school's 11 pool skills (`POOL_SIZE`), and three
bar slots show one skill each. A cast empties its slot, which refills from the pool after 1.5 to 2.5 s. A skill off
cooldown and uncast for 6 s leaves for another draw (`STALE`), whether or not there is qi to pay for it.

- **Tiers and traits.** Each draw rolls a tier (common, rare, epic; legendary only through duplicates) and traits
  (swift, heavy, piercing, lingering, cheap) that change its power, cost and timings (`data.rs`, `bar.rs`).
- **Evolving pairs.** Casting one skill of a pair while its partner is on the bar refills that slot at once with the
  partner evolved (x1.5 power). Thunder: bolt + seal, palm + step, quake + burst. Frost: shard + icicle,
  shatter + flurry, lotus + jet.
- **Lucky casts.** A cast sometimes rolls lucky (8%): doubled, free, or cast again (`data.rs`).

So a bot has seven castable slots: the three basics, the three bar slots and the ultimate.

## Qi and rage

- **Qi** pays for casts and for flying. It refills slowly on its own and fast while meditating (`QI_REGEN`,
  `MEDITATE_REGEN`); meditating goes on until the fighter does something else.
- **Rage** is a small whole-number pool: a point for each blow landed (ultimates earn none, and Frost's spells earn
  none on hit: its blade does, and `attuned` pays a point every second paid spell). When it is full, the ultimate is
  castable, and casting it spends it. Rage starts empty each round.

## Statuses

What a hit can leave behind (`data.rs` and `status.rs`): **chill** (stacks that slow walking; the last one freezes),
**freeze**, **shock** (stacks; a shocked fighter twitches, held and hurt), **burn** (a share of the caster's attack
every tick, ignoring defence), **stun**, **silence**, **seal**, **defence down**, a **pull**, and rage stolen. A bot
sees, for each of its slots, what that skill inflicts (`docs/bot.md`).

**Grit:** hard controls shorten as a fighter's grit builds, and from grit 2 a freeze is only a chill stack
(`data.rs`).

## Defence

- **Guard** stops only melee, blasts, dashes, bolts and orbs, and only facing the blow; zones and fields fall past
  it. Each held blow costs 8 qi (below 8 qi the guard breaks) and lets 20% through as chip damage, and a heavier blow
  is likelier to get through (`act.rs`, `data.rs`). Casting drops the guard.
- **Barriers** (bell, mirror) turn every blow aside for a moment; **wards** (ward, hex) hold 1-3 charges for 6 s,
  each erasing one blow; **dodge** is a stat roll.
- Two blades meeting **clash**.

## Frame data

The design is `docs/specs/2026-11-16-duel-frames-design.md`; the numbers are `data::FRAMES`.

- **A blow during a windup breaks the cast**: the cast is lost.
- **Counter:** a blow on a fighter in its windup does 1.2x and staggers 0.10 s longer.
- **Punish:** a blow on a fighter in its recover does 1.2x and staggers 0.20 s longer.
- **Unsafe on block:** dive, step, palm, burst and lotus recover 0.15 s longer when their blow is guarded.
- **Ultimate opening:** an ultimate cannot be hit in the first 0.3 s of its windup; a blow passes through as a dodge.
- A field's strike (void) gives neither a counter nor a punish.

A slow recover is an opening, and a bot sees every phase and its seconds left (`docs/bot.md`), so it can learn to
punish.

## Styles

Each fighter is drawn one of three styles, aggressive, balanced or defensive (`style.rs`), on dice of their own, so
the style never moves a fight's rolls. The style is only a tag a bot reads (`docs/bot.md`): it changes no rule, no
reward and not the hand-written brain. A bot may play it or ignore it; the shipped AI was trained in all three.
