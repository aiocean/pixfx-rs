import { expect, mock, test } from "bun:test";
import EventEmitter from "eventemitter3";
import type { Backend } from "../src/backend";
import { InlineBackend } from "../src/inline";
import { PaintedEngine } from "./fake-engine";
import { FakeGfx } from "./fake-gfx";

void mock.module("phaser", () => ({ default: { Events: { EventEmitter } } }));
const { EffectManager } = await import("../src/effects");

const BOUNDS = { bounds: { x: 100, y: 200, width: 20, height: 10 } };

function setup(paint = { color: 0, light: 0 }) {
  const engine = new PaintedEngine();
  engine.paint = paint;
  const gfx = new FakeGfx();
  const backend = new InlineBackend(engine);
  const fx = new EffectManager(gfx, (): Backend => backend, "pixfx:test:");
  return { engine, gfx, fx };
}

test("colour composites source-over; light adds and saturates", () => {
  const engine = new PaintedEngine();
  const gfx = new FakeGfx();
  const backend = new InlineBackend(engine);
  const fx = new EffectManager(gfx, (): Backend => backend, "pixfx:test:");
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
