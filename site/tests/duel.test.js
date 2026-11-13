// The duel's rules and minds, driven the way the hero drives them: step after step, playing what comes back.
import { describe, expect, test } from "bun:test";
import { SPELLS, createDuel, step } from "../assets/duel.js";

const PLAN2 = new Set([
  "fire/candle", "fire/campfire", "fire/brazier", "impacts/parry", "explosions/plasma-explosion", "explosions/implosion",
  "explosions/small-explosion", "arcane/rune-circle", "lightning/ball-lightning", "projectiles/laser-bolt",
  "ice/icicle-fall", "shadow/void-orb", "holy/feather-burst", "spawn/poof", "fire/spirit-flame", "weather/fireflies",
]);

const arena = (seed) => createDuel({ seed, left: 0, right: 220, ground: 150, has: (id) => PLAN2.has(id) });

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

  /** Two fighters face to face and frozen in thought, so a test decides everything they do. */
  function standoff(seed = 2) {
    const duel = arena(seed);
    const [thunder, frost] = duel.fighters;
    for (const f of duel.fighters) {
      f.think = 1e9;
      f.stats = { ...f.stats, dodge: 0 };
    }
    thunder.x = 100;
    frost.x = 116;
    thunder.face = 1;
    frost.face = -1;
    return { duel, thunder, frost };
  }

  /** Step until `done()` or `seconds` pass; returns the poses fighter `f` took, in order, and the seconds it took. */
  function watch(duel, f, done, seconds = 4) {
    const poses = [];
    for (let i = 0; i < seconds * 60; i++) {
      step(duel, 1 / 60);
      if (poses.at(-1) !== f.pose) poses.push(f.pose);
      if (done()) return { poses, t: (i + 1) / 60 };
    }
    return { poses, t: Infinity };
  }

  test("a heavy blow throws its target, which bounces, lies, gets up, and only then can be hit again", () => {
    const { duel, thunder, frost } = standoff();
    thunder.act = { spell: "palm", phase: "windup", t: SPELLS.palm.windup - 1 / 60 };
    step(duel, 1 / 60);
    expect(frost.down?.phase).toBe("air");
    frost.down.quick = false;
    const lands = [];
    const { poses } = watch(duel, frost, () => {
      for (const c of duel.out) if (c.type === "land") lands.push(c);
      if (frost.down?.phase === "down") frost.down.quick = false;
      return frost.down === null;
    });
    // up it goes, comes down, bounces off the floor once, lies there, sits up, kneels, and stands
    expect(poses.indexOf("down")).toBeGreaterThan(poses.indexOf("fall"));
    expect(poses.slice(poses.indexOf("down"))).toEqual(["down", "sit", "kneel", "crouch", "land"]);
    expect(lands.length).toBe(2);
  });
});
