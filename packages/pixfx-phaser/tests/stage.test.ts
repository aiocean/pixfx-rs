import { expect, mock, test } from "bun:test";
import EventEmitter from "eventemitter3";
import type { WorkerPort } from "../src/game";
import type { WorldCue } from "../src/stage";
import { FakeGfx } from "./fake-gfx";
import { memory, wasm } from "./wasm";

// Phaser's runtime needs a DOM. Phaser.Events.EventEmitter is a thin wrapper over eventemitter3, the only Phaser
// value the shell code below the plugin uses, so the test supplies exactly that.
void mock.module("phaser", () => ({ default: { Events: { EventEmitter } } }));
const { Game } = await import("../src/game");
const { Stage } = await import("../src/stage");
const { PixfxFeel } = await import("../src/feel");
const { parsePlayOptions } = await import("../src/options");

type GameT = InstanceType<typeof Game>;
type StageT = InstanceType<typeof Stage>;

function setup(ports: WorkerPort[] = []) {
  const gfx = new FakeGfx();
  const game = new Game(wasm, memory, ports);
  game.useTextures(gfx.texture);
  return { gfx, game, ...scene(game, gfx) };
}

/** Another scene of `game`, with its feel. */
function scene(game: GameT, gfx: FakeGfx) {
  const cues: WorldCue[] = [];
  let stage: StageT | null = null;
  const feel = new PixfxFeel(() => stage?.scene ?? null);
  feel.on("cue", (c: WorldCue) => cues.push(c));
  stage = new Stage(game, gfx.image, (c, e) => feel.take(c, e));
  return { stage, feel, cues };
}

/** One host frame: `dt` seconds pass, the scene draws, the frame ends. Returns the seconds played. */
function frame(game: GameT, stage: StageT, dt = 1 / 30): number {
  const played = stage.update(dt);
  stage.draw();
  game.endFrame();
  return played;
}

/** Where every image shown now sits. */
function places(gfx: FakeGfx): [number, number][] {
  return gfx.showing().map((i) => [i.shown?.x ?? Number.NaN, i.shown?.y ?? Number.NaN]);
}

test("an effect shows its colour normally and its light added over it, at its lit pixels near where it plays", () => {
  const { gfx, game, stage } = setup();
  stage.play("fire/candle", 30, 40, {});
  frame(game, stage);
  const shown = gfx.picture();
  expect(shown.length).toBeGreaterThan(0);
  expect(new Set(shown.map((s) => s.blend))).toEqual(new Set(["normal", "add"]));
  for (const s of shown) {
    expect(s.bytes.length).toBe(s.at.w * s.at.h * 4);
    expect(s.bytes.some((b) => b !== 0)).toBe(true);
    // a candle's flame sits on its origin
    expect(s.at.x).toBeLessThanOrEqual(30);
    expect(s.at.x + s.at.w).toBeGreaterThan(30);
    expect(s.at.y).toBeLessThan(40);
  }
});

test("moving an effect moves its images by as much, showing the same frame", () => {
  const { gfx, game, stage } = setup();
  const fx = stage.play("fire/candle", 30, 40, {});
  frame(game, stage);
  const before = gfx.picture();
  fx.setPosition(40, 45);
  frame(game, stage, 0);
  const after = gfx.picture();
  expect(after.map((s) => s.bytes)).toEqual(before.map((s) => s.bytes));
  expect(after.map((s) => [s.at.x - 10, s.at.y - 5])).toEqual(before.map((s) => [s.at.x, s.at.y]));
});

test("an effect's back and front depths and its scroll factor are the play's", () => {
  const { gfx, game, stage } = setup();
  const p = parsePlayOptions({ depth: { back: 5, front: 7 }, scrollFactor: 0.5 });
  // a rune circle draws behind and in front of what stands in it
  stage.play("arcane/rune-circle", 30, 40, p.engine, p.depth, p.scrollFactor);
  frame(game, stage);
  const shown = gfx.showing().map((i) => i.shown);
  expect(new Set(shown.map((s) => s?.depth))).toEqual(new Set([5, 7]));
  expect(shown.every((s) => s?.scrollFactor === 0.5)).toBe(true);
});

test("a followed effect tracks its target, stops when the target goes inactive, and completes once faded", () => {
  const { gfx, game, stage } = setup();
  const target = { x: 30, y: 40, active: true };
  const fx = stage.play("fire/candle", 0, 0, {}).follow(target, 0, -4);
  const events: string[] = [];
  fx.on("complete", () => events.push("complete"));
  fx.on("destroy", () => events.push("destroy"));
  frame(game, stage);
  const at = places(gfx);
  target.x += 12;
  frame(game, stage, 0);
  expect(places(gfx)).toEqual(at.map(([x, y]) => [x + 12, y]));
  target.active = false;
  for (let i = 0; i < 120 && events.length === 0; i++) frame(game, stage);
  expect(events).toEqual(["complete"]);
  expect(gfx.showing()).toEqual([]);
  expect(stage.counts().engines).toBe(0);
});

test("destroying an effect emits destroy, never complete, and it shows no more", () => {
  const { gfx, game, stage } = setup();
  const fx = stage.play("fire/candle", 30, 40, {});
  const events: string[] = [];
  fx.on("complete", () => events.push("complete"));
  fx.on("destroy", () => events.push("destroy"));
  frame(game, stage);
  fx.destroy();
  fx.destroy();
  for (let i = 0; i < 5; i++) frame(game, stage);
  expect(events).toEqual(["destroy"]);
  expect(gfx.showing()).toEqual([]);
});

test("an effect's cues reach it and the feel, placed in the world; its hit-stop freezes play", () => {
  const { game, stage, feel, cues } = setup();
  const fx = stage.play("impacts/parry", 100, 80, {});
  const own: WorldCue[] = [];
  fx.on("cue", (c: WorldCue) => own.push(c));
  const played: number[] = [];
  let trauma = 0;
  for (let i = 0; i < 20; i++) {
    played.push(frame(game, stage));
    trauma = Math.max(trauma, feel.trauma);
  }
  expect(own).toEqual(cues);
  const kinds = new Set<string>(cues.map((c) => c.cue.type));
  for (const k of ["shake", "hitstop", "ripple"]) expect(kinds.has(k)).toBe(true);
  for (const c of cues) {
    if (c.cue.type !== "ripple") continue;
    expect(Math.abs(c.cue.x - 100)).toBeLessThan(40);
    expect(Math.abs(c.cue.y - 80)).toBeLessThan(40);
  }
  expect(played).toContain(0);
  expect(trauma).toBeGreaterThan(0);
});

test("with the feel off, play runs every second and cues still reach the game", () => {
  const { game, stage, feel, cues } = setup();
  feel.setEnabled(false);
  stage.play("impacts/parry", 100, 80, {});
  const played: number[] = [];
  for (let i = 0; i < 20; i++) played.push(frame(game, stage));
  expect(played.every((p) => p > 0)).toBe(true);
  expect(cues.length).toBeGreaterThan(0);
  expect(feel.shake).toEqual([0, 0]);
});

test("the game's own freeze holds play, and timeScale says so", () => {
  const { game, stage, feel } = setup();
  stage.play("fire/candle", 30, 40, {});
  feel.freeze(0.1);
  const played = frame(game, stage);
  feel.played(1 / 30, played);
  expect(played).toBe(0);
  expect(feel.timeScale).toBe(0);
});

test("two scenes of a game showing identical effects paint each frame once", () => {
  const one = setup();
  one.stage.play("fire/candle", 30, 40, { seed: 3 });
  const two = setup();
  const other = scene(two.game, two.gfx);
  two.stage.play("fire/candle", 30, 40, { seed: 3 });
  other.stage.play("fire/candle", 90, 10, { seed: 3 });
  for (let i = 0; i < 10; i++) {
    frame(one.game, one.stage);
    two.stage.update(1 / 30);
    other.stage.update(1 / 30);
    two.stage.draw();
    other.stage.draw();
    two.game.endFrame();
  }
  const fills = (gfx: FakeGfx): number => gfx.textures.reduce((n, t) => n + t.fills, 0);
  expect(fills(two.gfx)).toBe(fills(one.gfx));
  expect(two.gfx.showing().length).toBe(2 * one.gfx.showing().length);
});

test("an unknown effect or an option the engine refuses throws, naming it", () => {
  const { stage } = setup();
  expect(() => stage.play("fire/nope", 0, 0, {})).toThrow(/fire\/nope/);
  expect(() => stage.play("fire/candle", 0, 0, { palette: "lava" })).toThrow(/lava|palette/);
  expect(() => stage.play("fire/candle", 0, 0, { margin: 3 })).toThrow(/margin/);
  expect(() => parsePlayOptions({ depth: { back: 1 } })).toThrow(/depth/);
});

test("shutting a scene down ends its effects; clearing the game lets go of every texture", () => {
  const { gfx, game, stage } = setup();
  const ended: string[] = [];
  for (let i = 0; i < 3; i++) stage.play("fire/candle", 20 * i, 40, { seed: i }).on("destroy", () => ended.push("destroy"));
  const stopping = stage.play("fire/candle", 80, 40, {});
  frame(game, stage);
  stopping.stop();
  frame(game, stage);
  stage.clear();
  expect(ended).toEqual(["destroy", "destroy", "destroy"]);
  expect(gfx.images.every((i) => i.destroyed)).toBe(true);
  game.clear();
  expect(game.counts()).toEqual({ surfaces: 0, textures: 0 });
  expect(gfx.textures.every((t) => t.destroyed)).toBe(true);
});
