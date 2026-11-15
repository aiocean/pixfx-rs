// The duel driven the way the hero drives it, step after step, playing what comes back: every effect it plays takes
// the options it is given. The rules themselves are crates/pixfx-duel's, tested there.
import { describe, expect, test } from "bun:test";
import { createDuel, freeDuel, step } from "../assets/duel.js";
import { Fx, effects } from "./engine.js";

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
});
