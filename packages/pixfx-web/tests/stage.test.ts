import { expect, test } from "bun:test";
import { z } from "zod";
import { PixfxFeel } from "../src/feel";
import type { WorkerPort } from "../src/game";
import { Game } from "../src/game";
import { splitPlayOptions } from "../src/options";
import type { WorldCue } from "../src/stage";
import { Stage } from "../src/stage";
import { FakeGfx } from "./fake-gfx";
import { memory, wasm } from "./wasm";

type GameT = Game;
type StageT = Stage<number>;

/** A shell's own play options, as Phaser's: a depth pair and a number it places images by. */
const OWN = { place: z.number().default(1), depth: z.object({ back: z.number(), front: z.number() }).optional() };

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
  const feel = new PixfxFeel(() => stage?.live ?? null);
  feel.on("cue", (c: WorldCue) => cues.push(c));
  stage = new Stage<number>(game, gfx.image, (c, e) => feel.take(c, e), 1);
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
  stage.play("fire/candle", 30, 40, {}, 1);
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
  const fx = stage.play("fire/candle", 30, 40, {}, 1);
  frame(game, stage);
  const before = gfx.picture();
  fx.setPosition(40, 45);
  frame(game, stage, 0);
  const after = gfx.picture();
  expect(after.map((s) => s.bytes)).toEqual(before.map((s) => s.bytes));
  expect(after.map((s) => [s.at.x - 10, s.at.y - 5])).toEqual(before.map((s) => [s.at.x, s.at.y]));
});

test("an effect's back and front depths and its placement are the play's", () => {
  const { gfx, game, stage } = setup();
  const p = splitPlayOptions(OWN, { depth: { back: 5, front: 7 }, place: 0.5, seed: 2 });
  expect(p.engine).toEqual({ seed: 2 });
  // a rune circle draws behind and in front of what stands in it
  stage.play("arcane/rune-circle", 30, 40, p.engine, p.own.place, p.own.depth);
  frame(game, stage);
  const shown = gfx.showing().map((i) => i.shown);
  expect(new Set(shown.map((s) => s?.depth))).toEqual(new Set([5, 7]));
  expect(shown.every((s) => s?.place === 0.5)).toBe(true);
});

test("a followed effect tracks its target, stops when the target goes inactive, and completes once faded", () => {
  const { gfx, game, stage } = setup();
  const target = { x: 30, y: 40, active: true };
  const fx = stage.play("fire/candle", 0, 0, {}, 1).follow(target, 0, -4);
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
  const fx = stage.play("fire/candle", 30, 40, {}, 1);
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
  const fx = stage.play("impacts/parry", 100, 80, {}, 1);
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
  stage.play("impacts/parry", 100, 80, {}, 1);
  const played: number[] = [];
  for (let i = 0; i < 20; i++) played.push(frame(game, stage));
  expect(played.every((p) => p > 0)).toBe(true);
  expect(cues.length).toBeGreaterThan(0);
  expect(feel.shake).toEqual([0, 0]);
});

test("the game's own freeze holds play, and timeScale says so", () => {
  const { game, stage, feel } = setup();
  stage.play("fire/candle", 30, 40, {}, 1);
  feel.freeze(0.1);
  const played = frame(game, stage);
  feel.played(1 / 30, played);
  expect(played).toBe(0);
  expect(feel.timeScale).toBe(0);
});

test("two scenes of a game showing identical effects paint each frame once", () => {
  const one = setup();
  one.stage.play("fire/candle", 30, 40, { seed: 3 }, 1);
  const two = setup();
  const other = scene(two.game, two.gfx);
  two.stage.play("fire/candle", 30, 40, { seed: 3 }, 1);
  other.stage.play("fire/candle", 90, 10, { seed: 3 }, 1);
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
  expect(() => stage.play("fire/nope", 0, 0, {}, 1)).toThrow(/fire\/nope/);
  expect(() => stage.play("fire/candle", 0, 0, { palette: "lava" }, 1)).toThrow(/lava|palette/);
  expect(() => stage.play("fire/candle", 0, 0, { margin: 3 }, 1)).toThrow(/margin/);
  expect(() => splitPlayOptions(OWN, { depth: { back: 1 } })).toThrow(/depth/);
});

test("shutting a scene down ends its effects; clearing the game lets go of every texture", () => {
  const { gfx, game, stage } = setup();
  const ended: string[] = [];
  for (let i = 0; i < 3; i++) stage.play("fire/candle", 20 * i, 40, { seed: i }, 1).on("destroy", () => ended.push("destroy"));
  const stopping = stage.play("fire/candle", 80, 40, {}, 1);
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

test("a scene's feel reads still once the scene shut down, as a host loop may read it after destroy", () => {
  const { game, stage, feel } = setup();
  stage.play("explosions/small-explosion", 40, 40, {}, 1);
  for (let i = 0; i < 10; i++) frame(game, stage);
  expect(feel.trauma).toBeGreaterThan(0);
  stage.clear();
  expect(feel.shake).toEqual([0, 0]);
  expect(feel.roll).toBe(0);
  expect(feel.flash).toBe(0);
  expect(feel.trauma).toBe(0);
  expect(feel.frozen).toBe(false);
  expect(() => feel.addTrauma(1).freeze(1).slow(1, 0.5)).not.toThrow();
});

test("a heavy blow's ring bends a picture of the screen while it lasts; at shake strength 0 nothing bends", () => {
  const { game, stage, feel } = setup();
  const [w, h] = [64, 48];
  // stripes, so moving any pixel sideways or up changes it
  const src = new Uint8ClampedArray(w * h * 4).map((_, i) => ((i >> 2) % w) * 4 + (Math.floor((i >> 2) / w) % 2) * 100);
  expect(feel.bend(src, w, h, 1)).toBeUndefined();
  stage.play("explosions/plasma-explosion", 32, 24, {}, 1);
  let bent: Uint8ClampedArray | undefined;
  for (let i = 0; i < 90 && !bent; i++) {
    frame(game, stage);
    bent = feel.bend(src, w, h, 1);
  }
  if (!bent) throw new Error("no ring came");
  expect(bent.length).toBe(src.length);
  expect(bent.some((b, i) => b !== src[i])).toBe(true);
  feel.setShakeStrength(0);
  expect(feel.bend(src, w, h, 1)).toBeUndefined();
});

test("a draw budget shows effects that share nothing a few per frame, until every one shows", () => {
  const run = (budget: number) => {
    const { gfx, game, stage } = setup();
    stage.setDrawBudget(budget);
    for (let seed = 1; seed <= 4; seed++) stage.play("fire/candle", 20 * seed, 40, { seed }, 1);
    const shown: number[] = [];
    for (let i = 0; i < 6; i++) {
      frame(game, stage, i === 0 ? 1 / 30 : 0);
      shown.push(gfx.showing().length);
    }
    return shown;
  };
  const free = run(0);
  const capped = run(1);
  expect(free.every((n) => n === free[0])).toBe(true);
  expect(capped[0]).toBeLessThan(free[0] ?? 0);
  for (let i = 1; i < capped.length; i++) expect(capped[i]).toBeGreaterThanOrEqual(capped[i - 1] ?? 0);
  expect(capped.at(-1)).toBe(free[0]);
});

test("a draw budget is a whole number of layers, 0 or more", () => {
  const { stage } = setup();
  for (const bad of [-1, 1.5, Number.NaN, Number.POSITIVE_INFINITY]) expect(() => stage.setDrawBudget(bad)).toThrow();
});
