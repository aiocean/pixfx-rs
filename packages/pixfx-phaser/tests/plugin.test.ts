import { expect, mock, test } from "bun:test";
import EventEmitter from "eventemitter3";
import type { Backend } from "../src/backend";
import type { EngineOptions, FxLike } from "../src/engine";
import { InlineBackend } from "../src/inline";
import { getBackend } from "../src/load";
import { FakeEngine } from "./fake-engine";
import { FakeGfx } from "./fake-gfx";

// Phaser's runtime needs a DOM. Phaser.Events.EventEmitter is a thin wrapper over eventemitter3,
// the only Phaser value the Phaser-free effect code uses, so the test supplies exactly that.
void mock.module("phaser", () => ({ default: { Events: { EventEmitter } } }));
const { EffectManager } = await import("../src/effects");

/** Records the options handed to the engine. */
class SpyEngine extends FakeEngine {
  readonly opts: EngineOptions[] = [];
  override create(id: string, opts: EngineOptions): FxLike {
    const fx = super.create(id, opts);
    this.opts.push(opts);
    return fx;
  }
}

function setup() {
  const engine = new SpyEngine();
  const gfx = new FakeGfx();
  const backend = new InlineBackend(engine);
  const fx = new EffectManager(gfx, (): Backend => backend, "pixfx:test:");
  return { engine, gfx, fx };
}

const EMPTY = { engines: 0, textures: 0, images: 0 };

test("play of an unknown id throws naming the id and creates nothing", () => {
  const { engine, gfx, fx } = setup();
  expect(() => fx.play("nope/none", 0, 0)).toThrow(/nope\/none/);
  expect(fx.stats()).toEqual(EMPTY);
  expect(engine.live).toBe(0);
  expect(gfx.textures.length).toBe(0);
  expect(gfx.images.length).toBe(0);
});

test("play before loadPixfx throws the load error", () => {
  const gfx = new FakeGfx();
  const fx = new EffectManager(gfx, getBackend, "pixfx:test:");
  expect(() => fx.play("fake/spark", 0, 0)).toThrow(new Error("pixfx: call await loadPixfx() before play()"));
  expect(fx.stats()).toEqual(EMPTY);
  expect(gfx.textures.length).toBe(0);
});

test("invalid options throw and create nothing", () => {
  const { engine, fx } = setup();
  expect(() => fx.play("fake/spark", 0, 0, { seed: -1 })).toThrow(/seed/);
  expect(fx.stats()).toEqual(EMPTY);
  expect(engine.live).toBe(0);
});

test("one play makes one engine, four textures, four images", () => {
  const { engine, gfx, fx } = setup();
  fx.play("fake/spark", 0, 0);
  expect(fx.stats()).toEqual({ engines: 1, textures: 4, images: 4 });
  expect(engine.live).toBe(1);
  expect(gfx.textures.map((t) => [t.width, t.height])).toEqual([
    [8, 6],
    [8, 6],
    [8, 6],
    [8, 6],
  ]);
  expect(new Set(gfx.textures.map((t) => t.key)).size).toBe(4);
  expect(gfx.textures.every((t) => t.key.startsWith("pixfx:test:"))).toBe(true);
});

test("engine gets the glow margin and no Phaser-side options", () => {
  const { engine, fx } = setup();
  fx.play("fake/spark", 0, 0, { seed: 7, scrollFactor: 0, depth: { back: 3, front: 4 } });
  expect(engine.opts).toEqual([{ seed: 7, margin: 4 }]);
});

test("light images blend additively, colour images normally, light drawn after colour", () => {
  const { gfx, fx } = setup();
  fx.play("fake/spark", 0, 0);
  expect(gfx.images.map((i) => i.blend)).toEqual(["normal", "add", "normal", "add"]);
  expect(gfx.images.map((i) => i.key)).toEqual(gfx.textures.map((t) => t.key));
});

test("default depths are back 0, front 1; opts.depth and setDepths override", () => {
  const { gfx, fx } = setup();
  fx.play("fake/spark", 0, 0);
  expect(gfx.images.map((i) => i.depth)).toEqual([0, 0, 1, 1]);

  const e = fx.play("fake/spark", 0, 0, { depth: { back: 5, front: 9 } });
  expect(gfx.images.slice(4).map((i) => i.depth)).toEqual([5, 5, 9, 9]);

  expect(e.setDepths(-2, 2)).toBe(e);
  expect(gfx.images.slice(4).map((i) => i.depth)).toEqual([-2, -2, 2, 2]);
});

test("scrollFactor defaults to 1 and follows the option", () => {
  const { gfx, fx } = setup();
  fx.play("fake/spark", 0, 0);
  fx.play("fake/spark", 0, 0, { scrollFactor: 0 });
  expect(gfx.images.map((i) => i.scrollFactor)).toEqual([1, 1, 1, 1, 0, 0, 0, 0]);
});

test("images sit at the snapped top-left of the effect", () => {
  const { gfx, fx } = setup();
  const e = fx.play("fake/spark", 10.6, 20);
  expect(gfx.images.map((i) => [i.x, i.y])).toEqual(Array.from({ length: 4 }, () => [11 - 4, 20 - 5]));
  expect(e.setPosition(30, 40)).toBe(e);
  expect(gfx.images.map((i) => [i.x, i.y])).toEqual(Array.from({ length: 4 }, () => [30 - 4, 40 - 5]));
});

test("follow tracks the target on scene updates", () => {
  const { gfx, fx } = setup();
  const e = fx.play("fake/spark", 0, 0);
  const target = { x: 100, y: 50, active: true };
  expect(e.follow(target, 2, -3)).toBe(e);
  fx.update(16, 1);
  expect(gfx.images.map((i) => [i.x, i.y])).toEqual(Array.from({ length: 4 }, () => [102 - 4, 47 - 5]));
  target.x = 200;
  fx.update(16, 1);
  expect(gfx.images[0]?.x).toBe(202 - 4);
});

test("textures upload only when the effect reaches a new frame of its fps", () => {
  const { gfx, fx } = setup();
  fx.play("fake/spark", 0, 0); // fake/spark: 30 fps
  const afterPlay = gfx.uploads();
  fx.update(16, 0);
  fx.update(16, Number.NaN);
  fx.update(-16, 1);
  fx.update(0, 1);
  expect(gfx.uploads()).toBe(afterPlay);
  fx.update(16, 1); // t 0.016 s: still frame 0
  expect(gfx.uploads()).toBe(afterPlay);
  fx.update(20, 1); // t 0.036 s: frame 1
  expect(gfx.uploads()).toBe(afterPlay + 4);
  fx.update(16, 1); // t 0.052 s: frame 1
  expect(gfx.uploads()).toBe(afterPlay + 4);
  fx.update(16, 1); // t 0.068 s: frame 2
  expect(gfx.uploads()).toBe(afterPlay + 8);
});

test("between effect frames a followed effect still moves every display frame", () => {
  const { gfx, fx } = setup();
  const target = { x: 10, y: 10, active: true };
  fx.play("fake/spark", 0, 0).follow(target);
  fx.update(10, 1);
  const uploads = gfx.uploads();
  target.x = 30;
  fx.update(10, 1); // t 0.02 s: same frame 0, new position
  expect(gfx.uploads()).toBe(uploads);
  expect(gfx.images[0]?.x).toBe(30 - 4);
});

test("scene time drives the effect: delta times timeScale", () => {
  const { engine, fx } = setup();
  fx.play("fake/spark", 0, 0);
  fx.update(100, 0.5);
  expect(engine.created[0]?.t()).toBeCloseTo(0.05);
});

test("completion emits complete once, then everything is freed", () => {
  const { engine, gfx, fx } = setup();
  const e = fx.play("fake/spark", 0, 0);
  let completes = 0;
  let destroys = 0;
  e.on("complete", () => (completes += 1));
  e.on("destroy", () => (destroys += 1));
  fx.update(600, 1);
  expect(completes).toBe(0);
  fx.update(600, 1);
  expect(completes).toBe(1);
  expect(destroys).toBe(0);
  expect(fx.stats()).toEqual(EMPTY);
  expect(engine.live).toBe(0);
  expect(gfx.textures.every((t) => t.destroyed)).toBe(true);
  expect(gfx.images.every((i) => i.destroyed)).toBe(true);
  fx.update(600, 1);
  e.stop();
  e.destroy();
  expect(completes).toBe(1);
  expect(destroys).toBe(0);
});

test("stop lets the effect fade, then it completes", () => {
  const { fx } = setup();
  const e = fx.play("fake/spark", 0, 0);
  let completes = 0;
  e.on("complete", () => (completes += 1));
  expect(e.stop()).toBe(e);
  fx.update(250, 1);
  expect(completes).toBe(0);
  fx.update(250, 1);
  expect(completes).toBe(1);
  expect(fx.stats()).toEqual(EMPTY);
});

test("destroy emits destroy, not complete, and frees at once", () => {
  const { engine, fx } = setup();
  const e = fx.play("fake/spark", 0, 0);
  let completes = 0;
  let destroys = 0;
  e.on("complete", () => (completes += 1));
  e.on("destroy", () => (destroys += 1));
  e.destroy();
  e.destroy();
  fx.update(2000, 1);
  expect(destroys).toBe(1);
  expect(completes).toBe(0);
  expect(fx.stats()).toEqual(EMPTY);
  expect(engine.live).toBe(0);
});

test("scene shutdown with three effects playing frees everything", () => {
  const { engine, gfx, fx } = setup();
  for (let i = 0; i < 3; i++) fx.play("fake/spark", i, i);
  expect(fx.stats()).toEqual({ engines: 3, textures: 12, images: 12 });
  fx.destroyAll();
  expect(fx.stats()).toEqual(EMPTY);
  expect(engine.live).toBe(0);
  expect(gfx.textures.every((t) => t.destroyed)).toBe(true);
  expect(gfx.images.every((i) => i.destroyed)).toBe(true);
  // A restarted scene plays again with fresh, distinct texture keys.
  fx.play("fake/spark", 0, 0);
  expect(new Set(gfx.textures.map((t) => t.key)).size).toBe(16);
});

test("a destroyed follow target stops the effect at its last position", () => {
  const { gfx, fx } = setup();
  const e = fx.play("fake/spark", 0, 0);
  let completes = 0;
  e.on("complete", () => (completes += 1));
  const target = { x: 10, y: 20, active: true };
  e.follow(target);
  fx.update(16, 1);
  target.active = false;
  target.x = 999;
  fx.update(16, 1);
  expect(gfx.images[0]?.x).toBe(10 - 4);
  fx.update(500, 1);
  expect(completes).toBe(1);
});
