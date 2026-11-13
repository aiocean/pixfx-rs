import { expect, mock, test } from "bun:test";
import EventEmitter from "eventemitter3";
import type { Backend } from "../src/backend";
import type { TimedCue } from "../src/protocol";
import { InlineBackend } from "../src/inline";
import { FakeEngine } from "./fake-engine";
import { FakeGfx } from "./fake-gfx";

void mock.module("phaser", () => ({ default: { Events: { EventEmitter } } }));
const { EffectManager } = await import("../src/effects");
const { PixfxFeel } = await import("../src/feel");

/** A hit 0.1 s in: a hit-stop of 0.05 s, full trauma, a ripple 3 px right of the effect, a flash, a sound. */
const HIT: TimedCue[] = [
  { t: 0.1, pan: null, cue: { type: "hitstop", dur: 0.05 } },
  { t: 0.1, pan: null, cue: { type: "shake", trauma: 1 } },
  { t: 0.1, pan: null, cue: { type: "ripple", x: 3, y: -2, r0: 0, r1: 9, dur: 0.3, amp: 1 } },
  { t: 0.1, pan: null, cue: { type: "flash", k: 0.5, dur: 0.1 } },
  { t: 0.1, pan: null, cue: { type: "sound", name: "boom", gain: 1, pitch: 1, len: null, x: null } },
];

function setup() {
  const engine = new FakeEngine();
  engine.schedule = HIT;
  const backend = new InlineBackend(engine);
  const feel = new PixfxFeel(() => engine);
  const fx = new EffectManager(new FakeGfx(), (): Backend => backend, "pixfx:test:", feel);
  return { engine, feel, fx };
}

test("an effect's hit-stop freezes every effect in the scene and tells the game through timeScale", () => {
  const { engine, feel, fx } = setup();
  fx.play("fake/spark", 50, 20);
  engine.schedule = [];
  fx.play("fake/spark", 0, 0);
  for (let i = 0; i < 7; i++) fx.update(16, 1); // past 0.1 s: the hit lands
  const times = () => engine.created.map((c) => c.elapsed);
  const at = times();
  fx.update(16, 1);
  expect(times()).toEqual(at);
  expect(feel.timeScale).toBe(0);
  expect(feel.frozen).toBe(true);
  for (let i = 0; i < 4; i++) fx.update(16, 1);
  expect(feel.timeScale).toBe(1);
  expect(times().every((t, i) => t > (at[i] ?? 0))).toBe(true);
});

test("cues come out as events, on the effect and on the feel, placed in the world", () => {
  const { feel, fx } = setup();
  const effect = fx.play("fake/spark", 50, 20);
  const onEffect: TimedCue[] = [];
  const onFeel: TimedCue[] = [];
  effect.on("cue", (c: TimedCue) => onEffect.push(c));
  feel.on("cue", (c: TimedCue) => onFeel.push(c));
  for (let i = 0; i < 7; i++) fx.update(16, 1);
  expect(onFeel).toEqual(onEffect);
  expect(onEffect.map((c) => c.cue.type)).toEqual(["hitstop", "shake", "ripple", "flash", "sound"]);
  const ripple = onEffect.find((c) => c.cue.type === "ripple")?.cue;
  expect(ripple).toEqual({ type: "ripple", x: 53, y: 18, r0: 0, r1: 9, dur: 0.3, amp: 1 });
  const sound = onEffect.find((c) => c.cue.type === "sound")?.cue;
  expect(sound).toMatchObject({ name: "boom", x: 50 });
});

test("the player can turn shake and flash down or off", () => {
  const { feel, fx } = setup();
  fx.play("fake/spark", 0, 0);
  for (let i = 0; i < 7; i++) fx.update(16, 1);
  expect(feel.shake).toEqual([1, -1]);
  expect(feel.flash).toBe(0.5);
  feel.setShakeStrength(0).setFlashStrength(0.5);
  expect(feel.shake).toEqual([0, 0]);
  expect(feel.flash).toBe(0.25);
});

test("off, effects play every second and the game still gets every cue", () => {
  const { engine, feel, fx } = setup();
  feel.setEnabled(false);
  fx.play("fake/spark", 0, 0);
  const types: string[] = [];
  feel.on("cue", (c: TimedCue) => types.push(c.cue.type));
  for (let i = 0; i < 10; i++) fx.update(16, 1);
  expect(engine.created[0]?.elapsed).toBeCloseTo(0.16, 9);
  expect(types).toHaveLength(5);
  expect(feel.shake).toEqual([0, 0]);
  expect(feel.flash).toBe(0);
  expect(feel.timeScale).toBe(1);
  expect(engine.feels.every((f) => f.taken.length === 0)).toBe(true);
});

test("effects in a layer fire their cues too", () => {
  const { feel, fx } = setup();
  const layer = fx.createLayer({ bounds: { x: 0, y: 0, width: 40, height: 40 } });
  layer.play("fake/spark", 10, 10);
  let n = 0;
  feel.on("cue", () => (n += 1));
  for (let i = 0; i < 7; i++) fx.update(16, 1);
  expect(n).toBe(5);
});

test("the game's own impacts join the effects' answer", () => {
  const { engine, feel, fx } = setup();
  engine.schedule = [];
  fx.play("fake/spark", 0, 0);
  fx.update(16, 1);
  feel.freeze(0.1).addTrauma(0.3);
  fx.update(16, 1);
  expect(feel.timeScale).toBe(0);
  expect(feel.trauma).toBe(0.3);
});
