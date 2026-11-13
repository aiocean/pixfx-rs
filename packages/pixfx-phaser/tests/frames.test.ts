import { expect, mock, test } from "bun:test";
import EventEmitter from "eventemitter3";
import * as wasm from "../wasm/pixfx_wasm.js";
import type { Backend } from "../src/backend";
import type { Engine } from "../src/engine";
import { wasmEngine } from "../src/engine";
import type { Frame } from "../src/frames";
import { FrameCache } from "../src/frames";
import { InlineBackend } from "../src/inline";
import { WorkerPool } from "../src/pool";
import { FakeEngine } from "./fake-engine";
import { FakeGfx } from "./fake-gfx";
import { LoopPort } from "./loop-port";

void mock.module("phaser", () => ({ default: { Events: { EventEmitter } } }));
const { EffectManager } = await import("../src/effects");
const { PixfxFeel } = await import("../src/feel");

type Mode = "inline" | "pool";

/**
 * A scene's effects on `engine`, drawn inline or by a pool of `workers` loop workers (each on its own engine
 * from `workerEngine`). `frame(ms)` is one display frame: scene update, the per-step flush, and (unless `late`)
 * the workers' answers.
 */
function rig<E extends Engine>(mode: Mode, engine: E, opts: { workers?: number; workerEngine?: () => E; budget?: number } = {}) {
  const cache = new FrameCache(opts.budget);
  const gfx = new FakeGfx();
  const ports = mode === "pool" ? Array.from({ length: opts.workers ?? 1 }, () => new LoopPort(opts.workerEngine?.() ?? engine)) : [];
  const backend: Backend = mode === "pool" ? new WorkerPool(engine, ports, cache) : new InlineBackend(engine, cache);
  const fx = new EffectManager(gfx, () => backend, "pixfx:test:", new PixfxFeel(() => new FakeEngine()));
  const answer = (): void => {
    for (const p of ports) while (p.answer());
  };
  const frame = (ms: number, late = false): void => {
    fx.update(ms, 1);
    backend.flush();
    if (!late) answer();
  };
  return { cache, gfx, ports, backend, fx, frame, answer };
}

/** Every byte of a frame, its boxes included, for comparing frames. */
function bytesOf(f: Frame): string {
  const side = (s: Frame["back"]): string =>
    `${s.box.x},${s.box.y},${s.box.w},${s.box.h}|${Array.from(new Uint8Array(s.color)).join(",")}|${Array.from(new Uint8Array(s.light)).join(",")}`;
  return `${side(f.back)}#${side(f.front)}`;
}

/** The shared frame id a fake effect of "fake/spark" with default options shows at frame `k`. */
function sparkId(k: number): string {
  return `${new FakeEngine().create("fake/spark", { margin: 4 }).shown()?.replace(/:0$/, "") ?? ""}:${k}`;
}

for (const mode of ["inline", "pool"] as const) {
  test(`${mode}: fifty identical effects draw each frame once and upload its textures once`, () => {
    const engine = new FakeEngine();
    const r = rig(mode, engine, { workers: 2 });
    const effects = Array.from({ length: 50 }, (_, i) => r.fx.play("fake/spark", i * 10, 0));
    r.frame(0);
    for (let i = 0; i < 12; i++) r.frame(40);
    const frames = r.cache.size;
    expect(frames).toBeGreaterThan(8);
    // back and front: two draws and four uploads per frame, however many effects show it
    expect(engine.draws()).toBe(2 * frames);
    expect(r.gfx.uploads()).toBe(4 * frames);
    // every effect shows the one frame
    const shown = new Set(r.gfx.images.filter((i) => !i.destroyed && i.blend === "normal").map((i) => i.key));
    expect(shown.size).toBe(2);
    if (mode === "pool") expect(r.ports[1]?.batches().length).toBe(0);
    for (const e of effects) e.destroy();
  });

  test(`${mode}: nothing is drawn or uploaded while the shown frame is the same`, () => {
    const engine = new FakeEngine();
    const r = rig(mode, engine);
    for (let i = 0; i < 10; i++) r.fx.play("fake/spark", i, 0);
    r.frame(0);
    const draws = engine.draws();
    const uploads = r.gfx.uploads();
    // 5 ms steps up to 30 ms: still frame 0 of 30 fps
    for (let i = 0; i < 6; i++) r.frame(5);
    expect(engine.draws()).toBe(draws);
    expect(r.gfx.uploads()).toBe(uploads);
    r.frame(5);
    expect(engine.draws()).toBe(draws + 2);
    expect(r.gfx.uploads()).toBe(uploads + 4);
  });

  test(`${mode}: a dropped frame is drawn again with the same bytes`, () => {
    const engine = new FakeEngine();
    engine.box = { x: 2, y: 1, w: 3, h: 4 };
    const r = rig(mode, engine, { budget: 0 });
    const first = r.fx.play("fake/spark", 0, 0);
    r.frame(0);
    const id = sparkId(0);
    const held = r.cache.get(id);
    expect(held).toBeDefined();
    const before = held ? bytesOf(held.frame) : "";
    // the effect moves on; a frame shown by nobody since the last step goes at the next one
    for (let i = 0; i < 3; i++) r.frame(40);
    expect(r.cache.has(id)).toBe(false);
    if (mode === "pool") expect(r.ports[0]?.batches().flatMap((b) => b.drops)).toContain(id);
    const again = r.fx.play("fake/spark", 50, 0);
    r.frame(0);
    r.frame(0);
    const back = r.cache.get(id);
    expect(back ? bytesOf(back.frame) : "").toBe(before);
    // shown at its box: the effect's top-left (50 - 4, 0 - 5) plus (2, 1)
    const image = r.gfx.images.slice(4).find((i) => i.blend === "normal");
    expect([image?.x, image?.y]).toEqual([48, -4]);
    first.destroy();
    again.destroy();
  });

  test(`${mode}: a stopped effect fades on its own textures, drawn only on a new clock frame`, () => {
    const engine = new FakeEngine();
    const r = rig(mode, engine);
    const fading = r.fx.play("fake/spark", 0, 0);
    const running = r.fx.play("fake/spark", 20, 0);
    r.frame(0);
    r.frame(100); // clock frame 3
    fading.stop();
    r.frame(5);
    const shared = r.cache.size;
    expect(r.fx.stats().textures).toBe(4);
    const [backColor, , frontColor] = r.gfx.images;
    expect(backColor?.key).toBe("pixfx:test:0-back-color");
    expect(backColor && r.gfx.shownBy(backColor)?.px(0, 0)).toEqual([135, 135, 135, 135]); // 128 + 2 * 3 + 1
    expect(frontColor && r.gfx.shownBy(frontColor)?.px(0, 0)).toEqual([136, 136, 136, 136]);
    // the running twin still shows the shared frame
    expect(r.gfx.images[4]?.key?.startsWith("pixfx:frame")).toBe(true);
    const own = r.gfx.textures.filter((t) => t.key.startsWith("pixfx:test:0-"));
    const uploads = (): number => own.reduce((n, t) => n + t.uploads, 0);
    const drawn = engine.draws();
    const ownUploads = uploads();
    r.frame(5); // 0.11 s: still clock frame 3
    expect(engine.draws()).toBe(drawn);
    expect(uploads()).toBe(ownUploads);
    running.destroy();
    r.frame(30); // clock frame 4
    expect(engine.draws()).toBe(drawn + 2);
    expect(uploads()).toBe(ownUploads + 4);
    expect(r.cache.size).toBe(shared);
    fading.destroy();
    expect(r.fx.stats()).toEqual({ engines: 0, textures: 0, images: 0 });
  });
}

test("pool: a frame dropped while a reply naming it is on its way is asked for and comes back the same", () => {
  const r = rig("pool", new FakeEngine(), { budget: 0, workerEngine: () => new FakeEngine() });
  const x0 = sparkId(0);
  r.fx.play("fake/spark", 0, 0);
  r.frame(0);
  const before = bytesOf(r.cache.get(x0)?.frame ?? { back: empty(), front: empty() });
  r.frame(40); // the first effect moves on; frame 0 is shown by nobody
  r.fx.play("fake/spark", 30, 0); // starts on frame 0, which its worker believes the main thread holds
  r.backend.flush();
  r.frame(16, true); // frame 0 is dropped while the batch is out
  expect(r.cache.has(x0)).toBe(false);
  r.answer(); // the reply names frame 0 without its bytes
  r.backend.flush();
  r.answer();
  const batches = r.ports[0]?.batches() ?? [];
  expect(batches.at(-1)?.drops).toContain(x0);
  expect(batches.at(-1)?.items.some((i) => i.want)).toBe(true);
  expect(bytesOf(r.cache.get(x0)?.frame ?? { back: empty(), front: empty() })).toBe(before);
  const image = r.gfx.images.slice(4).find((i) => i.blend === "normal");
  expect(image?.visible).toBe(true);
  expect(image && r.gfx.shownBy(image)?.px(0, 0)).toEqual([1, 1, 1, 1]);
});

function empty(): Frame["back"] {
  return { box: { x: 0, y: 0, w: 0, h: 0 }, color: new ArrayBuffer(0), light: new ArrayBuffer(0) };
}

test("wasm: inline and a late worker hold the same campfire frames, cropped to their lit boxes", async () => {
  const bytes = await Bun.file(new URL("../wasm/pixfx_wasm_bg.wasm", import.meta.url)).arrayBuffer();
  const { memory } = wasm.initSync({ module: bytes });
  const engine = wasmEngine(wasm, memory);
  const a = rig("inline", engine);
  const b = rig("pool", engine);
  for (const r of [a, b]) {
    r.fx.play("fire/campfire", 50, 50, { seed: 3 });
    r.fx.play("fire/campfire", 90, 50, { seed: 3 });
    r.fx.play("fire/campfire", 50, 90, { seed: 4 });
  }
  a.frame(0);
  b.frame(0);
  for (let i = 0; i < 24; i++) {
    a.frame(17);
    // the worker answers every third frame: its effects take the scene time of three frames in one step
    b.frame(17, i % 3 !== 2);
  }
  b.answer();
  expect(b.cache.size).toBeGreaterThan(6);
  const meta = engine.meta("fire/campfire");
  const full = (meta.size[0] + 2 * meta.glowMargin) * (meta.size[1] + 2 * meta.glowMargin) * 4 * 4;
  let checked = 0;
  for (let k = 0; k < 90; k++) {
    for (const seed of [3, 4]) {
      const fx = engine.create("fire/campfire", { seed, margin: meta.glowMargin });
      const clip = fx.shown()?.replace(/:0$/, "") ?? "";
      fx.free();
      const id = `${clip}:${k}`;
      const inPool = b.cache.get(id);
      if (!inPool) continue;
      const inline = a.cache.get(id);
      expect(inline).toBeDefined();
      expect(inline ? bytesOf(inline.frame) : "").toBe(bytesOf(inPool.frame));
      expect(inPool.bytes).toBeLessThan(full);
      checked += 1;
    }
  }
  expect(checked).toBe(b.cache.size);
});
