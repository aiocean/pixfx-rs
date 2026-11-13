import { expect, mock, test } from "bun:test";
import EventEmitter from "eventemitter3";
import type { Backend } from "../src/backend";
import { WorkerPool } from "../src/pool";
import { FakeEngine } from "./fake-engine";
import { FakeGfx } from "./fake-gfx";
import { LoopPort } from "./loop-port";

void mock.module("phaser", () => ({ default: { Events: { EventEmitter } } }));
const { EffectManager } = await import("../src/effects");
const { PixfxFeel } = await import("../src/feel");

function setup(workers = 1, workerEngine: () => FakeEngine = () => new FakeEngine()) {
  const main = new FakeEngine();
  const ports = Array.from({ length: workers }, () => new LoopPort(workerEngine()));
  const pool = new WorkerPool(main, ports);
  const gfx = new FakeGfx();
  const fx = new EffectManager(gfx, (): Backend => pool, "pixfx:test:", new PixfxFeel(() => main));
  return { main, ports, pool, gfx, fx };
}

/** One display frame: scene update, then the per-frame send. */
function frame(fx: { update(deltaMs: number, timeScale: number): void }, pool: WorkerPool, ms = 16): void {
  fx.update(ms, 1);
  pool.flush();
}

const EMPTY = { engines: 0, textures: 0, images: 0 };

test("a play builds and frees one main-thread instance for its size, and spawns the effect in a worker", () => {
  const { main, ports, pool, gfx, fx } = setup();
  fx.play("fake/spark", 0, 0, { seed: 7 });
  expect(main.created.length).toBe(1);
  expect(main.live).toBe(0);
  expect(gfx.images.length).toBe(4);
  expect(fx.stats()).toEqual({ engines: 1, textures: 0, images: 4 });
  pool.flush();
  expect(ports[0]?.batches()[0]?.spawns).toEqual([{ handle: 0, id: "fake/spark", opts: { seed: 7, margin: 4 } }]);
});

test("textures upload when the worker's reply arrives, not before", () => {
  const { ports, pool, gfx, fx } = setup();
  fx.play("fake/spark", 0, 0);
  frame(fx, pool);
  expect(gfx.uploads()).toBe(0);
  ports[0]?.answer();
  expect(gfx.uploads()).toBe(4);
  // FakeFx stamps each byte with its draw count: back colour/light are draw 1, front colour/light draw 2.
  expect(gfx.textures.map((t) => t.px(0, 0)[0])).toEqual([1, 1, 2, 2]);
});

test("one batch in flight per worker; scene time accumulates while it is busy", () => {
  const { ports, pool, fx } = setup();
  fx.play("fake/spark", 0, 0);
  frame(fx, pool);
  frame(fx, pool);
  frame(fx, pool);
  expect(ports[0]?.batches().length).toBe(1);
  ports[0]?.answer();
  pool.flush();
  const batches = ports[0]?.batches() ?? [];
  expect(batches.length).toBe(2);
  expect(batches[0]?.items[0]?.dt).toBeCloseTo(0.016);
  expect(batches[1]?.items[0]?.dt).toBeCloseTo(0.032);
  expect(batches[1]?.spawns).toEqual([]);
});

test("a done reply emits complete and frees everything on both sides", () => {
  const { ports, pool, gfx, fx } = setup();
  const e = fx.play("fake/spark", 0, 0);
  let completes = 0;
  let destroys = 0;
  e.on("complete", () => (completes += 1));
  e.on("destroy", () => (destroys += 1));
  frame(fx, pool, 600);
  ports[0]?.answer();
  frame(fx, pool, 600);
  expect(completes).toBe(0);
  ports[0]?.answer();
  expect(completes).toBe(1);
  expect(destroys).toBe(0);
  expect(fx.stats()).toEqual(EMPTY);
  // what stays is the game's shared frames, not the effect's
  expect(gfx.textures.filter((t) => !t.destroyed).every((t) => t.key.startsWith("pixfx:frame"))).toBe(true);
  expect(gfx.images.every((i) => i.destroyed)).toBe(true);
  expect(ports[0]?.engine.live).toBe(0);
  frame(fx, pool);
  expect(ports[0]?.answer()).toBe(false);
});

test("destroy while a batch is in flight: the late reply uploads nothing and the worker frees the instance", () => {
  const { ports, pool, gfx, fx } = setup();
  const e = fx.play("fake/spark", 0, 0);
  frame(fx, pool);
  e.destroy();
  expect(fx.stats()).toEqual(EMPTY);
  ports[0]?.answer();
  expect(gfx.uploads()).toBe(0);
  pool.flush();
  expect(ports[0]?.batches()[1]?.frees).toEqual([0]);
  ports[0]?.answer();
  expect(ports[0]?.engine.live).toBe(0);
});

test("destroy before the spawn was sent sends nothing at all", () => {
  const { ports, pool, fx } = setup();
  fx.play("fake/spark", 0, 0).destroy();
  pool.flush();
  expect(ports[0]?.posted).toEqual([]);
});

test("cues an effect fires in a worker reach the main thread in the reply", () => {
  const hit = [{ t: 0.01, pan: null, cue: { type: "shake" as const, trauma: 0.5 } }];
  const { ports, pool, fx } = setup(1, () => {
    const e = new FakeEngine();
    e.schedule = hit;
    return e;
  });
  const effect = fx.play("fake/spark", 0, 0);
  const got: unknown[] = [];
  effect.on("cue", (c: unknown) => got.push(c));
  frame(fx, pool);
  ports[0]?.answer();
  frame(fx, pool);
  ports[0]?.answer();
  expect(got).toEqual(hit);
});
