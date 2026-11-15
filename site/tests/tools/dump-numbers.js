// Writes what the Rust port of the duel's rules (crates/pixfx-duel) is checked against: the JS rules' tables, dice
// and numbers, computed by duel.js itself. Run from the repo root:
//   bun site/tests/tools/dump-numbers.js > crates/pixfx-duel/tests/fixtures/js_numbers.json
// Deleted with the JS rules (plan 2026-11-15-duel-rules-in-rust, Task 8).
import * as duel from "../../assets/duel.js";

const { RARITY, TRAITS, SPELLS, SCHOOLS, STATS, LUCKY, PAYOFF, RAGE, STALE, POOL_SIZE, DECOR, TIERS } = duel;

const SEEDS = [0, 1, 2, 7, 2 ** 31 + 5, 2 ** 32 - 1];
const rng = Object.fromEntries(SEEDS.map((seed) => {
  const r = duel.random(seed);
  return [String(seed), Array.from({ length: 50 }, () => r())];
}));

const ts = Array.from({ length: 22 }, (_, i) => i * 0.37);
const out = {
  rng,
  orbOffset: ts.map((t) => [t, ...duel.orbOffset(t)]),
  blockChance: [0, 1, 5, 20, 33.3, 80].map((p) => [p, duel.blockChance(p)]),
  shockInterval: [1, 2, 3, 4, 5].map((n) => [n, duel.shockInterval(n)]),
  slotPower: TIERS.flatMap((rarity) => [[], ["heavy"]].flatMap((traits) => [false, true].map((evolved) => [rarity, traits, evolved, duel.slotPower(rarity, traits, evolved)]))),
  fits: Object.keys(TRAITS).flatMap((t) => Object.keys(SPELLS).map((name) => [t, name, duel.fits(t, name)])),
  spells: SPELLS,
  schools: SCHOOLS,
  stats: STATS,
  rarity: RARITY,
  traits: TRAITS,
  lucky: LUCKY,
  payoff: PAYOFF,
  constants: { RAGE, STALE, POOL_SIZE, DECOR, TIERS },
};
console.log(JSON.stringify(out, null, 1));
