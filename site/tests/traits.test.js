// Chance on the bar: each draw rolls a rarity (a little more power) and traits that change how its skill plays.
import { describe, expect, test } from "bun:test";
import { LUCKY, RARITY, SPELLS, TRAITS, createDuel, draw, fits, ready, slotPower, step } from "../assets/duel.js";
import { Body, Mind, effects } from "./engine.js";

const all = new Set(effects());
const idle = () => ({ think: () => null, tell() {}, reset() {}, hold() {} });
/** A skill that lives on the bar (not the blade, the dive, the blink or an ultimate). */
const onBarSkill = (name) => !["slash", "dive", "blink"].includes(name) && !SPELLS[name].ultimate;

/** A duel whose fighter `index` casts `spell` once, from a bar slot set by `slot`; the other stands still. */
function castOnce(seed, index, spell, slot, setup = () => {}) {
  let cast = false;
  const caster = () => ({
    think: () => (cast ? null : ((cast = true), { type: "cast", skill: spell })),
    tell() {}, reset() {}, hold() {},
  });
  const minds = index === 0 ? [caster, idle] : [idle, caster];
  const duel = createDuel({ seed, left: 0, right: 220, ground: 150, has: (id) => all.has(id), Body, Mind, minds });
  const me = duel.fighters[index];
  const foe = duel.fighters[1 - index];
  for (const g of duel.fighters) g.stats = { ...g.stats, dodge: 0, crit: 0 };
  const traits = slot.traits ?? [];
  me.bar = [
    { spell, rarity: "common", evolved: false, ...slot, traits, power: slotPower(slot.rarity ?? "common", traits, !!slot.evolved) },
    { empty: 9, full: 9 },
    { empty: 9, full: 9 },
  ];
  if (!me.pool.includes(spell)) me.pool.push(spell);
  if (!me.spells.includes(spell)) me.spells.push(spell);
  foe.x = me.x + (index === 0 ? 30 : -30);
  me.face = Math.sign(foe.x - me.x);
  setup(duel, me, foe);
  return { duel, me, foe };
}

describe("rarity", () => {
  test("draws are common 70%, rare 22% and epic 8%, and rarer hits harder", () => {
    const duel = createDuel({ seed: 3, left: 0, right: 220, ground: 150, has: (id) => all.has(id), Body, Mind });
    const f = duel.fighters[0];
    const seen = { common: 0, rare: 0, epic: 0 };
    const n = 3000;
    for (let k = 0; k < n; k++) {
      f.bar[1] = { empty: 1, full: 1 };
      draw(duel, f, 1);
      const slot = f.bar[1];
      seen[slot.rarity]++;
      expect(slot.power).toBeCloseTo(slotPower(slot.rarity, slot.traits, false), 9);
    }
    expect(Math.abs(seen.common / n - 0.7)).toBeLessThan(0.03);
    expect(Math.abs(seen.rare / n - 0.22)).toBeLessThan(0.03);
    expect(Math.abs(seen.epic / n - 0.08)).toBeLessThan(0.03);
    expect(slotPower("rare", [], false)).toBeCloseTo(RARITY.rare.power, 9);
    expect(slotPower("epic", ["heavy"], true)).toBeCloseTo(1.5 * RARITY.epic.power * TRAITS.heavy.power, 9);
  });
});

describe("traits", () => {
  test("only traits that fit a skill are rolled: a common has at most one, a rare one, an epic two", () => {
    const duel = createDuel({ seed: 8, left: 0, right: 220, ground: 150, has: (id) => all.has(id), Body, Mind });
    let commonWith = 0;
    let commons = 0;
    for (const f of duel.fighters) {
      for (let k = 0; k < 1500; k++) {
        f.bar[2] = { empty: 1, full: 1 };
        draw(duel, f, 2);
        const { spell, rarity, traits } = f.bar[2];
        for (const t of traits) expect([spell, t, fits(t, spell)]).toEqual([spell, t, true]);
        expect(new Set(traits).size).toBe(traits.length);
        const fitting = Object.keys(TRAITS).filter((t) => fits(t, spell)).length;
        if (rarity === "common") {
          commons++;
          if (traits.length) commonWith++;
          expect(traits.length).toBeLessThanOrEqual(1);
        }
        if (rarity === "rare") expect(traits.length).toBe(Math.min(1, fitting));
        if (rarity === "epic") expect(traits.length).toBe(Math.min(2, fitting));
      }
    }
    expect(Math.abs(commonWith / commons - 0.4)).toBeLessThan(0.05);
  });

  test("heavy and piercing fit only skills that hurt; lingering only skills that chill or shock", () => {
    expect(fits("heavy", "heal")).toBe(false);
    expect(fits("piercing", "ward")).toBe(false);
    expect(fits("heavy", "palm")).toBe(true);
    expect(fits("lingering", "jet")).toBe(true);
    expect(fits("lingering", "lotus")).toBe(false);
    expect(fits("swift", "heal")).toBe(true);
    expect(fits("cheap", "hex")).toBe(true);
  });

  test("swift cuts the skill's cooldown to 60%", () => {
    const { duel, me } = castOnce(1, 0, "spark", { traits: ["swift"] });
    step(duel, 1 / 60);
    expect(me.coolFull.spark).toBeCloseTo(SPELLS.spark.cooldown * TRAITS.swift.cooldown + SPELLS.spark.windup, 9);
  });

  test("cheap halves the qi a cast costs, and the skill is ready on half the qi", () => {
    const { duel, me } = castOnce(1, 0, "burst", { traits: ["cheap"] }, (d, f) => {
      f.qi = SPELLS.burst.cost * 0.5 + 0.1;
    });
    expect(ready(me, "burst")).toBe(true);
    step(duel, 1 / 60);
    expect(me.act?.spell).toBe("burst");
    expect(me.qi).toBeLessThan(1);
  });

  test("piercing goes through a guard every time", () => {
    const landed = (traits) => {
      let n = 0;
      for (let seed = 1; seed <= 20; seed++) {
        const { duel, foe } = castOnce(seed, 0, "palm", { traits }, (d, me, f) => {
          f.face = Math.sign(me.x - f.x);
          f.guard = 5;
        });
        let hit = false;
        for (let i = 0; i < 60 && !hit; i++) {
          foe.guard = 5;
          hit = step(duel, 1 / 60).some((c) => c.type === "hit" && c.spell === "palm");
        }
        if (hit) n++;
      }
      return n;
    };
    expect(landed(["piercing"])).toBe(20);
    expect(landed([])).toBeLessThan(18);
  });

  test("lingering adds a stack, on top of evolution's", () => {
    const chill = (() => {
      const { duel, foe } = castOnce(4, 1, "jet", { traits: ["lingering"] });
      for (let i = 0; i < 60 && foe.chill === 0; i++) step(duel, 1 / 60);
      return foe.chill;
    })();
    expect(chill).toBe(2);
    const { duel, foe } = castOnce(4, 0, "spark", { traits: ["lingering"], evolved: true });
    for (let i = 0; i < 60 && !foe.shock; i++) step(duel, 1 / 60);
    expect(foe.shock?.stacks).toBe(3);
  });
});

describe("lucky casts", () => {
  /** Fighter 0 casts `spell` every step it can, `n` times, with qi and the slot topped up; what each cast did. */
  function casts(spell, n) {
    const always = () => ({ think: () => ({ type: "cast", skill: spell }), tell() {}, reset() {}, hold() {} });
    const duel = createDuel({ seed: 12, left: 0, right: 220, ground: 150, has: (id) => all.has(id), Body, Mind, minds: [always, idle] });
    const me = duel.fighters[0];
    if (!me.spells.includes(spell)) me.spells.push(spell);
    const onBar = me.pool.includes(spell) || SPELLS[spell].kind !== "melee";
    if (onBar && !me.pool.includes(spell)) me.pool.push(spell);
    const out = [];
    while (out.length < n) {
      me.act = null;
      me.cool = {};
      me.qi = 50;
      me.hp = 100;
      if (onBar) me.bar = [{ spell, power: 1, evolved: false, rarity: "common", traits: [] }, { empty: 9, full: 9 }, { empty: 9, full: 9 }];
      const lucky = step(duel, 1 / 600).find((c) => c.type === "lucky");
      if (me.act?.spell === spell) out.push({ kind: lucky?.kind ?? null, power: me.act.power, qi: me.qi, cool: me.cool[spell] ?? 0, kept: me.bar[0].spell === spell });
    }
    return out;
  }

  test("about one bar cast in twelve is lucky, and each kind does what it says", () => {
    const seen = casts("spark", 3000);
    const lucky = seen.filter((c) => c.kind);
    expect(Math.abs(lucky.length / seen.length - LUCKY.chance)).toBeLessThan(0.02);
    const kinds = new Set(lucky.map((c) => c.kind));
    expect([...kinds].sort()).toEqual(["again", "double", "free"]);
    for (const c of seen) {
      if (c.kind === "double") expect(c.power).toBeCloseTo(2, 9);
      else expect(c.power).toBeCloseTo(1, 9);
      if (c.kind === "free") expect(c.qi).toBeGreaterThan(49.9);
      else expect(c.qi).toBeLessThan(45);
      // Again keeps the skill on its slot for another cast, its cooldown over; any other cast empties the slot
      if (c.kind === "again") expect([c.cool, c.kept]).toEqual([0, true]);
      else if (onBarSkill("spark")) expect([c.cool > 0, c.kept]).toEqual([true, false]);
    }
  });

  test("the blade and the ultimate are never lucky", () => {
    expect(casts("slash", 400).filter((c) => c.kind).length).toBe(0);
  });
});

describe("review fixes", () => {
  test("piercing fits only skills whose blow a guard can stop", () => {
    for (const name of ["icicle", "collapse", "quake", "frostfire", "void", "flurry", "seal"]) expect([name, fits("piercing", name)]).toEqual([name, false]);
    for (const name of ["palm", "burst", "spark", "shard", "jet", "orb", "bolt"]) expect([name, fits("piercing", name)]).toEqual([name, true]);
  });

  test("a lucky cast names its own slot, and Again puts the skill back on it", () => {
    const always = () => ({ think: () => ({ type: "cast", skill: "spark" }), tell() {}, reset() {}, hold() {} });
    const duel = createDuel({ seed: 21, left: 0, right: 220, ground: 150, has: (id) => all.has(id), Body, Mind, minds: [always, idle] });
    const me = duel.fighters[0];
    if (!me.pool.includes("spark")) me.pool.push("spark");
    if (!me.spells.includes("spark")) me.spells.push("spark");
    const kinds = new Set();
    for (let k = 0; k < 4000 && kinds.size < 3; k++) {
      me.act = null;
      me.cool = {};
      me.qi = 50;
      me.lucky = null;
      me.bar = [{ empty: 9, full: 9 }, { spell: "spark", power: 1, evolved: false, rarity: "common", traits: [] }, { empty: 9, full: 9 }];
      step(duel, 1 / 600);
      if (me.lucky) {
        expect(me.lucky.slot).toBe(1);
        expect(me.bar[1].spell === "spark").toBe(me.lucky.kind === "again");
        kinds.add(me.lucky.kind);
      }
    }
    expect([...kinds].sort()).toEqual(["again", "double", "free"]);
  });

  test("a heal on the bar heals by its power, as the AI is told", () => {
    const healed = (power) => {
      const { duel, me } = castOnce(2, 0, "heal", {}, (d, f) => {
        f.hp = 40;
        f.qi = 100;
      });
      me.bar[0].power = power;
      for (let i = 0; i < 90 && me.hp === 40; i++) step(duel, 1 / 60);
      return me.hp - 40;
    };
    expect(healed(1.4)).toBeCloseTo(SPELLS.heal.amount * 1.4, 5);
  });
});
