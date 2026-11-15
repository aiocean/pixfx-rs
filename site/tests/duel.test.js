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
  test("fights end in knockouts and each school wins at least 35% of rounds", () => {
    // the whole engine, as the landing page has it: with fewer effects a school loses the spells they play
    const every = new Set(effects());
    const wins = [0, 0];
    for (let seed = 1; seed <= 40; seed++) {
      const { rounds } = run(createDuel({ seed, left: 0, right: 220, ground: 150, has: (id) => every.has(id), Body, Mind }), 120);
      expect([seed, rounds.length > 0]).toEqual([seed, true]);
      const [last] = rounds.slice(-1);
      wins[0] += last[0];
      wins[1] += last[1];
    }
    const total = wins[0] + wins[1];
    console.log(`rounds won over seeds 1-40: thunder ${wins[0]}, frost ${wins[1]}`);
    for (const w of wins) expect(w / total).toBeGreaterThanOrEqual(0.35);
  }, 600_000);

  test("a cast puts its spell on its own cooldown, and a bar skill's one cast empties its slot", () => {
    let checked = 0;
    for (const seed of [1, 2, 3, 4]) {
      const duel = arena(seed);
      const was = [null, null];
      for (let i = 0; i < 90 * 60; i++) {
        const before = duel.fighters.map((f) => f.bar.map((slot) => ({ ...slot })));
        step(duel, 1 / 60);
        for (const f of duel.fighters) {
          const spell = f.act?.spell ?? null;
          const fresh = spell !== null && spell !== was[f.index] && f.act.t < 1 / 30;
          was[f.index] = spell;
          if (!fresh) continue;
          const j = before[f.index].findIndex((slot) => slot.spell === spell);
          // a swift skill's cooldown is cut to 60%
          const swift = j >= 0 && before[f.index][j].traits?.includes("swift") ? 0.6 : 1;
          expect(f.coolFull[spell]).toBeCloseTo(SPELLS[spell].cooldown * swift + SPELLS[spell].windup, 5);
          if (j < 0) continue;
          const after = f.bar[j];
          const prior = before[f.index][j];
          // one cast empties its slot, unless its partner on the bar refills it evolved, or a lucky Again keeps it
          const refilled = after.evolved && !prior.evolved;
          const again = f.lucky?.kind === "again" && f.lucky.spell === spell && after.spell === spell;
          expect(after.empty !== undefined || refilled || again).toBe(true);
          checked++;
        }
      }
    }
    expect(checked).toBeGreaterThan(30);
  }, 30_000);

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

describe("whole ticks", () => {
  /** Duel seed 5 stepped at `hz` frames a second for 60 s: what decides the fight. */
  const fought = (hz) => {
    const duel = arena(5);
    for (let i = 0; i < 60 * hz; i++) step(duel, 1 / hz);
    return { wins: duel.wins, round: duel.round, fighters: duel.fighters.map((f) => [f.hp, f.x]) };
  };

  test("a seed fights the same at 60 Hz and 144 Hz", () => {
    expect(fought(144)).toEqual(fought(60));
  });

  test("step(0) moves nothing", () => {
    const duel = arena(5);
    for (let i = 0; i < 90; i++) step(duel, 1 / 60);
    const before = { time: duel.time, x: duel.fighters.map((f) => f.x), hp: duel.fighters.map((f) => f.hp) };
    expect(step(duel, 0)).toEqual([]);
    expect({ time: duel.time, x: duel.fighters.map((f) => f.x), hp: duel.fighters.map((f) => f.hp) }).toEqual(before);
  });
});
