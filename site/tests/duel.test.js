// The duel driven the way the hero drives it, step after step, playing what comes back: every effect it plays takes
// the options it is given. The rules themselves are crates/pixfx-duel's, tested there.
import { describe, expect, test } from "bun:test";
import { createDuel, damageOf, freeDuel, step } from "../assets/duel.js";
import { Fx, effects, pkg } from "./engine.js";

describe("duel", () => {
  test("every effect the duel plays takes the options it is given, with every effect in the engine", () => {
    const all = new Set(effects());
    const played = new Set();
    for (const seed of [1, 2, 3, 4, 5, 6, 7, 8]) {
      const duel = createDuel({ seed, left: 0, right: 220, ground: 150, sky: 96, has: (id) => all.has(id) });
      for (let i = 0; i < 90 * 60; i++) {
        for (const c of step(duel, 1 / 60)) {
          if (c.type !== "play") continue;
          played.add(c.fx);
          new Fx(c.fx, c.opts).free();
        }
      }
      freeDuel(duel);
    }
    expect(played.has("explosions/dirt-explosion")).toBe(true);
  }, 30_000);

  test("holding a corner that does not exist throws instead of doing nothing", () => {
    const duel = createDuel({ seed: 1, left: 0, right: 220, ground: 150, sky: 96, has: () => true });
    expect(() => duel.rs.hold(2, 1)).toThrow(/corner/);
    duel.rs.hold(1, 1);
    freeDuel(duel);
  });

  test("damageOf throws on a misspelt field instead of reading it as zero", () => {
    const duel = createDuel({ seed: 1, left: 0, right: 220, ground: 150, sky: 96, has: () => true });
    const [a, b] = duel.fighters;
    expect(damageOf(a, b, 10, "physical", false, 0.5)).toBeGreaterThan(0);
    expect(() => pkg.damageOf({ stats: a.stats, furry: 1 }, { stats: b.stats }, 10, "physical", false, 0.5)).toThrow(/furry/);
    freeDuel(duel);
  });
});
