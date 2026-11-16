// The proof that the page's fighters are smart: the trained policy, in the styles each fight draws, against the
// utility brain and the duel's first mind. Each seed is fought twice, the policy as Thunder and then as Frost, so the
// schools' own balance cancels out. The full bars, per style over 150 seeds, are the Rust ladder's
// (`cargo run --release -p pixfx-duel --example ladder`); docs/superpowers/specs/2026-11-15-duel-rules-in-rust-design.md.
import { describe, expect, test } from "bun:test";
import { createDuel, freeDuel, step } from "../assets/duel.js";
import { effects } from "./engine.js";

const SEEDS = Array.from({ length: 20 }, (_, i) => i + 1);
const ROUNDS = 3;
const LIMIT = 300;
const all = new Set(effects());

/** Fight `seed` with the policy in corner `me` (0 Thunder, 1 Frost) against `foe` until ROUNDS rounds end or LIMIT s. */
function fight(seed, me, foe) {
  const seats = [foe, foe];
  seats[me] = "policy";
  const duel = createDuel({ seed, left: 0, right: 220, ground: 150, sky: 96, has: (id) => all.has(id), seats });
  let round = duel.round;
  let started = 0;
  const lengths = [];
  for (let i = 0; i < LIMIT * 60 && duel.wins[0] + duel.wins[1] < ROUNDS; i++) {
    step(duel, 1 / 60);
    if (duel.round !== round) {
      lengths.push(duel.time - started);
      started = duel.time;
      round = duel.round;
    }
  }
  const out = { won: duel.wins[me], lost: duel.wins[1 - me], lengths };
  freeDuel(duel);
  return out;
}

/** The policy's share of the rounds against `foe`, and the mean round length. */
function match(foe) {
  let won = 0;
  let lost = 0;
  const lengths = [];
  for (const seed of SEEDS) {
    for (const me of [0, 1]) {
      const r = fight(seed, me, foe);
      won += r.won;
      lost += r.lost;
      lengths.push(...r.lengths);
    }
  }
  const mean = lengths.reduce((a, b) => a + b, 0) / lengths.length;
  console.log(`policy vs ${foe}: won ${won} of ${won + lost} rounds (${((won / (won + lost)) * 100).toFixed(1)}%), mean round ${mean.toFixed(1)} s`);
  return { rate: won / (won + lost), rounds: won + lost, mean };
}

describe("the trained policy", () => {
  test("wins at least 65% of the rounds against the brain, in rounds that end", () => {
    const r = match("brain");
    expect(r.rounds).toBe(SEEDS.length * 2 * ROUNDS);
    expect(r.rate).toBeGreaterThanOrEqual(0.65);
    expect(r.mean).toBeLessThan(45);
  }, 600_000);

  test("wins at least 75% of the rounds against the first mind", () => {
    const r = match("first");
    expect(r.rounds).toBe(SEEDS.length * 2 * ROUNDS);
    expect(r.rate).toBeGreaterThanOrEqual(0.75);
  }, 600_000);
});
