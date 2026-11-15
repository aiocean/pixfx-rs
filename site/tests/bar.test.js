// The rotating skill bar: three slots, each a skill drawn from the school's pool with its own casts and power; a
// spent skill empties its slot, which counts down and draws a new one.
import { describe, expect, test } from "bun:test";
import { SCHOOLS, SPELLS, addChill, addShock, castsPower, createDuel, draw, skillBar, spend, step } from "../assets/duel.js";
import { ICON_CELLS, ICON_COLUMNS } from "../assets/skill-icons.js";
import { Body, Mind, effects } from "./engine.js";

const all = new Set(effects());
const arena = (seed) => createDuel({ seed, left: 0, right: 220, ground: 150, has: (id) => all.has(id), Body, Mind });
const onBar = (f) => f.bar.filter((s) => !s.empty).map((s) => s.spell);

describe("the bar", () => {
  test("fewer casts hit harder", () => {
    expect([1, 2, 3, 4].map(castsPower)).toEqual([1.4, 1.2, 1.1, 1.0]);
  });

  test("a fresh round draws three different skills from the school's pool", () => {
    for (let seed = 1; seed <= 10; seed++) {
      for (const f of arena(seed).fighters) {
        expect(f.bar.length).toBe(3);
        const spells = onBar(f);
        expect(spells.length).toBe(3);
        expect(new Set(spells).size).toBe(3);
        for (const s of f.bar) {
          expect(SCHOOLS[f.school].pool).toContain(s.spell);
          expect(s.casts).toBeGreaterThanOrEqual(1);
          expect(s.casts).toBeLessThanOrEqual(4);
          expect(s.left).toBe(s.casts);
          expect(s.power).toBe(castsPower(s.casts));
        }
      }
    }
  });

  test("a spent skill empties its slot, which counts down 3 to 5 s and draws a skill not on the bar", () => {
    for (let seed = 1; seed <= 6; seed++) {
      const duel = arena(seed);
      const f = duel.fighters[seed % 2];
      f.mind.hold(1e9);
      duel.fighters[1 - f.index].mind.hold(1e9);
      const casts = f.bar[0].casts;
      for (let k = 0; k < casts; k++) spend(duel, f, 0);
      expect(f.bar[0].empty).toBeGreaterThanOrEqual(3);
      expect(f.bar[0].empty).toBeLessThanOrEqual(5);
      const wait = f.bar[0].empty;
      for (let i = 0; i < Math.ceil(wait * 60) + 1; i++) step(duel, 1 / 60);
      expect(f.bar[0].empty).toBeUndefined();
      const others = [f.bar[1].spell, f.bar[2].spell];
      expect(others).not.toContain(f.bar[0].spell);
    }
  });

  test("a draw fills an empty slot with a skill not on the bar", () => {
    const duel = arena(4);
    const f = duel.fighters[0];
    f.bar[1] = { empty: 1 };
    draw(duel, f, 1);
    expect(f.bar[1].spell).toBeDefined();
    expect(new Set(onBar(f)).size).toBe(3);
  });

  test("the blade, the dive, the blink and the ultimate are always at hand", () => {
    for (const f of arena(2).fighters) {
      for (const name of ["slash", "dive", "blink", SCHOOLS[f.school].ultimate]) expect(f.spells).toContain(name);
    }
  });

  test("the same seed plays the same bar", () => {
    const bars = () => {
      const duel = arena(7);
      const seen = [];
      for (let i = 0; i < 60 * 60; i++) {
        step(duel, 1 / 60);
        if (i % 30 === 0) seen.push(JSON.stringify(duel.fighters.map((f) => f.bar)));
      }
      return seen;
    };
    expect(bars()).toEqual(bars());
  }, 30_000);

  test("skill bars turn over in a fight", () => {
    const duel = arena(3);
    const seen = new Set();
    for (let i = 0; i < 90 * 60; i++) {
      step(duel, 1 / 60);
      for (const f of duel.fighters) for (const s of onBar(f)) seen.add(`${f.index}:${s}`);
    }
    expect(seen.size).toBeGreaterThan(8);
  }, 30_000);

  test("the screen's bar holds no NaN, before a fight and after a knockout", () => {
    const duel = arena(5);
    const clean = (f) => {
      for (const slot of skillBar(f)) for (const v of Object.values(slot)) if (typeof v === "number") expect(Number.isNaN(v)).toBe(false);
    };
    for (const f of duel.fighters) clean(f);
    duel.fighters[0].hp = 0;
    step(duel, 1 / 60);
    for (const f of duel.fighters) clean(f);
  });
});

describe("the pools", () => {
  test("the new spells are in their schools' pools, and every pool spell's effect is in the engine", () => {
    for (const name of ["spark", "burst", "quake", "hex"]) expect(SCHOOLS.thunder.pool).toContain(name);
    for (const name of ["shard", "jet", "flurry"]) expect(SCHOOLS.frost.pool).toContain(name);
    for (const school of Object.values(SCHOOLS)) {
      for (const name of [...school.pool, school.ultimate]) {
        const fx = SPELLS[name].fx;
        if (fx) expect(all.has(fx)).toBe(true);
        expect(SPELLS[name].cost).toBeGreaterThan(name === school.ultimate ? -1 : 0);
      }
    }
  });

  test("every pool spell is cast in some fight", () => {
    const cast = new Set();
    // heal and void are rare picks for the brain (a heal only below half life, a void only with a long opening)
    for (let seed = 1; seed <= 24; seed++) {
      const duel = arena(seed);
      for (let i = 0; i < 120 * 60; i++) {
        step(duel, 1 / 60);
        for (const f of duel.fighters) if (f.act && f.act.t === 0) cast.add(f.act.spell);
      }
    }
    for (const school of Object.values(SCHOOLS)) for (const name of school.pool) expect([name, cast.has(name)]).toEqual([name, true]);
  }, 300_000);
});

describe("draw rules", () => {
  test("a round's first slot is the school's signature kind", () => {
    for (let seed = 1; seed <= 15; seed++) {
      for (const f of arena(seed).fighters) expect(SPELLS[f.bar[0].spell].kind).toBe(SCHOOLS[f.school].signature);
    }
  });

  test("with a status on the foe, the skills that spend it are drawn far more often", () => {
    const rate = (set) => {
      let pays = 0;
      const n = 600;
      const duel = arena(9);
      const [thunder, frost] = duel.fighters;
      for (let k = 0; k < n; k++) {
        set(duel, thunder, frost);
        thunder.bar[1] = { empty: 1, full: 1 };
        draw(duel, thunder, 1);
        if (["palm", "burst"].includes(thunder.bar[1].spell)) pays++;
      }
      return pays / n;
    };
    const plain = rate((duel, by, f) => {
      f.shock = null;
    });
    const shocked = rate((duel, by, f) => {
      f.shock = null;
      addShock(duel, f, by, 2);
    });
    expect(shocked).toBeGreaterThan(plain + 0.15);
  });

  test("a draw never repeats a skill on the bar", () => {
    const duel = arena(11);
    const f = duel.fighters[1];
    for (let k = 0; k < 300; k++) {
      addChill(duel, duel.fighters[0], 1);
      const i = k % 3;
      f.bar[i] = { empty: 1, full: 1 };
      draw(duel, f, i);
      expect(new Set(onBar(f)).size).toBe(3);
    }
  });
});

describe("the icons", () => {
  test("every bar skill and ultimate of a school has its own cell in the icon atlas", () => {
    const cells = [];
    for (const [school, { pool, ultimate }] of Object.entries(SCHOOLS)) {
      for (const name of [...pool, ultimate]) {
        const cell = ICON_CELLS[school]?.[name];
        expect([school, name, Number.isInteger(cell)]).toEqual([school, name, true]);
        cells.push(cell);
      }
    }
    expect(new Set(cells).size).toBe(cells.length);
    expect(Math.max(...cells)).toBeLessThan(ICON_COLUMNS * ICON_COLUMNS);
  });
});
