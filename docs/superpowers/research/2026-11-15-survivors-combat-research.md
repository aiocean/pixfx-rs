# Skills, damage, elements and counters in three survivors-likes, read for the duel

Design notes only (two-room rule): mechanics and formulas in our own words, from the RE notes of Halls of Torment
(HoT), Vampire Survivors browser alpha (VS) and Valhalla Survival (VhS) under `learnSecurity/targets/<game>/notes/`.
No code, data tables or assets come across. Confidence: HIGH = the notes cite code or data.

## What the three agree on

- **No element wheel.** None has an element-beats-element table. Elements act only through statuses (HoT), flat
  resist stats capped at 75% (VhS), or tint (VS: cosmetic hit types). HIGH in all three. With two schools a wheel
  means nothing anyway: our elements should be statuses with payoffs, not a matrix.
- **No cleanse.** Statuses are prevented (resist, shorter duration) or outlast, never removed. HIGH. In a 1v1, where
  a status-heavy bar can decide a round, a cleanse is a real answer that hordes never needed.
- **Little or no variance in damage.** HoT has none, VhS ±1%, VS none; chance lives in crit, block, status and the
  draw. Ours is ±5% (`duel.js:30`).
- **Control is bounded.** VhS staggers at most once per second and caps status resist at 80%; VS freezes only through
  a resistance check; HoT makes bosses immune to stun.

## Skills and how they are offered

| | HoT | VS | VhS |
|---|---|---|---|
| Offer | 3 cards, ≥1 upgrades an owned ability | 3 cards (4th from luck), owned items first, rest by rarity weight, no duplicates | active 60% / passive 40%, first card a class active |
| Growth | 10-tier talent tree, branches lock each other | 8 levels, then evolution: max weapon + paired passive | 5 levels, evolution at 5 with a partner skill |
| Timing | auto-fire on a timer | auto-fire on a timer, volley spacing | fixed cooldowns, 0.4 s floor |
| Chance | uniform picks; reroll, banish, lock | luck bends card count, chest tier, crit | rerolls earned, no banish |

## Damage

- **HoT, taken:** i-frames negate hits no bigger than the one that started them; a block roll whose chance falls with
  hit size, `min(0.5r, 0.5√r)` with `r = block / hit`; then a defence curve mixing a linear and a hyperbolic term.
  Crit chance over 100% gives extra crits. Percent bonuses add into one multiplier.
- **VS:** damage = might × weapon power; flat armour (max 8, min 1 damage); 120 ms hit blink; a hit's type sets the
  blink and knockback length.
- **VhS:** attack and defence compare as a level gap, multiplier `1 + 2Δ/(20+Δ)` above parity (×2 at +20, ×3 max)
  and `1 + Δ/(10−Δ)` below; the defender's reductions only cancel the attacker's increases; stacked reductions
  multiply and cap at 80%; ratings (crit, dodge, status resist) saturate as `A + B·x/(C+x)`.

## Statuses

- **HoT:** stacks on one copy per target. Burn caps at 20 and bursts when applied at the cap; electrify spends a stack
  per tick and ticks faster with more; frost stores stacks and detonates; decay lowers defence per stack forever;
  fragile adds 5% damage taken per stack and halves when it ends. Every full 100% of chance is a sure stack, the rest
  rolls. Combos are explicit: bonus damage per stack on the target, procs gated by element.
- **VS:** freeze has a power that must beat the target's resistance; it refreshes, never stacks.
- **VhS:** stun, root, freeze (a slow), silence, blind, poison; proc chance per hit mostly 1.0; duration cut by flat
  then percent (≤ 50%).

## Taken for the duel (proposed)

| Idea | From | In the duel |
|---|---|---|
| Statuses stack to a payoff | HoT | chill stacks to 3, the third freezes; shock stacks spend one per tick, faster with more |
| A payoff per stack on the foe | HoT | one skill per school hits harder per status stack the foe carries |
| Control gets harder to repeat | VS, VhS | each freeze or stun raises the target's resistance for a few seconds; a weaker repeat becomes a chill |
| Energy roll sets power | VhS (ramp), ours | a skill drawn with 1 cast hits harder than one drawn with 4 |
| Draw rules | VS, VhS, HoT | no duplicates on the bar; the round's first draw is the school's signature kind; one draw in three pays off a status the foe carries |
| Block by hit size | HoT | guard turns aside light blows more than heavy ones |
| Ward as hit charges | VhS | Frost's ward erases 1 to 3 whole blows instead of a share of life |
| Less damage variance | all three | spread ±5% → ±1.5%; stacked damage reductions multiply and cap at 80% |
| Evolution on the bar | VS, VhS | a spent skill whose partner is on the bar refills as its evolved form (later) |
| Cleanse | gap in all three | a heal or ward that clears burn, silence and defence-down (later) |

## Left out

Horde tools: screen kills, chain and pierce falloff, splitting damage, knockback tiers, contact damage. VS's flat
armour (fits many small hits, not a duel's few big ones). VhS's defence-to-level conversion and rating stats (two
fixed schools need no rating economy).
