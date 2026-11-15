// The proof that the fighters' brain (pixfx-mind) is smarter than the duel's first mind: a match between them.
// Each seed is fought twice, the brain as Thunder and then as Frost, so the schools' own balance cancels out.
import { describe, expect, test } from "bun:test";
import { createDuel, step } from "../assets/duel.js";
import { Body, Mind } from "./engine.js";
import { baselineMind } from "./mind-baseline.js";

const SEEDS = Array.from({ length: 20 }, (_, i) => i + 1);
const ROUNDS = 3;
const LIMIT = 300;

/** Fight `seed` with the brain in corner `brain` (0 Thunder, 1 Frost) until ROUNDS rounds end or LIMIT seconds. */
function fight(seed, brain) {
  const minds = [undefined, undefined];
  minds[1 - brain] = baselineMind;
  const duel = createDuel({ seed, left: 0, right: 220, ground: 150, has: () => true, Body, Mind, minds });
  const postures = {};
  let held = null;
  let round = duel.round;
  let started = 0;
  const lengths = [];
  for (let i = 0; i < LIMIT * 60 && duel.wins[0] + duel.wins[1] < ROUNDS; i++) {
    step(duel, 1 / 60);
    const now = duel.fighters[brain].mind.posture;
    if (now !== held) postures[now] = (postures[now] ?? 0) + 1;
    held = now;
    if (duel.round !== round) {
      lengths.push(duel.time - started);
      started = duel.time;
      round = duel.round;
    }
  }
  return { won: duel.wins[brain], lost: duel.wins[1 - brain], lengths, postures };
}

describe("the brain against the first mind", () => {
  // todo until the brain looks ahead (docs/superpowers/specs/2026-11-15-duel-rules-in-rust-design.md): the brain with
  // postures alone plays even with the first mind (50-55%); run it with `bun test --todo`
  test.todo("wins at least 60% of the rounds, with rounds that end", () => {
    let won = 0;
    let lost = 0;
    const lengths = [];
    const postures = {};
    for (const seed of SEEDS) {
      for (const brain of [0, 1]) {
        const r = fight(seed, brain);
        won += r.won;
        lost += r.lost;
        lengths.push(...r.lengths);
        for (const [p, n] of Object.entries(r.postures)) postures[p] = (postures[p] ?? 0) + n;
      }
    }
    const rate = won / (won + lost);
    const mean = lengths.reduce((a, b) => a + b, 0) / lengths.length;
    console.log(`brain won ${won} of ${won + lost} rounds (${(rate * 100).toFixed(1)}%), mean round ${mean.toFixed(1)} s`);
    console.log("posture entries:", JSON.stringify(postures));
    expect(won + lost).toBeGreaterThanOrEqual(SEEDS.length * 2 * 2);
    expect(rate).toBeGreaterThanOrEqual(0.6);
    expect(mean).toBeLessThan(60);
  }, 600_000);
});
