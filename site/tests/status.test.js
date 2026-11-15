// Statuses that stack (after Halls of Torment): chill builds to a freeze, shock ticks its stacks away, and some
// skills spend the stacks on the foe to hit harder.
import { describe, expect, test } from "bun:test";
import { SPELLS, STATS, addChill, addShock, createDuel, shockInterval, step } from "../assets/duel.js";
import { Body, Mind, effects } from "./engine.js";

const all = new Set(effects());
/** A duel whose two minds never act, so only the rules move. */
function still(seed = 1) {
  const duel = createDuel({ seed, left: 0, right: 220, ground: 150, has: (id) => all.has(id), Body, Mind });
  for (const f of duel.fighters) f.mind.hold(1e9);
  return duel;
}
const run = (duel, seconds) => {
  for (let i = 0; i < Math.round(seconds * 60); i++) step(duel, 1 / 60);
};

describe("chill", () => {
  test("the third stack freezes and clears the stacks", () => {
    const duel = still();
    const f = duel.fighters[0];
    addChill(duel, f, 1);
    addChill(duel, f, 1);
    expect(f.chill).toBe(2);
    expect(f.frozen).toBe(0);
    addChill(duel, f, 1);
    expect(f.chill).toBe(0);
    expect(f.frozen).toBeGreaterThan(0);
  });

  test("stacks fade 2.5 s after the last one", () => {
    const duel = still();
    const f = duel.fighters[0];
    addChill(duel, f, 1);
    run(duel, 2.3);
    expect(f.chill).toBe(1);
    run(duel, 0.3);
    expect(f.chill).toBe(0);
  });
});

describe("shock", () => {
  test("ticks come faster with more stacks", () => {
    expect(shockInterval(1)).toBeCloseTo(0.6, 9);
    expect(shockInterval(5)).toBeCloseTo(0.6 * 0.854 ** 4, 9);
  });

  test("each tick spends a stack, costs 3% of the caster's attack and holds the target", () => {
    const duel = still();
    const [by, f] = duel.fighters;
    addShock(duel, f, by, 3);
    expect(f.shock.stacks).toBe(3);
    const hp = f.hp;
    run(duel, shockInterval(3) + 1 / 60);
    expect(f.shock.stacks).toBe(2);
    expect(f.shocked).toBeGreaterThan(0);
    const lost = ((hp - f.hp) / 100) * STATS[f.school].hp;
    expect(lost).toBeCloseTo(Math.round(STATS[by.school].atk * 0.03), 0);
    run(duel, 3);
    expect(f.shock).toBeNull();
  });

  test("stacks stop at five", () => {
    const duel = still();
    const [by, f] = duel.fighters;
    addShock(duel, f, by, 9);
    expect(f.shock.stacks).toBe(5);
  });
});

describe("payoffs", () => {
  /** The damage of one `spell` from fighter `by` to `f`, with `stacks` set first. */
  function blow(spell, set) {
    const duel = still(3);
    const [thunder, frost] = duel.fighters;
    const [by, f] = SPELLS[spell] && ["palm", "burst"].includes(spell) ? [thunder, frost] : [frost, thunder];
    for (const g of duel.fighters) g.stats = { ...g.stats, dodge: 0, crit: 0 };
    set(duel, by, f);
    const hits = [];
    const hp = f.hp;
    by.act = { spell, phase: "windup", t: SPELLS[spell].windup, target: f.x, power: 1 };
    by.x = f.x - (by.index === 0 ? 20 : -20);
    by.face = Math.sign(f.x - by.x);
    for (let i = 0; i < 90 && f.hp === hp; i++) hits.push(...step(duel, 1 / 60).filter((c) => c.type === "hit"));
    return { hit: hits.find((h) => h.spell === spell), f };
  }

  test("palm hits 12% harder per shock stack, spends them, and leaves its own", () => {
    const plain = blow("palm", () => {});
    const shocked = blow("palm", (duel, by, f) => addShock(duel, f, by, 2));
    expect(shocked.hit.power / plain.hit.power).toBeCloseTo(1.24, 5);
    expect(shocked.f.shock.stacks).toBe(1);
  });

  test("lotus hits 15% harder per chill stack, and spends them", () => {
    const plain = blow("lotus", () => {});
    const chilled = blow("lotus", (duel, by, f) => {
      addChill(duel, f, 1);
      addChill(duel, f, 1);
    });
    expect(chilled.hit.power / plain.hit.power).toBeCloseTo(1.3, 5);
    expect(chilled.f.chill).toBe(0);
  });
});
