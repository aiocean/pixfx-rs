import { expect, mock, test } from "bun:test";
import EventEmitter from "eventemitter3";
import type { Backend } from "../src/backend";
import type { WorkerPort } from "../src/pool";
import { WorkerPool } from "../src/pool";
import type { Batch, ToWorker } from "../src/protocol";
import { fromWorkerSchema, toWorkerSchema } from "../src/protocol";
import { WorkerCore } from "../src/worker-core";
import { FakeEngine, PaintedEngine } from "./fake-engine";
import { FakeGfx } from "./fake-gfx";

void mock.module("phaser", () => ({ default: { Events: { EventEmitter } } }));
const { EffectManager } = await import("../src/effects");

/**
 * A worker without a thread: posts are structured-cloned with their transfer list (so a transferred buffer is
 * detached on the sending side, as in a browser) and wait until the test calls `answer()`, which runs them
 * through a real `WorkerCore` on its own fake engine.
 */
class LoopPort implements WorkerPort {
  readonly engine: FakeEngine;
  readonly core: WorkerCore;
  readonly posted: ToWorker[] = [];
  private queue: ToWorker[] = [];
  private deliver: ((data: unknown) => void) | null = null;

  constructor(engine: FakeEngine = new FakeEngine()) {
    this.engine = engine;
    this.core = new WorkerCore(engine);
  }
  post(message: ToWorker, transfer: ArrayBuffer[]): void {
    const copy = toWorkerSchema.parse(structuredClone(message, { transfer }));
    this.posted.push(copy);
    this.queue.push(copy);
  }
  listen(onMessage: (data: unknown) => void): void {
    this.deliver = onMessage;
  }
  /** Run the oldest posted batch and deliver the reply; false when nothing is in flight. */
  answer(): boolean {
    const msg = this.queue.shift();
    if (!msg) return false;
    if (msg.type !== "batch") throw new Error("the pool only posts batches");
    const { reply, transfer } = this.core.run(msg);
    this.deliver?.(structuredClone(reply, { transfer }));
    return true;
  }
  batches(): Batch[] {
    return this.posted.flatMap((m) => (m.type === "batch" ? [m] : []));
  }
}

function setup(workers = 1, workerEngine: () => FakeEngine = () => new FakeEngine()) {
  const main = new FakeEngine();
  const ports = Array.from({ length: workers }, () => new LoopPort(workerEngine()));
  const pool = new WorkerPool(main, ports);
  const gfx = new FakeGfx();
  const fx = new EffectManager(gfx, (): Backend => pool, "pixfx:test:");
  return { main, ports, pool, gfx, fx };
}

/** One display frame: scene update, then the per-frame send. */
function frame(fx: { update(deltaMs: number, timeScale: number): void }, pool: WorkerPool, ms = 16): void {
  fx.update(ms, 1);
  pool.flush();
}

const EMPTY = { engines: 0, textures: 0, images: 0 };

test("play validates on the main thread: unknown ids and bad options throw at once and send nothing", () => {
  const { main, ports, pool, fx } = setup();
  expect(() => fx.play("nope/none", 0, 0)).toThrow(/nope\/none/);
  expect(() => fx.play("fake/spark", 0, 0, { seed: -1 })).toThrow(/seed/);
  pool.flush();
  expect(ports[0]?.posted).toEqual([]);
  expect(fx.stats()).toEqual(EMPTY);
  expect(main.live).toBe(0);
});

test("a play builds and frees one main-thread instance for its size, and spawns the effect in a worker", () => {
  const { main, ports, pool, gfx, fx } = setup();
  fx.play("fake/spark", 0, 0, { seed: 7 });
  expect(main.created.length).toBe(1);
  expect(main.live).toBe(0);
  expect(gfx.textures.map((t) => [t.width, t.height])).toEqual(Array.from({ length: 4 }, () => [8, 6]));
  expect(fx.stats()).toEqual({ engines: 1, textures: 4, images: 4 });
  pool.flush();
  expect(ports[0]?.batches()[0]?.spawns).toEqual([{ handle: 0, id: "fake/spark", opts: { seed: 7, margin: 4 }, fps: 30 }]);
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

test("a paused scene sends no time; a play while paused is still spawned", () => {
  const { ports, pool, gfx, fx } = setup();
  fx.update(16, 0);
  fx.play("fake/spark", 0, 0);
  pool.flush();
  ports[0]?.answer();
  expect(gfx.uploads()).toBe(4);
  fx.update(16, 0);
  pool.flush();
  expect(ports[0]?.batches().length).toBe(1);
});

test("uploaded buffers go back to the worker and are reused: a long run allocates one set", () => {
  const { ports, pool, gfx, fx } = setup();
  fx.play("fake/spark", 0, 0);
  for (let i = 0; i < 25; i++) {
    frame(fx, pool, 1000 / 30);
    ports[0]?.answer();
  }
  expect(gfx.textures[0]?.uploads).toBeGreaterThan(20);
  // An own-surface effect hands its frame back as soon as it is uploaded, so one set keeps cycling.
  expect(ports[0]?.core.allocated).toBe(1);
  const returned = (ports[0]?.batches() ?? []).flatMap((b) => b.items.flatMap((i) => i.give));
  expect(returned.length).toBeGreaterThan(20);
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
  expect(gfx.textures.every((t) => t.destroyed)).toBe(true);
  expect(gfx.images.every((i) => i.destroyed)).toBe(true);
  expect(ports[0]?.engine.live).toBe(0);
  frame(fx, pool);
  expect(ports[0]?.answer()).toBe(false);
});

test("stop is forwarded; the effect fades in the worker, then completes", () => {
  const { ports, pool, fx } = setup();
  const e = fx.play("fake/spark", 0, 0);
  let completes = 0;
  e.on("complete", () => (completes += 1));
  e.stop();
  frame(fx, pool, 250);
  expect(ports[0]?.batches()[0]?.items[0]?.stop).toBe(true);
  ports[0]?.answer();
  frame(fx, pool, 250);
  ports[0]?.answer();
  expect(completes).toBe(1);
  expect(ports[0]?.engine.live).toBe(0);
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

test("scene shutdown frees every effect in its worker", () => {
  const { ports, pool, fx } = setup(2);
  for (let i = 0; i < 4; i++) fx.play("fake/spark", i, i);
  frame(fx, pool);
  for (const p of ports) p.answer();
  fx.destroyAll();
  expect(fx.stats()).toEqual(EMPTY);
  pool.flush();
  for (const p of ports) p.answer();
  expect(ports.map((p) => p.engine.live)).toEqual([0, 0]);
});

test("each effect goes to the worker holding the least pixel area", () => {
  const big = () => {
    const e = new FakeEngine();
    e.metas["fake/big"] = { ...e.meta("fake/spark"), id: "fake/big", size: [40, 30] };
    return e;
  };
  const { main, ports, pool, fx } = setup(2, big);
  main.metas["fake/big"] = { ...main.meta("fake/spark"), id: "fake/big", size: [40, 30] };
  fx.play("fake/big", 0, 0); // worker 0: 1200 px
  fx.play("fake/spark", 0, 0); // worker 1: 48
  fx.play("fake/spark", 0, 0); // worker 1: 96
  const e = fx.play("fake/big", 0, 0); // worker 1: 1296
  e.destroy(); // worker 1 back to 96
  fx.play("fake/spark", 0, 0); // worker 1: 144
  pool.flush();
  const spawned = ports.map((p) => p.batches().flatMap((b) => b.spawns.map((s) => s.id)));
  expect(spawned).toEqual([["fake/big"], ["fake/spark", "fake/spark", "fake/spark"]]);
});

test("a layer composites its members from their stored frames", () => {
  const painted = () => {
    const e = new PaintedEngine();
    e.paint = { color: 0xff0000ff, light: 0x00000010 };
    return e;
  };
  const { ports, pool, gfx, fx } = setup(1, painted);
  const layer = fx.createLayer({ bounds: { x: 100, y: 200, width: 20, height: 10 } });
  const e = layer.play("fake/spark", 104, 205);
  fx.flushLayers();
  expect(gfx.textures[0]?.written()).toBe(false);
  pool.flush();
  ports[0]?.answer();
  fx.flushLayers();
  expect(gfx.textures[0]?.px(0, 0)).toEqual([255, 0, 0, 255]);
  expect(gfx.textures[1]?.px(0, 0)).toEqual([16, 0, 0, 255]);
  // A move re-composites from the stored frame; the worker is not asked for anything.
  e.setPosition(110, 205);
  fx.flushLayers();
  expect(gfx.textures[0]?.px(0, 0)).toEqual([0, 0, 0, 0]);
  expect(gfx.textures[0]?.px(6, 0)).toEqual([255, 0, 0, 255]);
  expect(ports[0]?.answer()).toBe(false);
});

test("worker replies are parsed: a malformed reply throws", () => {
  const ports: { deliver: (data: unknown) => void }[] = [];
  const port: WorkerPort = {
    post: () => {},
    listen: (onMessage) => ports.push({ deliver: onMessage }),
  };
  new WorkerPool(new FakeEngine(), [port]);
  expect(() => ports[0]?.deliver({ type: "results", results: [{ handle: "x" }] })).toThrow();
  expect(() => ports[0]?.deliver({ type: "error", message: "boom" })).toThrow(/boom/);
  expect(fromWorkerSchema.safeParse({ type: "ready" }).success).toBe(true);
});
