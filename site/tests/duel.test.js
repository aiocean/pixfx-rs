// The duel's rules and minds, driven the way the hero drives them: step after step, playing what comes back.
import { describe, expect, test } from "bun:test";
import { RAGE, SCHOOLS, SPELLS, createDuel, skillBar, step } from "../assets/duel.js";
import { Body, Fx, Mind, effects } from "./engine.js";

const PLAN2 = new Set([
  "fire/candle", "fire/campfire", "fire/brazier", "impacts/parry", "explosions/plasma-explosion", "explosions/implosion",
  "explosions/small-explosion", "arcane/rune-circle", "lightning/ball-lightning", "projectiles/laser-bolt",
  "ice/icicle-fall", "shadow/void-orb", "holy/feather-burst", "spawn/poof", "fire/spirit-flame", "weather/fireflies",
]);

const arena = (seed) => createDuel({ seed, left: 0, right: 220, ground: 150, has: (id) => PLAN2.has(id), Body, Mind });

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
    // ten minutes of fighting, two brains thinking: about 3 s on an idle machine
  }, 30_000);

  /** Two fighters face to face and frozen in thought, so a test decides everything they do. */
  function standoff(seed = 2) {
    const duel = arena(seed);
    const [thunder, frost] = duel.fighters;
    for (const f of duel.fighters) {
      f.mind.hold(1e9);
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
    // it gets up the slow way: sit, kneel, stand (a quick rise rolls when the duel's dice fall under its chance)
    duel.rng = () => 0.99;
    const lands = [];
    const { poses } = watch(duel, frost, () => {
      for (const c of duel.out) if (c.type === "land") lands.push(c);
      return frost.down === null;
    });
    // up it goes, comes down, bounces off the floor once, lies there, sits up, kneels, and stands
    expect(poses.indexOf("down")).toBeGreaterThan(poses.indexOf("fall"));
    expect(poses.slice(poses.indexOf("down"))).toEqual(["down", "sit", "kneel", "crouch", "land"]);
    expect(lands.length).toBe(2);
  });
  test("a cast puts its whole skill slot on cooldown: no spell in that slot is cast until it runs out", () => {
    let checked = 0;
    for (const seed of [1, 2, 3, 4]) {
      const duel = arena(seed);
      const was = [null, null];
      // the slot each fighter cast from last, and how long it stays shut
      const shut = [null, null];
      for (let i = 0; i < 90 * 60; i++) {
        step(duel, 1 / 60);
        for (const f of duel.fighters) {
          const spell = f.act?.spell ?? null;
          const fresh = spell !== null && spell !== was[f.index] && f.act.t < 1 / 30;
          was[f.index] = spell;
          if (shut[f.index] && duel.time >= shut[f.index].until) shut[f.index] = null;
          if (!fresh) continue;
          const i = SCHOOLS[f.school].slots.findIndex((slot) => slot.includes(spell));
          if (i < 0) continue;
          expect(shut[f.index]?.slot === i && duel.round === shut[f.index].round).toBe(false);
          const slot = skillBar(f)[i];
          expect(slot.ready).toBe(false);
          expect(slot.full).toBeCloseTo(SPELLS[spell].cooldown + SPELLS[spell].windup, 5);
          shut[f.index] = { slot: i, round: duel.round, until: duel.time + slot.left - 1e-6 };
          checked++;
        }
      }
    }
    expect(checked).toBeGreaterThan(50);
  });

  test("the skill bar shows four slots, the last one the ultimate, charged by rage", () => {
    const duel = arena(3);
    for (const f of duel.fighters) {
      const bar = skillBar(f);
      expect(bar.map((s) => s.ultimate)).toEqual([false, false, false, true]);
      f.rage = RAGE - 1;
      expect(skillBar(f)[3]).toMatchObject({ charge: (RAGE - 1) / RAGE, ready: false });
      f.rage = RAGE;
      expect(skillBar(f)[3]).toMatchObject({ charge: 1, ready: true });
    }
  });

  test("every effect the duel plays takes the options it is given, with every effect in the engine", () => {
    // the hero's duel has the whole registry, so spells such as a dive's slam play effects PLAN2 leaves out
    const all = new Set(effects());
    const played = new Set();
    for (const seed of [1, 2, 3, 4, 5, 6, 7, 8]) {
      const duel = createDuel({ seed, left: 0, right: 220, ground: 150, has: (id) => all.has(id), Body, Mind });
      for (const c of run(duel, 90).commands) {
        if (c.type !== "play") continue;
        played.add(c.fx);
        new Fx(c.fx, c.opts).free();
      }
    }
    expect(played.has("explosions/dirt-explosion")).toBe(true);
    // twelve minutes of fighting, two brains thinking: about 4 s on an idle machine
  }, 30_000);
});
