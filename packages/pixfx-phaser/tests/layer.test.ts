import { expect, mock, test } from "bun:test";
import EventEmitter from "eventemitter3";
import type { Engine, EngineOptions, FxLike } from "../src/engine";
import type { Gfx, GfxBlend, GfxImage, GfxTexture } from "../src/gfx";
import { FakeEngine, FakeFx } from "./fake-engine";

void mock.module("phaser", () => ({ default: { Events: { EventEmitter } } }));
const { EffectManager } = await import("../src/effects");

class FakeTexture implements GfxTexture {
  readonly pixels: Uint8ClampedArray;
  uploads = 0;
  uploadsWhenDestroyed = 0;
  destroyed = false;
  constructor(
    readonly key: string,
    readonly width: number,
    readonly height: number,
  ) {
    this.pixels = new Uint8ClampedArray(width * height * 4);
  }
  upload(): void {
    this.uploads += 1;
    if (this.destroyed) this.uploadsWhenDestroyed += 1;
  }
  destroy(): void {
    this.destroyed = true;
  }
  px(x: number, y: number): number[] {
    const o = (y * this.width + x) * 4;
    return Array.from(this.pixels.slice(o, o + 4));
  }
  written(): boolean {
    return this.pixels.some((v) => v !== 0);
  }
}

class FakeImage implements GfxImage {
  x = Number.NaN;
  y = Number.NaN;
  depth = Number.NaN;
  destroyed = false;
  constructor(
    readonly key: string,
    readonly blend: GfxBlend,
    readonly scrollFactor: number,
  ) {}
  setPosition(x: number, y: number): void {
    this.x = x;
    this.y = y;
  }
  setDepth(depth: number): void {
    this.depth = depth;
  }
  destroy(): void {
    this.destroyed = true;
  }
}

class FakeGfx implements Gfx {
  readonly textures: FakeTexture[] = [];
  readonly images: FakeImage[] = [];
  texture(key: string, width: number, height: number): FakeTexture {
    const t = new FakeTexture(key, width, height);
    this.textures.push(t);
    return t;
  }
  image(key: string, blend: GfxBlend, scrollFactor: number): FakeImage {
    const i = new FakeImage(key, blend, scrollFactor);
    this.images.push(i);
    return i;
  }
  uploads(): number {
    return this.textures.reduce((n, t) => n + t.uploads, 0);
  }
}

/** Every effect fills its colour with one ABGR word and its light with one xBGR word. */
class PaintedFx extends FakeFx {
  colorWord = 0;
  lightWord = 0;
  override color(): Uint32Array {
    return new Uint32Array(this.width() * this.height()).fill(this.colorWord);
  }
  override light(): Uint32Array {
    return new Uint32Array(this.width() * this.height()).fill(this.lightWord);
  }
}

class PaintedEngine extends FakeEngine {
  paint = { color: 0, light: 0 };
  override create(id: string, _opts: EngineOptions): FxLike {
    const m = this.meta(id);
    const fx = new PaintedFx(this, m.duration, m.size, m.origin);
    fx.colorWord = this.paint.color;
    fx.lightWord = this.paint.light;
    this.live += 1;
    this.created.push(fx);
    return fx;
  }
}

const EMPTY = { engines: 0, textures: 0, images: 0 };
const BOUNDS = { bounds: { x: 100, y: 200, width: 20, height: 10 } };

function setup(paint = { color: 0, light: 0 }) {
  const engine = new PaintedEngine();
  engine.paint = paint;
  const gfx = new FakeGfx();
  const fx = new EffectManager(gfx, (): Engine => engine, "pixfx:test:");
  return { engine, gfx, fx };
}

test("two effects in one layer share four textures and four images, with two engines", () => {
  const { gfx, fx } = setup();
  const layer = fx.createLayer(BOUNDS);
  expect(fx.stats()).toEqual({ engines: 0, textures: 4, images: 4 });
  layer.play("fake/spark", 105, 205);
  layer.play("fake/spark", 110, 205);
  expect(fx.stats()).toEqual({ engines: 2, textures: 4, images: 4 });
  expect(gfx.textures.map((t) => [t.width, t.height])).toEqual(Array.from({ length: 4 }, () => [20, 10]));
  expect(gfx.images.map((i) => [i.blend, i.x, i.y, i.depth, i.scrollFactor])).toEqual([
    ["normal", 100, 200, 0, 1],
    ["add", 100, 200, 0, 1],
    ["normal", 100, 200, 1, 1],
    ["add", 100, 200, 1, 1],
  ]);
});

test("layer depths and scrollFactor come from the config", () => {
  const { gfx, fx } = setup();
  fx.createLayer({ ...BOUNDS, depths: { back: 3, front: 8 }, scrollFactor: 0 });
  expect(gfx.images.map((i) => [i.depth, i.scrollFactor])).toEqual([
    [3, 0],
    [3, 0],
    [8, 0],
    [8, 0],
  ]);
});

test("invalid layer config throws and creates nothing", () => {
  const { gfx, fx } = setup();
  expect(() => fx.createLayer({ bounds: { ...BOUNDS.bounds, width: 0 } })).toThrow(/bounds\.width/);
  expect(() => fx.createLayer({ bounds: { ...BOUNDS.bounds, x: 0.5 } })).toThrow(/bounds\.x/);
  expect(fx.stats()).toEqual(EMPTY);
  expect(gfx.textures.length).toBe(0);
});

test("an effect fully outside the bounds draws nothing but still completes", () => {
  const { gfx, fx } = setup({ color: 0xff0000ff, light: 0x00ffffff });
  const layer = fx.createLayer(BOUNDS);
  const e = layer.play("fake/spark", 500, 500);
  let completes = 0;
  e.on("complete", () => (completes += 1));
  fx.update(100, 1);
  expect(gfx.textures.some((t) => t.written())).toBe(false);
  fx.update(1000, 1);
  expect(completes).toBe(1);
  expect(fx.stats()).toEqual({ engines: 0, textures: 4, images: 4 });
});

test("an effect partly outside is drawn in part, clipped to the bounds", () => {
  const { gfx, fx } = setup({ color: 0xff0000ff, light: 0x00000010 });
  const layer = fx.createLayer(BOUNDS);
  // Effect rect: x 96..104, y 195..201 -> overlap x 100..104, y 200..201.
  layer.play("fake/spark", 100, 200);
  fx.flushLayers();
  const [backColor, backLight] = gfx.textures;
  expect(backColor?.px(0, 0)).toEqual([255, 0, 0, 255]);
  expect(backColor?.px(3, 0)).toEqual([255, 0, 0, 255]);
  expect(backColor?.px(4, 0)).toEqual([0, 0, 0, 0]);
  expect(backColor?.px(0, 1)).toEqual([0, 0, 0, 0]);
  expect(backLight?.px(0, 0)).toEqual([16, 0, 0, 255]);
  expect(backLight?.px(4, 0)).toEqual([0, 0, 0, 0]);
});

test("colour composites source-over; light adds and saturates", () => {
  const engine = new PaintedEngine();
  const gfx = new FakeGfx();
  const fx = new EffectManager(gfx, (): Engine => engine, "pixfx:test:");
  const layer = fx.createLayer(BOUNDS);
  engine.paint = { color: 0xff0000ff, light: 0x00000090 }; // opaque red, light r=144
  layer.play("fake/spark", 104, 205);
  engine.paint = { color: 0x80ff0000, light: 0x00000090 }; // half-transparent blue, light r=144
  layer.play("fake/spark", 104, 205);
  fx.update(16, 1);
  const [backColor, backLight] = gfx.textures;
  // blue (a=128) over opaque red: r = 255*(1-128/255), b = 255*128/255.
  expect(backColor?.px(0, 0)).toEqual([127, 0, 128, 255]);
  expect(backLight?.px(0, 0)).toEqual([255, 0, 0, 255]);
});

test("each tick clears the layer first, and uploads only when a member moved or reached a new frame", () => {
  const { gfx, fx } = setup({ color: 0xff0000ff, light: 0 });
  const layer = fx.createLayer(BOUNDS);
  const e = layer.play("fake/spark", 104, 205);
  fx.flushLayers();
  expect(gfx.textures[0]?.px(0, 0)).toEqual([255, 0, 0, 255]);
  const uploads = gfx.textures[0]?.uploads ?? 0;
  fx.update(0, 1);
  fx.update(16, 0);
  fx.flushLayers();
  expect(gfx.textures[0]?.uploads).toBe(uploads);
  e.setPosition(110, 205);
  fx.flushLayers();
  expect(gfx.textures[0]?.px(0, 0)).toEqual([0, 0, 0, 0]);
  const moved = gfx.textures[0]?.uploads ?? 0;
  expect(moved).toBeGreaterThan(uploads);
  fx.update(16, 1); // t 0.016 s: still frame 0 at 30 fps, not moved
  fx.flushLayers();
  expect(gfx.textures[0]?.uploads).toBe(moved);
  fx.update(20, 1); // t 0.036 s: frame 1
  expect(gfx.textures[0]?.uploads).toBe(moved + 1);
});

test("members keep the effect API: complete, stop, destroy, follow, setDepths no-op", () => {
  const { gfx, fx } = setup({ color: 0xff0000ff, light: 0 });
  const layer = fx.createLayer(BOUNDS);
  const a = layer.play("fake/spark", 104, 205);
  const b = layer.play("fake/spark", 104, 205);
  let completes = 0;
  let destroys = 0;
  a.on("complete", () => (completes += 1));
  b.on("destroy", () => (destroys += 1));
  expect(a.setDepths(9, 9)).toBe(a);
  expect(gfx.images.map((i) => i.depth)).toEqual([0, 0, 1, 1]);
  a.stop();
  b.destroy();
  expect(destroys).toBe(1);
  fx.flushLayers();
  expect(gfx.textures[0]?.px(0, 0)).toEqual([255, 0, 0, 255]);
  fx.update(600, 1);
  expect(completes).toBe(1);
  expect(fx.stats()).toEqual({ engines: 0, textures: 4, images: 4 });
  expect(gfx.textures[0]?.written()).toBe(false);
});

test("layer destroy frees members and everything it holds", () => {
  const { engine, gfx, fx } = setup();
  const layer = fx.createLayer(BOUNDS);
  const e = layer.play("fake/spark", 105, 205);
  layer.play("fake/spark", 110, 205);
  let destroys = 0;
  e.on("destroy", () => (destroys += 1));
  layer.destroy();
  layer.destroy();
  expect(destroys).toBe(1);
  expect(fx.stats()).toEqual(EMPTY);
  expect(engine.live).toBe(0);
  expect(gfx.textures.every((t) => t.destroyed)).toBe(true);
  expect(gfx.images.every((i) => i.destroyed)).toBe(true);
  expect(() => layer.play("fake/spark", 0, 0)).toThrow(/destroyed/);
});

test("scene shutdown destroys layers", () => {
  const { fx } = setup();
  fx.createLayer(BOUNDS).play("fake/spark", 105, 205);
  fx.createLayer(BOUNDS);
  fx.destroyAll();
  expect(fx.stats()).toEqual(EMPTY);
});

test("play on a layer rejects unknown ids and invalid options without leaks", () => {
  const { engine, fx } = setup();
  const layer = fx.createLayer(BOUNDS);
  expect(() => layer.play("nope", 0, 0)).toThrow(/nope/);
  expect(() => layer.play("fake/spark", 0, 0, { seed: -1 })).toThrow(/seed/);
  expect(fx.stats()).toEqual({ engines: 0, textures: 4, images: 4 });
  expect(engine.live).toBe(0);
});

test("layer destroy with three members uploads nothing", () => {
  const { gfx, fx } = setup({ color: 0xff0000ff, light: 0 });
  const layer = fx.createLayer(BOUNDS);
  for (let i = 0; i < 3; i++) layer.play("fake/spark", 105 + i, 205);
  const before = gfx.textures.reduce((n, t) => n + t.uploads, 0);
  layer.destroy();
  expect(gfx.textures.reduce((n, t) => n + t.uploads, 0)).toBe(before);
  expect(fx.stats()).toEqual(EMPTY);
});

test("a complete listener that destroys the layer leaves its textures untouched", () => {
  const { gfx, fx } = setup({ color: 0xff0000ff, light: 0 });
  const layer = fx.createLayer(BOUNDS);
  const a = layer.play("fake/spark", 105, 205);
  layer.play("fake/spark", 106, 205);
  a.on("complete", () => layer.destroy());
  fx.update(1100, 1);
  expect(gfx.textures.reduce((n, t) => n + t.uploadsWhenDestroyed, 0)).toBe(0);
  expect(gfx.textures.every((t) => t.destroyed)).toBe(true);
  expect(fx.stats()).toEqual(EMPTY);
});

test("k plays and moves in one frame cost one redraw (one upload per texture)", () => {
  const { gfx, fx } = setup({ color: 0xff0000ff, light: 0 });
  const layer = fx.createLayer(BOUNDS);
  const members = Array.from({ length: 5 }, (_, i) => layer.play("fake/spark", 105 + i, 205));
  for (const m of members) m.setPosition(106, 205);
  expect(gfx.uploads()).toBe(0);
  fx.flushLayers();
  expect(gfx.textures.map((t) => t.uploads)).toEqual([1, 1, 1, 1]);
  fx.flushLayers();
  expect(gfx.uploads()).toBe(4);
});

test("update flushes once at its end, leaving nothing for the pre-render flush", () => {
  const { gfx, fx } = setup({ color: 0xff0000ff, light: 0 });
  const layer = fx.createLayer(BOUNDS);
  layer.play("fake/spark", 105, 205);
  layer.play("fake/spark", 106, 205);
  fx.update(16, 1);
  expect(gfx.textures.map((t) => t.uploads)).toEqual([1, 1, 1, 1]);
  fx.flushLayers();
  expect(gfx.uploads()).toBe(4);
});

test("a play while the scene is paused (no update) shows after the pre-render flush", () => {
  const { gfx, fx } = setup({ color: 0xff0000ff, light: 0 });
  const layer = fx.createLayer(BOUNDS);
  layer.play("fake/spark", 104, 205);
  expect(gfx.textures[0]?.written()).toBe(false);
  fx.flushLayers();
  expect(gfx.textures[0]?.px(0, 0)).toEqual([255, 0, 0, 255]);
});

test("a destroyed layer never flushes", () => {
  const { gfx, fx } = setup({ color: 0xff0000ff, light: 0 });
  const layer = fx.createLayer(BOUNDS);
  layer.play("fake/spark", 104, 205);
  layer.destroy();
  fx.flushLayers();
  expect(gfx.textures.reduce((n, t) => n + t.uploadsWhenDestroyed, 0)).toBe(0);
  expect(gfx.uploads()).toBe(0);
});
