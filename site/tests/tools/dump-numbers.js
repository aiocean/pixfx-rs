// Writes what the Rust port of the duel's rules (crates/pixfx-duel) is checked against: the JS rules' tables, dice
// and numbers, computed by duel.js itself. Run from the repo root:
//   bun site/tests/tools/dump-numbers.js > crates/pixfx-duel/tests/fixtures/js_numbers.json
// Deleted with the JS rules (plan 2026-11-15-duel-rules-in-rust, Task 8).
import * as duel from "../../assets/duel.js";
import { Body, Mind, effects } from "../engine.js";
import { baselineMind } from "../mind-baseline.js";

const { RARITY, TRAITS, SPELLS, SCHOOLS, STATS, LUCKY, PAYOFF, RAGE, STALE, POOL_SIZE, DECOR, TIERS } = duel;

const SEEDS = [0, 1, 2, 7, 2 ** 31 + 5, 2 ** 32 - 1];
const rng = Object.fromEntries(SEEDS.map((seed) => {
  const r = duel.random(seed);
  return [String(seed), Array.from({ length: 50 }, () => r())];
}));

// damageOf over both schools, both blow types, crit, fury, defence-down and the spread's ends
const DAMAGE = [
  ["thunder", 0, "frost", 0, 20, "magic", false, 0.5],
  ["thunder", 0, "frost", 0, 20, "physical", false, 0.5],
  ["frost", 0, "thunder", 0, 25, "magic", false, 0],
  ["frost", 0, "thunder", 0, 25, "magic", false, 0.999],
  ["thunder", 3, "frost", 0, 13, "magic", false, 0.25],
  ["thunder", 5, "frost", 2, 15, "physical", true, 0.75],
  ["frost", 0, "thunder", 4, 28, "magic", true, 0.1],
  ["frost", 0, "thunder", 0, 5, "physical", false, 0.6],
  ["thunder", 1, "frost", 0, 0.1, "magic", false, 0.3],
  ["thunder", 0, "thunder", 0, 14, "magic", false, 0.9],
  ["frost", 0, "frost", 1, 31, "magic", true, 0.45],
  ["thunder", 2, "frost", 3, 24 * 1.36, "physical", false, 0.05],
].map(([bs, fury, fs, defDown, power, type, crit, roll]) => [bs, fury, fs, defDown, power, type, crit, roll, duel.damageOf({ stats: STATS[bs], fury }, { stats: STATS[fs], defDown }, power, type, crit, roll)]);

// each fighter's pool and opening bar, as createDuel draws them with every effect in the registry
const registry = new Set(effects());
const arenas = Object.fromEntries([1, 2, 3, 4, 5, 6, 7, 8].map((seed) => {
  const d = duel.createDuel({ seed, left: 0, right: 220, ground: 150, has: (id) => registry.has(id), Body, Mind });
  return [String(seed), d.fighters.map((f) => ({ pool: f.pool, bar: f.bar.map((s) => ({ spell: s.spell, rarity: s.rarity, traits: s.traits, power: s.power })) }))];
}));

// the first 600 choices the first mind makes in a duel of two first minds, each as [tick, fighter, choice]
const firstMind = Object.fromEntries([1, 2, 3, 4, 5].map((seed) => {
  const choices = [];
  let tick = 0;
  const record = (d, f) => {
    const m = baselineMind(d, f);
    const think = m.think;
    m.think = (duel, me, foe, dt) => {
      const c = think(duel, me, foe, dt);
      if (c && choices.length < 600) choices.push([tick, me.index, c]);
      return c;
    };
    return m;
  };
  const d = duel.createDuel({ seed, left: 0, right: 220, ground: 150, has: (id) => registry.has(id), Body, Mind, minds: [record, record] });
  for (; choices.length < 600 && tick < 600 * 60; tick++) duel.step(d, 1 / 60);
  return [String(seed), choices];
}));

const ts = Array.from({ length: 22 }, (_, i) => i * 0.37);
const out = {
  rng,
  orbOffset: ts.map((t) => [t, ...duel.orbOffset(t)]),
  blockChance: [0, 1, 5, 20, 33.3, 80].map((p) => [p, duel.blockChance(p)]),
  shockInterval: [1, 2, 3, 4, 5].map((n) => [n, duel.shockInterval(n)]),
  slotPower: TIERS.flatMap((rarity) => [[], ["heavy"]].flatMap((traits) => [false, true].map((evolved) => [rarity, traits, evolved, duel.slotPower(rarity, traits, evolved)]))),
  damage: DAMAGE,
  arenas,
  firstMind,
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
