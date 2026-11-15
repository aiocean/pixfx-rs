// The proof that the duel's rules in Rust (crates/pixfx-duel, through the wasm `Duel`) are duel.js's: the same seed
// fought by both, step by step, gives the same commands and the same fighters. Both compute with libm (`useMath`).
// Deleted with the JS rules (plan 2026-11-15-duel-rules-in-rust, Task 8).
import { describe, expect, test } from "bun:test";
import { RAGE, SPELLS, STALE, STATS, TIERS, TRAITS, DECOR, buffsOf, createDuel, damageOf, orbOffset, skillBar, step, useMath } from "../assets/duel.js";
import { Body, Duel, Mind, damageOf as rsDamageOf, duelConstants, duelSpells, duelStats, duelTraits, effects, libmHypot, libmSin, libmSqrt, orbOffset as rsOrbOffset } from "./engine.js";
import { baselineMind } from "./mind-baseline.js";

useMath({ sin: libmSin, hypot: libmHypot, sqrt: libmSqrt });

const EFFECTS = effects();
const registry = new Set(EFFECTS);
const ARENA = { left: 0, right: 220, ground: 150, sky: 96 };
const SECONDS = 120;

/** A JS fighter in the shape of the wasm view's fighters. */
function fighterView(f) {
  return {
    index: f.index, school: f.school, hp: f.hp, qi: f.qi, rage: f.rage, x: f.x, air: f.air, vx: f.vx, vy: f.vy,
    face: f.face, pose: f.pose, poseT: f.poseT, flash: f.flash, hitT: f.hitT, hitPower: f.hitPower, frozen: f.frozen,
    shocked: f.shocked, dodged: f.dodged, rang: f.rang, charges: f.charges, barrier: f.barrier, burn: !!f.burn,
    lucky: f.lucky && { kind: f.lucky.kind, spell: f.lucky.spell, slot: f.lucky.slot, at: f.lucky.at },
    act: f.act && { spell: f.act.spell, phase: f.act.phase, t: f.act.t }, stats: f.stats,
    untouchable: f.body.untouchable(), skillBar: skillBar(f), buffs: buffsOf(f),
  };
}

/** A JS duel in the shape of the wasm view. */
function duelView(d) {
  return { time: d.time, round: d.round, wins: d.wins, winner: d.winner, left: d.left, right: d.right, ground: d.ground, fighters: d.fighters.map(fighterView) };
}

/** The path to the first place `a` and `b` differ, or null. */
function firstDiff(a, b, path = "") {
  if (Object.is(a, b)) return null;
  if (typeof a !== "object" || typeof b !== "object" || a === null || b === null) return `${path}: ${JSON.stringify(a)} vs ${JSON.stringify(b)}`;
  for (const k of new Set([...Object.keys(a), ...Object.keys(b)])) {
    if (a[k] === undefined && b[k] === undefined) continue;
    const d = firstDiff(a[k], b[k], `${path}.${k}`);
    if (d) return d;
  }
  return null;
}

/** Fight `seed` in both for SECONDS; the first difference, named, or null. */
function replay(seed, seats) {
  const minds = seats[0] === "first" ? [baselineMind, baselineMind] : [];
  const js = createDuel({ seed, ...ARENA, has: (id) => registry.has(id), Body, Mind, minds });
  const rs = new Duel({ seed, ...ARENA, effects: EFFECTS }, seats);
  try {
    let d = firstDiff(duelView(js), rs.view(), "view");
    if (d) return `seed ${seed} at 0: ${d}`;
    for (let i = 1; i <= SECONDS * 60; i++) {
      d = firstDiff(step(js, 1 / 60), rs.step(1 / 60), "out") ?? firstDiff(duelView(js), rs.view(), "view");
      if (d) return `seed ${seed} at ${(i / 60).toFixed(3)}s: ${d}`;
    }
    return null;
  } finally {
    rs.free();
  }
}

describe("the duel's rules in Rust replay duel.js", () => {
  test("its tables are duel.js's", () => {
    expect(duelSpells()).toEqual(SPELLS);
    expect(duelTraits()).toEqual(TRAITS);
    expect(duelConstants()).toEqual({ RAGE, STALE, DECOR, TIERS });
    expect(duelStats()).toEqual(STATS);
  });

  test("damageOf and orbOffset are duel.js's", () => {
    for (const [a, b] of [["thunder", "frost"], ["frost", "thunder"], ["frost", "frost"]]) {
      for (const [fury, defDown, power, type, crit, roll] of [[0, 0, 20, "magic", false, 0.5], [3, 2, 15, "physical", true, 0.9], [5, 0, 0.1, "magic", false, 0]]) {
        const by = { stats: STATS[a], fury };
        const f = { stats: STATS[b], defDown };
        expect(rsDamageOf(by, f, power, type, crit, roll)).toBe(damageOf(by, f, power, type, crit, roll));
      }
    }
    for (let t = 0; t < 8; t += 0.37) expect(rsOrbOffset(t)).toEqual(orbOffset(t));
  });

  // one test a seed, so a slow machine never times a whole sweep out
  for (let seed = 1; seed <= 200; seed++) {
    test(`seed ${seed}, two brains, ${SECONDS} s`, () => expect(replay(seed, ["brain", "brain"])).toBeNull(), 60_000);
  }
  for (let seed = 1; seed <= 20; seed++) {
    test(`seed ${seed}, two first minds, ${SECONDS} s`, () => expect(replay(seed, ["first", "first"])).toBeNull(), 60_000);
  }
});
