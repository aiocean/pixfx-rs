// The rotating skill bar: three slots, each a skill drawn from the school's pool with its own casts and power; a
// spent skill empties its slot, which counts down and draws a new one.
import { describe, expect, test } from "bun:test";
import { PAYOFF, POOL_SIZE, SCHOOLS, SPELLS, slotPower, addChill, addShock, castsPower, createDuel, draw, skillBar, spend, step } from "../assets/duel.js";
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
          expect(s.power).toBeCloseTo(slotPower(s.casts, s.rarity, s.traits, false), 9);
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
      // three skills with no evolution partner among them, so the spent one empties its slot
      const plain = f.school === "thunder" ? ["orb", "bell", "heal"] : ["mirror", "void", "frostfire"];
      f.bar = plain.map((spell) => ({ spell, casts: 2, left: 2, power: castsPower(2), evolved: false }));
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

describe("a fight's pool", () => {
  test("each fight plays 8 of its school's skills, with its signature kind and a skill that pays off its status", () => {
    expect(POOL_SIZE).toBe(8);
    for (let seed = 1; seed <= 30; seed++) {
      for (const f of arena(seed).fighters) {
        const school = SCHOOLS[f.school];
        expect(f.pool.length).toBe(8);
        expect(new Set(f.pool).size).toBe(8);
        for (const name of f.pool) expect(school.pool).toContain(name);
        expect(f.pool.some((name) => SPELLS[name].kind === school.signature)).toBe(true);
        expect(f.pool.some((name) => PAYOFF[name])).toBe(true);
        expect(f.spells.filter((name) => school.pool.includes(name)).sort()).toEqual([...f.pool].sort());
      }
    }
  });

  test("the same seed plays the same pools, and seeds differ", () => {
    const pools = (seed) => arena(seed).fighters.map((f) => [...f.pool].sort().join(",")).join("|");
    expect(pools(5)).toBe(pools(5));
    const seen = new Set();
    for (let seed = 1; seed <= 10; seed++) seen.add(pools(seed));
    expect(seen.size).toBeGreaterThanOrEqual(5);
  });
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
      // the whole pool, so both payoff skills can be drawn whatever this fight's pool holds
      thunder.pool = [...SCHOOLS.thunder.pool];
      thunder.bar[0] = { spell: "orb", casts: 1, left: 1, power: 1.4, evolved: false };
      thunder.bar[2] = { spell: "bell", casts: 1, left: 1, power: 1.4, evolved: false };
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

  test("a new round's draws ignore the statuses the last round left on the foe", () => {
    const nextBar = (seed, leftover) => {
      const duel = arena(seed);
      for (const f of duel.fighters) f.mind.hold(1e9);
      const [thunder, frost] = duel.fighters;
      if (leftover) addShock(duel, frost, thunder, 3);
      frost.hp = 0;
      const round = duel.round;
      for (let i = 0; i < 60 * 10 && duel.round === round; i++) step(duel, 1 / 60);
      expect(duel.round).toBe(round + 1);
      return thunder.bar.map((slot) => slot.spell);
    };
    for (let seed = 1; seed <= 20; seed++) expect([seed, nextBar(seed, true)]).toEqual([seed, nextBar(seed, false)]);
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

describe("evolution", () => {
  /** A still duel; fighter `f`'s bar set to `spells`, each with `casts` casts. */
  function barOf(seed, index, spells, casts = 1) {
    const duel = arena(seed);
    for (const g of duel.fighters) g.mind.hold(1e9);
    const f = duel.fighters[index];
    f.bar = spells.map((spell) => ({ spell, casts, left: casts, power: castsPower(casts), evolved: false }));
    return { duel, f };
  }

  test("a spent skill whose partner is on the bar refills at once, evolved and half again as strong", () => {
    for (let seed = 1; seed <= 8; seed++) {
      const { duel, f } = barOf(seed, 0, ["palm", "step", "orb"]);
      spend(duel, f, 0);
      const slot = f.bar[0];
      expect(slot.spell).toBe("palm");
      expect(slot.evolved).toBe(true);
      expect(slot.left).toBe(slot.casts);
      expect(slot.power).toBeCloseTo(1.5 * castsPower(slot.casts), 9);
    }
  });

  test("each school has its own pairs: Frost's shard evolves with the icicle, Thunder's bolt with the seal", () => {
    expect(SCHOOLS.frost.pool).not.toContain("bolt");
    const frost = barOf(1, 1, ["shard", "icicle", "lotus"]);
    spend(frost.duel, frost.f, 0);
    expect(frost.f.bar[0].evolved).toBe(true);
    const thunder = barOf(1, 0, ["bolt", "seal", "orb"]);
    spend(thunder.duel, thunder.f, 0);
    expect(thunder.f.bar[0].evolved).toBe(true);
  });

  test("without its partner on the bar a spent skill counts down as usual", () => {
    const { duel, f } = barOf(2, 0, ["palm", "orb", "bell"]);
    spend(duel, f, 0);
    expect(f.bar[0].empty).toBeGreaterThanOrEqual(3);
  });

  test("an evolved skill, once spent, empties its slot: it does not evolve again", () => {
    const { duel, f } = barOf(3, 0, ["palm", "step", "orb"]);
    spend(duel, f, 0);
    const casts = f.bar[0].casts;
    for (let k = 0; k < casts; k++) spend(duel, f, 0);
    expect(f.bar[0].empty).toBeGreaterThanOrEqual(3);
  });

  test("an evolved skill inflicts one more stack of what it inflicts", () => {
    const chillOf = (evolved) => {
      let cast = false;
      const caster = () => ({
        think: () => (cast ? null : ((cast = true), { type: "cast", skill: "jet" })),
        tell() {}, reset() {}, hold() {},
      });
      const idle = () => ({ think: () => null, tell() {}, reset() {}, hold() {} });
      const duel = createDuel({ seed: 4, left: 0, right: 220, ground: 150, has: (id) => all.has(id), Body, Mind, minds: [idle, caster] });
      const [thunder, frost] = duel.fighters;
      for (const g of duel.fighters) g.stats = { ...g.stats, dodge: 0, crit: 0 };
      frost.bar = [{ spell: "jet", casts: 2, left: 2, power: 1, evolved }, { spell: "shard", casts: 1, left: 1, power: 1, evolved: false }, { spell: "mirror", casts: 1, left: 1, power: 1, evolved: false }];
      frost.x = thunder.x + 30;
      frost.face = -1;
      for (let i = 0; i < 60; i++) step(duel, 1 / 60);
      return thunder.chill;
    };
    expect(chillOf(false)).toBe(1);
    expect(chillOf(true)).toBe(2);
  });
});
