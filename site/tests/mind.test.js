// pixfx-mind's `Mind` as JS gets it: built from plain data, fed a sense a tick, answering with thoughts.
import { describe, expect, test } from "bun:test";
import { Mind } from "./engine.js";

const profile = {
  aggression: 1.1, tempo: 2, spread: 0.8, range: 100, dry: 80, sky: 48, risk: 0.6, floor: 0.3, lead: 0.8, prior: 6,
  reaction: 0.16, think: 0.12, postureScale: [1, 1, 1, 1, 1, 1, 1, 1],
};
const body = { halfWidth: 6, chest: 20, body: 34, duck: 16, walk: 52 };
const skill = (name, kind, extra = {}) => ({
  name, kind, cost: 0, windup: 0.2, recover: 0.3, damage: 5, reach: 24, radius: 0, speed: 0, lands: 0, strikes: 1,
  ultimate: false, grounded: false, ...extra,
});
const skills = [skill("slash", "melee"), skill("bolt", "bolt", { cost: 18, speed: 230, damage: 13, reach: 0 })];
const me = {
  x: 40, air: 0, face: 1, hp: 100, qi: 100, rage: 0, rageFull: 4, flying: false, hover: 0, vx: 0, canAct: true,
  silenced: false, sealed: false, barrier: false, shield: false, seekerOut: false,
};
const foe = { x: 150, air: 0, flying: false, hp: 100, qi: 100, doing: { type: "idle" }, outFor: 0, openIn: 0, frozen: 0, shocked: 0, barrier: false, ready: [true] };
const sense = (time) => ({ time, me, foe, arena: { left: 0, right: 220, top: 96 }, threats: [], ready: [true, true] });

describe("Mind", () => {
  test("a mind thinks from a sense", () => {
    const mind = new Mind(profile, body, skills, 1);
    mind.meet([skill("slash", "melee")]);
    let thought = null;
    for (let i = 0; i < 60 && !thought; i++) thought = mind.think(sense(i / 60), 1 / 60);
    expect(thought).not.toBeNull();
    expect(["move", "jump", "duck", "guard", "fly", "meditate", "wait", "cast"]).toContain(thought.choice.type);
    expect(typeof thought.posture).toBe("string");
  });

  test("a misspelt sense field throws naming it", () => {
    const mind = new Mind(profile, body, skills, 1);
    const bad = { ...sense(0), me: { ...me, canAkt: true } };
    expect(() => mind.think(bad, 1 / 60)).toThrow(/canAkt|canAct/);
  });

  test("a misspelt skill kind throws", () => {
    expect(() => new Mind(profile, body, [skill("x", "sword")], 1)).toThrow(/kind/);
  });

  test("posture reads as a string, and a new round starts neutral", () => {
    const mind = new Mind(profile, body, skills, 1);
    for (let i = 0; i < 120; i++) mind.think(sense(i / 60), 1 / 60);
    mind.reset();
    expect(mind.posture).toBe("neutral");
  });

  test("tell takes an outcome", () => {
    const mind = new Mind(profile, body, skills, 1);
    mind.tell({ type: "landed", skill: "slash" });
    expect(() => mind.tell({ type: "hit", skill: "slash" })).toThrow();
  });

  test("explain fills why", () => {
    const mind = new Mind(profile, body, skills, 1);
    mind.explain(true);
    let thought = null;
    for (let i = 0; i < 60 && !thought; i++) thought = mind.think(sense(i / 60), 1 / 60);
    expect(thought.why.postures.length).toBe(8);
  });
});
