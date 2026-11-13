// The duel's rules and minds, driven the way the hero drives them: step after step, playing what comes back.
import { describe, expect, test } from "bun:test";
import { SPELLS, createDuel, step } from "../assets/duel.js";

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
});
