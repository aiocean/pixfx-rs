// The duel's rules and minds, driven the way the hero drives them: step after step, playing what comes back.
import { describe, expect, test } from "bun:test";
import { SCHOOLS, SPELLS, STATS, createDuel, damageOf, step } from "../assets/duel.js";

const PLAN2 = new Set([
  "fire/candle", "fire/campfire", "fire/brazier", "impacts/parry", "explosions/plasma-explosion", "explosions/implosion",
  "explosions/small-explosion", "arcane/rune-circle", "lightning/ball-lightning", "projectiles/laser-bolt",
  "ice/icicle-fall", "shadow/void-orb", "holy/feather-burst", "spawn/poof", "fire/spirit-flame", "weather/fireflies",
]);
const MAIN = new Set(["fire/candle", "fire/campfire", "fire/brazier"]);

const arena = (seed, have = PLAN2) => createDuel({ seed, left: 0, right: 220, ground: 150, has: (id) => have.has(id) });

/** Run a duel for `seconds` at 60 fps; returns every command and what happened in each round. */
function run(duel, seconds) {
  const commands = [];
  const rounds = [];
  let round = duel.round;
  for (let i = 0; i < seconds * 60; i++) {
    commands.push(...step(duel, 1 / 60));
    if (duel.round !== round) {
      rounds.push(duel.wins.slice());
      round = duel.round;
    }
  }
  return { commands, rounds };
}

describe("duel", () => {
  test("a seed replays the same fight", () => {
    const a = run(arena(7), 20).commands;
    const b = run(arena(7), 20).commands;
    expect(a.length).toBeGreaterThan(20);
    expect(JSON.stringify(b)).toBe(JSON.stringify(a));
  });

  test("fights end in a knockout and the next round starts", () => {
    for (const seed of [1, 2, 3, 4, 5]) {
      const { rounds } = run(arena(seed), 120);
      expect(rounds.length).toBeGreaterThanOrEqual(1);
      const [last] = rounds.slice(-1);
      expect(last[0] + last[1]).toBe(rounds.length);
    }
  });

  test("both schools use their own magic, and only effects that exist", () => {
    const played = new Set();
    for (const seed of [1, 2, 3]) for (const c of run(arena(seed), 60).commands) if (c.type === "play") played.add(c.fx);
    for (const fx of played) expect(PLAN2.has(fx)).toBe(true);
    for (const fx of ["projectiles/laser-bolt", "lightning/ball-lightning", "ice/icicle-fall", "arcane/rune-circle"]) {
      expect(played.has(fx)).toBe(true);
    }
  });

  test("with only the fire effects, they fight with swords and play nothing that is missing", () => {
    const duel = arena(3, MAIN);
    // the blade needs no effect: a slash, and a dive from the air
    expect(duel.fighters.map((f) => f.spells)).toEqual([["slash", "dive"], ["slash", "dive"]]);
    const { commands, rounds } = run(duel, 120);
    for (const c of commands) if (c.type === "play") expect(MAIN.has(c.fx)).toBe(true);
    expect(rounds.length).toBeGreaterThanOrEqual(1);
  });

  test("a fighter that sees a bolt coming mostly gets out of its way", () => {
    let hits = 0;
    const tries = 40;
    for (let seed = 1; seed <= tries; seed++) {
      const duel = arena(seed);
      const [shooter, target] = duel.fighters;
      // the shooter only shoots; the target is free and full of qi
      shooter.spells = ["bolt"];
      shooter.think = 1e9;
      // its first look comes once the bolt has been in the air for its reaction time
      target.think = 0.2;
      shooter.x = 40;
      target.x = 180;
      shooter.act = { spell: "bolt", phase: "windup", t: SPELLS.bolt.windup };
      const before = target.hp;
      for (let i = 0; i < 60; i++) step(duel, 1 / 60);
      // a blocked bolt still chips a little; a hit is one that was neither dodged nor blocked
      if (before - target.hp >= SPELLS.bolt.damage / 2) hits++;
    }
    expect(hits / tries).toBeLessThan(0.3);
  });

  test("a fighter that sees a palm blast wound up within reach mostly avoids it", () => {
    let hits = 0;
    const tries = 40;
    for (let seed = 1; seed <= tries; seed++) {
      const duel = arena(seed);
      const [striker, target] = duel.fighters;
      striker.think = 1e9;
      striker.x = 100;
      target.x = 125;
      target.think = 0.17;
      striker.act = { spell: "palm", phase: "windup", t: 0 };
      const before = target.hp;
      for (let i = 0; i < 40; i++) step(duel, 1 / 60);
      if (before - target.hp >= SPELLS.palm.damage / 2) hits++;
    }
    expect(hits / tries).toBeLessThan(0.3);
  });

  test("an icicle is aimed where a walking foe will be", () => {
    const duel = arena(5);
    const [caster, foe] = duel.fighters;
    caster.spells = ["icicle"];
    caster.qi = 100;
    caster.think = 0;
    foe.think = 1e9;
    foe.x = 120;
    foe.intent = { move: 1 };
    step(duel, 1 / 60);
    expect(caster.act?.spell).toBe("icicle");
    expect(caster.act.target).toBeGreaterThan(foe.x + 20);
  });
  test("an ultimate waits for full rage, and blows landed or taken fill it", () => {
    const duel = arena(4);
    const [me] = duel.fighters;
    me.spells = ["tribulation"];
    me.qi = 100;
    me.think = 0;
    step(duel, 1 / 60);
    expect(me.act).toBeNull();
    const { commands } = run(duel, 60);
    expect(commands.filter((c) => c.type === "ultimate").length).toBeGreaterThan(0);
  });

  test("a fighter caught in a prison's pull cannot blink out of it", () => {
    let escaped = 0;
    for (let seed = 1; seed <= 20; seed++) {
      const duel = arena(seed);
      const [caster, target] = duel.fighters;
      caster.think = 1e9;
      caster.x = 60;
      target.x = 120;
      target.qi = 100;
      target.think = 0.2;
      caster.rage = 8;
      caster.act = { spell: "prison", phase: "windup", t: SPELLS.prison.windup - 1 / 60, target: 120 };
      const commands = run(duel, 1.3).commands;
      if (commands.some((c) => c.type === "play" && c.fx === SPELLS.blink.fx)) escaped++;
    }
    expect(escaped).toBe(0);
  });

  test("a frozen fighter stays where it is", () => {
    const duel = arena(6);
    const [, target] = duel.fighters;
    target.frozen = 0.8;
    target.intent = { move: 1 };
    const x = target.x;
    for (let i = 0; i < 30; i++) step(duel, 1 / 60);
    expect(target.x).toBe(x);
  });
  test("damage is attack times the skill's power, cut by the matching defence, raised by a crit", () => {
    const thunder = { stats: STATS.thunder, fury: 0 };
    const frost = { stats: STATS.frost, fury: 0, defDown: 0 };
    const physical = damageOf(thunder, frost, 10, "physical", false, 0.5);
    const magic = damageOf(thunder, frost, 10, "magic", false, 0.5);
    // frost guards better against spells than blades
    expect(magic).toBeLessThan(physical);
    expect(damageOf(thunder, frost, 20, "physical", false, 0.5)).toBeCloseTo(physical * 2, -1);
    expect(damageOf(thunder, frost, 10, "physical", true, 0.5)).toBeCloseTo(physical * STATS.thunder.critDamage, -1);
    expect(damageOf(thunder, { ...frost, defDown: 2 }, 10, "physical", false, 0.5)).toBeGreaterThan(physical);
    expect(damageOf({ ...thunder, fury: 5 }, frost, 10, "physical", false, 0.5)).toBeGreaterThan(physical);
  });

  /** Hold both fighters still, then land one bolt on fighter `at` from the other. */
  function boltOn(duel, at, setup) {
    const target = duel.fighters[at];
    const shooter = duel.fighters[1 - at];
    shooter.think = 1e9;
    target.think = 1e9;
    shooter.x = at === 1 ? 60 : 120;
    target.x = at === 1 ? 120 : 60;
    shooter.face = Math.sign(target.x - shooter.x);
    target.stats = { ...target.stats, dodge: 0 };
    setup?.(target);
    shooter.act = { spell: "bolt", phase: "windup", t: SPELLS.bolt.windup - 1 / 60 };
    const hits = [];
    for (let i = 0; i < 40; i++) for (const c of step(duel, 1 / 60)) if (c.type === "hit" || c.type === "unyielding") hits.push(c);
    return hits;
  }

  test("a frost ward takes a blow before life does", () => {
    const duel = arena(2);
    const [, frost] = duel.fighters;
    const hits = boltOn(duel, 1, (t) => {
      t.shield = 5000;
      t.shieldT = 5;
    });
    expect(hits.filter((h) => h.type === "hit").length).toBe(1);
    expect(frost.hp).toBe(100);
    expect(frost.shield).toBeLessThan(5000);
  });

  test("thunder stands up from its first killing blow each round, with 1 life, but not its second", () => {
    const duel = arena(2);
    const [thunder] = duel.fighters;
    expect(thunder.school).toBe("thunder");
    let hits = boltOn(duel, 0, (t) => (t.hp = 0.5));
    expect(hits.some((h) => h.type === "unyielding")).toBe(true);
    expect(thunder.hp).toBeGreaterThan(0);
    hits = boltOn(duel, 0);
    expect(thunder.hp).toBe(0);
    expect(hits.some((h) => h.type === "unyielding")).toBe(false);
  });

  test("a burn keeps hurting after the orb is gone", () => {
    const duel = arena(3);
    const [, target] = duel.fighters;
    target.think = 1e9;
    duel.fighters[0].think = 1e9;
    target.burn = { t: 3, tick: 0.5, per: 50, by: 0 };
    const ticks = [];
    for (let i = 0; i < 4 * 60; i++) for (const c of step(duel, 1 / 60)) if (c.type === "tick") ticks.push(c);
    expect(ticks.length).toBe(6);
    expect(target.burn).toBeNull();
  });

  test("a silenced fighter answers only with its blade", () => {
    const duel = arena(5);
    const [caster, foe] = duel.fighters;
    caster.spells = ["bolt", "slash"];
    caster.qi = 100;
    caster.silence = 3;
    foe.think = 1e9;
    const cast = new Set();
    for (let i = 0; i < 2 * 60; i++) {
      step(duel, 1 / 60);
      if (caster.act) cast.add(caster.act.spell);
    }
    expect(cast.has("bolt")).toBe(false);
  });
  test("whose spell it is shows in its colours: every tinted effect a fighter plays uses its school's palette", () => {
    const owner = new Map();
    let checked = 0;
    for (const seed of [1, 2, 3]) {
      const duel = arena(seed);
      for (let i = 0; i < 40 * 60; i++) {
        for (const c of step(duel, 1 / 60)) {
          if (c.type !== "play" || !c.opts?.palette) continue;
          const schools = Object.entries(SCHOOLS).filter(([, sc]) =>
            [sc.tint?.[c.fx]?.palette, sc.circle, sc.bolt].some((p) => JSON.stringify(p) === JSON.stringify(c.opts.palette)),
          );
          // the palette belongs to exactly one school
          expect(schools.length).toBe(1);
          owner.set(c.key, schools[0][0]);
          checked++;
        }
      }
    }
    expect(checked).toBeGreaterThan(50);
    expect(new Set(owner.values()).size).toBe(2);
  });

  test("the same blast is gold from thunder and ice blue from frost", () => {
    for (const [index, spell] of [[0, "seal"], [1, "shatter"]]) {
      const duel = arena(4);
      const me = duel.fighters[index];
      const foe = duel.fighters[1 - index];
      me.think = 1e9;
      foe.think = 1e9;
      me.act = { spell, phase: "windup", t: SPELLS[spell].windup - 1 / 60, target: foe.x };
      let strike;
      for (let i = 0; i < 60 && !strike; i++) strike = step(duel, 1 / 60).find((c) => c.type === "play" && c.fx === "explosions/plasma-explosion");
      expect(strike.opts.palette).toEqual(SCHOOLS[me.school].tint["explosions/plasma-explosion"].palette);
    }
  });

  test("a seal marks the spot before its strike lands there", () => {
    const duel = arena(4);
    const [caster, foe] = duel.fighters;
    caster.think = 1e9;
    foe.think = 1e9;
    caster.x = 60;
    foe.x = 140;
    caster.act = { spell: "seal", phase: "windup", t: SPELLS.seal.windup - 1 / 60, target: 140 };
    const plays = [];
    for (let i = 0; i < 60; i++) for (const c of step(duel, 1 / 60)) if (c.type === "play") plays.push({ ...c, i });
    const mark = plays.find((c) => c.fx === "arcane/rune-circle" && c.x === 140);
    const strike = plays.find((c) => c.fx === SPELLS.seal.fx);
    expect(mark).toBeDefined();
    expect(strike.x).toBe(140);
    expect((strike.i - mark.i) / 60).toBeCloseTo(SPELLS.seal.sigil, 1);
  });
  test("two fighters on the ground keep a little room between them", () => {
    for (const seed of [1, 2, 3, 4, 5]) {
      const duel = arena(seed);
      const [a, b] = duel.fighters;
      // start them on one spot
      a.x = 110;
      b.x = 110;
      let closest = Infinity;
      for (let i = 0; i < 60 * 60; i++) {
        step(duel, 1 / 60);
        if (duel.winner === null && Math.abs(a.air - b.air) < 20) closest = Math.min(closest, Math.abs(a.x - b.x));
      }
      expect(closest).toBeGreaterThanOrEqual(16);
    }
  });
});
