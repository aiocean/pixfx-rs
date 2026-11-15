// The bout an effect's page stages, driven the way the page drives it: a take of the effect, step after step at
// 60 fps, the effects played on a stand-in stage of real engine instances whose cues go back to `impact`.
import { describe, expect, test } from "bun:test";
import { readFileSync, readdirSync } from "node:fs";
import { TAKE_MAX, createBout, impact, step, take } from "../assets/bout.js";
import { ROLES, roleFor } from "../assets/effect-roles.js";
import { Body, Fx, effects } from "./engine.js";
// engine.js has loaded the wasm package; the same module gives the effects' meta
const { meta } = await import(new URL("../../packages/pixfx-web/wasm/pixfx_wasm.js", import.meta.url).href);

/** Every effect id the engine's source declares, built into the test's wasm yet or not. */
function sourceIds() {
  const root = new URL("../../crates/pixfx-effects/src/", import.meta.url);
  const ids = new Set();
  for (const dir of readdirSync(root, { withFileTypes: true })) {
    if (!dir.isDirectory()) continue;
    for (const file of readdirSync(new URL(`${dir.name}/`, root))) {
      const m = readFileSync(new URL(`${dir.name}/${file}`, root), "utf8").match(/^\s*id: "([a-z-]+\/[a-z-]+)"/m);
      if (m) ids.add(m[1]);
    }
  }
  return ids;
}

const BUILT = new Set(effects());
const ALL = [...new Set([...sourceIds(), ...BUILT])].sort();
/** Roles in which the effect (or the blow it gives the caster) strikes the other fighter. */
const HITTING = new Set(["strike", "projectile", "stream", "buff", "ground-self", "ground-foe", "flight", "blink", "ambient", "death"]);

const arena = (seed) => createBout({ left: 20, right: 300, ground: 150, Body, seed });

/**
 * Run one take of `id` to the end the way the page does, at 60 fps. `cues`: let the effects' own cues mark their
 * impacts (an effect not in the built engine plays without an instance, so its blows land at their own times);
 * a number passes on only the cues of that many frames, the first ones. Returns every command with the frame it
 * came out in (`at`), the frames a cue was passed on in (`cued`), and the seconds each take took.
 */
function run(id, { seed = 3, takes = 1, cues = true, opts = {} } = {}) {
  const pass = cues === true ? Infinity : cues === false ? 0 : cues;
  const at = [];
  const cued = [];
  let frame = 0;
  const bout = arena(seed);
  const m = BUILT.has(id) ? meta(id) : undefined;
  const role = roleFor(id, m);
  const live = new Map();
  const commands = [];
  const lengths = [];
  for (let n = 0; n < takes; n++) {
    take(bout, id, role, opts);
    let i = 0;
    for (; i < TAKE_MAX * 60 + 60 && !bout.done; i++) {
      const out = step(bout, 1 / 60);
      frame++;
      commands.push(...out);
      for (const _ of out) at.push(frame);
      for (const c of out) {
        if (c.type === "play" && BUILT.has(c.fx)) live.set(c.key, new Fx(c.fx, c.opts));
        else if (c.type === "stop" && live.has(c.key)) {
          live.get(c.key).free();
          live.delete(c.key);
        }
      }
      for (const [key, fx] of live) {
        fx.update(1 / 60);
        if (fx.takeCues().length > 0 && cued.length < pass) {
          impact(bout, key);
          cued.push(frame);
        }
        if (fx.done()) {
          fx.free();
          live.delete(key);
        }
      }
    }
    lengths.push(i / 60);
  }
  for (const fx of live.values()) fx.free();
  return { bout, role, commands, lengths, at, cued };
}

describe("effect roles", () => {
  test("the engine's source and its wasm build agree on the effects this test knows", () => {
    expect(ALL.length).toBeGreaterThanOrEqual(BUILT.size);
    expect(ALL).toContain("explosions/plasma-explosion");
  });

  test.each(ALL)("%s has a known role", (id) => {
    const { role } = roleFor(id, BUILT.has(id) ? meta(id) : undefined);
    expect(ROLES).toContain(role);
  });

  test("an effect the table does not know is placed by its tags, and anything else strikes", () => {
    expect(roleFor("new/arrow", { tags: ["projectile"], loop: true }).role).toBe("projectile");
    expect(roleFor("new/rain", { tags: ["weather", "ambient"] }).role).toBe("ambient");
    expect(roleFor("new/thing", { tags: [] }).role).toBe("strike");
    expect(roleFor("new/thing").role).toBe("strike");
  });
});

describe("bout", () => {
  test.each(ALL)("a take of %s plays it, lands its blow and settles back to rest", (id) => {
    for (const cues of [true, false]) {
      const { bout, role, commands, lengths } = run(id, { cues });
      const caster = 0;
      expect(commands.some((c) => c.type === "play" && c.fx === id)).toBe(true);
      if (HITTING.has(role.role)) {
        const by = role.role === "ambient" ? 1 - caster : caster;
        expect(commands.some((c) => c.type === "hit" && c.by === by)).toBe(true);
      }
      if (role.role === "ward") expect(commands.filter((c) => c.type === "block").length).toBe(role.blows.length);
      expect(bout.done).toBe(true);
      expect(lengths[0]).toBeLessThanOrEqual(TAKE_MAX);
      for (const f of bout.fighters) {
        expect(f.down).toBe(null);
        expect(f.act).toBe(null);
        expect(f.air).toBeLessThan(0.5);
      }
      // every play is in screen rows inside a sane band around the arena, and a loop the take played is stopped
      for (const c of commands.filter((c) => c.type === "play")) expect(c.y).toBeGreaterThan(-100);
      if (role.loop) {
        const played = commands.filter((c) => c.type === "play").map((c) => c.key);
        const stopped = new Set(commands.filter((c) => c.type === "stop").map((c) => c.key));
        for (const key of played) expect(stopped.has(key)).toBe(true);
      }
    }
  });

  test("a blow with a cue lands the frame after its cue; with no cue, at the effect's own time and a little more", () => {
    // icicle-fall drops at 0.944 s (duel.js SPELLS.icicle); the bout's fallback lands it 0.15 s after that
    const frameOf = (r, is) => r.at[r.commands.findIndex(is)];
    const play = (c) => c.type === "play" && c.fx === "ice/icicle-fall";
    const hit = (c) => c.type === "hit";
    const cued = run("ice/icicle-fall");
    expect(cued.cued.length).toBeGreaterThan(0);
    // impact() queues the blow: it comes out of the step after the cue
    expect(frameOf(cued, hit) - cued.cued[0]).toBe(1);
    const uncued = run("ice/icicle-fall", { cues: false });
    expect(uncued.cued.length).toBe(0);
    const late = (frameOf(uncued, hit) - frameOf(uncued, play)) / 60;
    expect(late).toBeGreaterThanOrEqual(0.944 + 0.15 - 1 / 60);
    expect(late).toBeLessThanOrEqual(0.944 + 0.15 + 2 / 60);
    // and the cue is what makes it land sooner
    expect(frameOf(cued, hit) - frameOf(cued, play)).toBeLessThan(frameOf(uncued, hit) - frameOf(uncued, play));
  });

  test("blows whose cues never come still land at their own times", () => {
    // only the first of hex-shield's four blows is cued; the other three land on the bout's own clock
    const { commands, cued, at, lengths } = run("arcane/hex-shield", { cues: 1 });
    expect(cued.length).toBe(1);
    const blocks = commands.map((c, i) => [c, at[i]]).filter(([c]) => c.type === "block");
    expect(blocks.length).toBe(4);
    expect(blocks[0][1]).toBe(cued[0] + 1);
    expect(lengths[0]).toBeLessThan(TAKE_MAX - 2);
  });

  test("a hex shield turns aside each of its four blows exactly once, cued or not", () => {
    for (const cues of [true, false]) {
      const { commands } = run("arcane/hex-shield", { cues });
      expect(commands.filter((c) => c.type === "block").length).toBe(4);
      expect(commands.some((c) => c.type === "hit")).toBe(false);
    }
  });

  test("takes alternate casters: Thunder, then Frost", () => {
    const { commands } = run("projectiles/laser-bolt", { takes: 2 });
    const hits = commands.filter((c) => c.type === "hit").map((c) => c.by);
    expect(hits).toEqual([0, 1]);
    // Frost on the right shoots leftward: the bolt is flipped
    const plays = commands.filter((c) => c.type === "play");
    expect(plays.map((c) => !!c.opts.flip)).toEqual([false, true]);
  });

  test("a take may name its caster; without one, the turns go on from whoever cast last", () => {
    const laser = roleFor("projectiles/laser-bolt", meta("projectiles/laser-bolt"));
    const casters = (picks) => {
      const bout = arena(1);
      return picks.map((caster) => {
        take(bout, "projectiles/laser-bolt", laser, {}, caster === undefined ? {} : { caster });
        return bout.caster;
      });
    };
    // a control changed twice while Thunder's take plays: Thunder casts again both times
    expect(casters([0, 0])).toEqual([0, 0]);
    // without a caster the two take turns, Thunder first, as before
    expect(casters([undefined, undefined, undefined, undefined])).toEqual([0, 1, 0, 1]);
    // the automatic replay after a named caster goes to the other fighter
    expect(casters([1, undefined, 1, 1, undefined])).toEqual([1, 0, 1, 1, 0]);
    // and the one named is the one who strikes
    const bout = arena(1);
    const hits = [];
    for (let n = 0; n < 2; n++) {
      take(bout, "projectiles/laser-bolt", laser, {}, { caster: 0 });
      for (let i = 0; i < TAKE_MAX * 60 && !bout.done; i++) hits.push(...step(bout, 1 / 60).filter((c) => c.type === "hit"));
    }
    expect(hits.map((c) => c.by)).toEqual([0, 0]);
    expect(() => take(bout, "projectiles/laser-bolt", laser, {}, { caster: 2 })).toThrow("caster must be 0 or 1");
  });

  test("a body burst plays on the fighter the caster strikes down, not on the caster", () => {
    const { commands } = run("explosions/body-burst");
    const play = commands.find((c) => c.type === "play" && c.fx === "explosions/body-burst");
    const hit = commands.find((c) => c.type === "hit");
    // Thunder (0) casts; the burst takes Frost's (1) body, where Frost stands as the blow lands
    expect(play?.by).toBe(1);
    expect(hit?.by).toBe(0);
    expect(commands.indexOf(play)).toBeLessThan(commands.indexOf(hit));
    const bout = arena(3);
    take(bout, "explosions/body-burst", roleFor("explosions/body-burst", meta("explosions/body-burst")));
    expect(bout.caption).toBe("Thunder strikes Frost down, and Frost's body bursts apart");
  });

  test("only an effect that draws the body itself takes the fighter's place", () => {
    expect(meta("explosions/body-burst").replacesActor).toBe(true);
    expect(meta("feedback/outline").replacesActor).toBe(false);
    expect(meta("explosions/plasma-explosion").replacesActor).toBe(false);
  });

  test("a take says what it shows, the caster first", () => {
    const bout = arena(1);
    take(bout, "projectiles/laser-bolt", roleFor("projectiles/laser-bolt", meta("projectiles/laser-bolt")));
    expect(bout.caption).toBe("Thunder throws it at Frost");
    take(bout, "projectiles/laser-bolt", roleFor("projectiles/laser-bolt", meta("projectiles/laser-bolt")));
    expect(bout.caption).toBe("Frost throws it at Thunder");
  });

  test("a screen wipe comes after the spar's blow, ending the scene", () => {
    const { commands } = run("transitions/crt-off");
    const struck = commands.findIndex((c) => c.type === "hit");
    const wiped = commands.findIndex((c) => c.type === "play" && c.fx === "transitions/crt-off");
    expect(struck).toBeGreaterThanOrEqual(0);
    expect(wiped).toBeGreaterThan(struck);
  });

  test("the page's options reach the effect", () => {
    const { commands } = run("explosions/small-explosion", { opts: { size: 1.5, seed: 9 } });
    const play = commands.find((c) => c.type === "play");
    expect(play.opts).toMatchObject({ size: 1.5, seed: 9 });
  });

  test("a new take cuts the one playing short and stops what it played", () => {
    const bout = arena(1);
    take(bout, "water/water-jet", roleFor("water/water-jet", meta("water/water-jet")));
    const first = [];
    for (let i = 0; i < 90; i++) first.push(...step(bout, 1 / 60));
    const jet = first.find((c) => c.type === "play");
    expect(jet).toBeDefined();
    take(bout, "spawn/poof", roleFor("spawn/poof", meta("spawn/poof")));
    expect(step(bout, 1 / 60).some((c) => c.type === "stop" && c.key === jet.key)).toBe(true);
  });

  test("a seed replays the same takes", () => {
    for (const id of ["explosions/implosion", "lightning/ball-lightning", "fire/campfire", "scifi/jetpack"]) {
      const a = JSON.stringify(run(id, { seed: 5, takes: 3 }).commands);
      const b = JSON.stringify(run(id, { seed: 5, takes: 3 }).commands);
      expect(b).toBe(a);
    }
  });
});
