# The duel's bar: readable on screen, fair fights, more chance

## Intent

The user saw the rotating bar (`2026-11-15-duel-rotating-skills-design.md`) and said it is not good yet: it looks wrong
on screen, the fights feel wrong, there is too little chance, and the four deferred review findings should be fixed.
They chose all four kinds of chance: traits, rarity, lucky casts and a random pool per fight.

Every number below is the builder's call; all are constants at the top of `site/assets/duel.js`.

## 1. On screen (`drawSkills` in `site/assets/duel-art.js`)

What is wrong today, seen at 3× zoom: most icons sit at 30% almost all fight (any slot not castable this instant is
faded), the cast pips are 2×1 specks on the gold frame while the big slot numbers 1 2 3 read as the casts, an empty
slot is a flat olive block with a jagged corner, and the ultimate's rage is a muddy wash over its icon.

- **Casts left**: a digit on the tab where the slot number is now, in the school's glow. Slot numbers go: in a duel
  the AI plays, they mean nothing.
- **Not castable**: a cooldown keeps its dark clockwise sweep; short of qi or silenced, the icon shows at 60%, not 30%.
- **Empty slot**: dark field, the seconds left as a digit in the middle, and the frame lighting up clockwise as the
  wait runs out.
- **Ultimate**: the icon stays clean; the frame fills with the school's glow from the bottom as rage builds, and
  blazes once full.
- **Rarity** colours the frame: common bronze, rare the school's glow, epic white and pulsing.
- **Trait**: a 5×5 glyph in the icon's top-right corner, one per trait.
- **Lucky cast**: the slot flashes gold, and a word floats over the fighter ("Double!", "Free!", "Again!").

## 2. Chance

- **Random pool per fight**: each fight, each school plays a seeded 8 of its pool, always with at least one skill of
  its signature kind and one payoff skill.
- **Rarity** on each draw: common 70% (power ×1.0), rare 22% (×1.15), epic 8% (×1.3). Multiplies with the casts power.
- **Traits** on each draw: a common skill has a 40% chance of one, a rare has one, an epic two (different). Only traits
  that do something to that skill are rolled:
  - Swift: cooldown ×0.6.
  - Heavy: power ×1.3 (damaging skills).
  - Piercing: cannot be blocked by a guard (damaging skills).
  - Lingering: one more stack (skills that chill or shock); adds to evolution's.
  - Cheap: qi cost ×0.5.
- **Lucky casts**: each cast of a bar skill has an 8% chance of one: Double (the blow at ×2), Free (its qi back) or
  Again (its cooldown ends at once).
- Evolution keeps the skill's rarity and traits. All rolls use the duel's seeded dice.

## 3. Fair fights

Thunder wins 49 rounds to Frost's 12 over seeds 1–20 (it was 51 to 11 before the bar). Thunder deals 159k damage to
Frost's 110k: 481 slashes to 258, 189 dives to 85, and Frost's pool spells land little. Tune `STATS`, costs and damage
until, with all of the above in, each school wins 40–60% of rounds over seeds 1–40. The test asserts at least 35%
each, so dice changes elsewhere do not break it.

## 4. The four review findings

- **The AI reads each skill's real power.** `pixfx-mind`'s `Sense` gains `power: Vec<f64>` (one per skill, 1 when
  absent), and the AI scales a skill's damage by it when it values a cast. The duel passes rarity × traits × casts power
  × evolution for bar skills, 1 for the rest.
- **The orb shocks**, as the first spec lists.
- **Frost's bolt leaves Frost's pool**: Frost already has shard, its own chilling bolt. Evolution pair bolt–icicle
  becomes shard–icicle. Thunder keeps bolt.
- **Guard chip** takes the blow's real damage (`damageOf`) × CHIP, not its raw power.

## Tests (site/tests, and pixfx-mind's own)

- Pool: 8 per fight, with the signature kind and a payoff; same seed, same pool.
- Rarity and traits: odds within bounds over many draws; only fitting traits; each trait does what it says.
- Lucky casts: about 8%; each of the three does what it says.
- Fair fights: each school ≥35% of rounds over seeds 1–40; fights end in knockouts.
- Findings: orb shocks; Frost's pool has no bolt; chip follows defence; pixfx-mind values a stronger skill higher.
- Screen: a browser check of the bar at 3× zoom (casts digit, empty countdown, rarity frames, trait glyphs, lucky flash).

## Not in scope

The Rust port of the rules and lookahead.
