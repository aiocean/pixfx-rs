// The trained policy on the page: wasm plays the same fights as the native rules (crates/pixfx-duel's
// goldens/policy_fight.json), and a decision costs well under a frame.
import { describe, expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { createDuel, freeDuel, step } from "../assets/duel.js";
import { effects } from "./engine.js";

const all = new Set(effects());
const golden = JSON.parse(readFileSync(new URL("../../crates/pixfx-duel/tests/goldens/policy_fight.json", import.meta.url), "utf8"));
const make = (seed, seats) => createDuel({ seed, left: 0, right: 220, ground: 150, sky: 96, has: (id) => all.has(id), seats });

describe("the trained policy", () => {
  test("the wasm policy fight equals the native one", () => {
    for (const seed of [1, 2, 3]) {
      const duel = make(seed);
      for (let i = 0; i < 60 * 60; i++) step(duel, 1 / 60);
      const got = {
        time: duel.time,
        round: duel.round,
        wins: duel.wins,
        fighters: duel.fighters.map((f) => [f.hp, f.qi, f.rage, f.x, f.air]),
      };
      expect(got).toEqual(golden[String(seed)]);
      freeDuel(duel);
    }
  });

  test("a decision costs under 0.2 ms in wasm", () => {
    // the same fight's ticks with the policy and with the first mind in both corners: what the policy adds, over the
    // decisions it makes (each fighter about every 0.12 s), is what a decision costs; best of three runs each
    const ticks = 120 * 60;
    const time = (seats) => {
      let best = Infinity;
      for (let rep = 0; rep < 3; rep++) {
        const duel = make(7, seats);
        const start = performance.now();
        for (let i = 0; i < ticks; i++) step(duel, 1 / 60);
        best = Math.min(best, performance.now() - start);
        freeDuel(duel);
      }
      return best;
    };
    const extra = time(["policy", "policy"]) - time(["first", "first"]);
    const decisions = (2 * ticks) / 60 / 0.12;
    console.log(`policy decision in wasm: ${(extra / decisions).toFixed(3)} ms over ${decisions} decisions`);
    expect(extra / decisions).toBeLessThan(0.2);
  }, 30_000);

  test("each fighter shows the style it fights in", () => {
    const duel = make(3);
    for (const f of duel.fighters) expect(["aggressive", "balanced", "defensive"]).toContain(f.style);
    freeDuel(duel);
  });
});
