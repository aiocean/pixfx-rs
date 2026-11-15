// Guard by the size of the blow (Halls of Torment), a ward that erases whole blows (Valhalla), and little spread.
import { describe, expect, test } from "bun:test";
import { SPELLS, addShock, blockChance, createDuel, damageOf, step } from "../assets/duel.js";
import { Body, Mind, effects } from "./engine.js";

const all = new Set(effects());
function still(seed = 1) {
  const duel = createDuel({ seed, left: 0, right: 220, ground: 150, has: (id) => all.has(id), Body, Mind });
  for (const f of duel.fighters) {
    f.mind.hold(1e9);
    f.stats = { ...f.stats, dodge: 0, crit: 0 };
  }
  return duel;
}

/** Fighter `by` lands a `spell` on `f` from 20 px; returns the hit commands and whether f's life dropped. */
function strike(duel, by, f, spell) {
  by.act = { spell, phase: "windup", t: SPELLS[spell].windup, target: f.x, power: 1 };
  by.x = f.x - 20 * Math.sign(f.x - by.x || 1);
  by.face = Math.sign(f.x - by.x);
  const out = [];
  for (let i = 0; i < 30; i++) out.push(...step(duel, 1 / 60));
  return out;
}

describe("guard", () => {
  test("light blows are always turned aside, heavy ones less often", () => {
    expect(blockChance(5)).toBe(1);
    expect(blockChance(11)).toBeCloseTo(0.5 * Math.sqrt(20 / 11), 9);
    expect(blockChance(24)).toBeCloseTo(0.5 * (20 / 24), 9);
  });

  test("a failed block lets the whole blow through, and the guard's qi is paid anyway", () => {
    let through = 0;
    let held = 0;
    for (let seed = 1; seed <= 40; seed++) {
      const duel = still(seed);
      const [by, f] = duel.fighters;
      f.face = Math.sign(by.x - f.x);
      f.guard = 5;
      const qi = f.qi;
      const out = strike(duel, by, f, "palm");
      expect(f.qi).toBeLessThan(qi);
      if (out.some((c) => c.type === "hit" && c.spell === "palm")) through++;
      else held++;
    }
    // palm (power 20): about half get through
    expect(through).toBeGreaterThan(8);
    expect(held).toBeGreaterThan(8);
  });
});

describe("ward", () => {
  test("each charge erases one whole blow", () => {
    const duel = still(2);
    const [by, f] = duel.fighters;
    f.charges = 2;
    f.shieldT = 6;
    const hp = f.hp;
    strike(duel, by, f, "slash");
    by.cool = {};
    strike(duel, by, f, "slash");
    expect(f.hp).toBe(hp);
    expect(f.charges).toBe(0);
    by.cool = {};
    strike(duel, by, f, "slash");
    expect(f.hp).toBeLessThan(hp);
  });

  test("a shock tick passes the ward", () => {
    const duel = still(2);
    const [by, f] = duel.fighters;
    f.charges = 3;
    f.shieldT = 6;
    addShock(duel, f, by, 1);
    const hp = f.hp;
    for (let i = 0; i < 60; i++) step(duel, 1 / 60);
    expect(f.hp).toBeLessThan(hp);
    expect(f.charges).toBe(3);
  });

  test("a ward holds 1 to 3 charges", () => {
    const seen = new Set();
    for (let seed = 1; seed <= 30; seed++) {
      const duel = still(seed);
      const f = duel.fighters[1];
      f.act = { spell: "ward", phase: "windup", t: SPELLS.ward.windup, target: f.x, power: 1 };
      step(duel, 1 / 60);
      seen.add(f.charges);
    }
    expect([...seen].sort()).toEqual([1, 2, 3]);
  });
});

describe("spread", () => {
  test("damage stays within ±1.5% of its middle", () => {
    const duel = still();
    const [by, f] = duel.fighters;
    const mid = damageOf(by, f, 20, "magic", false, 0.5);
    for (const roll of [0, 0.25, 0.75, 0.999]) {
      expect(Math.abs(damageOf(by, f, 20, "magic", false, roll) - mid) / mid).toBeLessThanOrEqual(0.016);
    }
    expect(damageOf(by, f, 20, "magic", false, 0)).toBeLessThan(damageOf(by, f, 20, "magic", false, 0.999));
  });
});
