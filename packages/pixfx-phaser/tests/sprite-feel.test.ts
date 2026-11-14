import { expect, test } from "bun:test";
import { type FeelSprite, PixfxSpriteFeel, SpriteFeelCore } from "../src/sprite-feel";
import { wasmEngine } from "../src/engine";
import { wasm } from "./wasm";

/** A sprite drawn at scale 2 that records what it is drawn at. */
function sprite(body?: unknown): FeelSprite & { drawn: [number, number, number][] } {
  const drawn: [number, number, number][] = [];
  const s = {
    x: 50,
    y: 100,
    scaleX: 2,
    scaleY: 2,
    alpha: 1,
    active: true,
    body,
    drawn,
    setScale(x: number, y: number) {
      s.scaleX = x;
      s.scaleY = y;
      s.drawn.push([x, y, s.alpha]);
      return s;
    },
    setAlpha(a: number) {
      s.alpha = a;
      const last = s.drawn.at(-1);
      if (last) last[2] = a;
      return s;
    },
  };
  return s;
}

const engine = wasmEngine(wasm);

/** The engine's squash and blink, noting when they are freed. */
function feel(s: FeelSprite) {
  const freed: string[] = [];
  const sq = engine.squash();
  const bl = engine.blink();
  const squash = { ...sq, free: () => (freed.push("squash"), sq.free()) };
  const blink = { ...bl, free: () => (freed.push("blink"), bl.free()) };
  const gone: PixfxSpriteFeel[] = [];
  const f = new PixfxSpriteFeel(s, new SpriteFeelCore(squash, blink), (g) => gone.push(g));
  return { f, freed, gone };
}

const last = (s: { drawn: [number, number, number][] }): [number, number, number] => s.drawn.at(-1) ?? [Number.NaN, Number.NaN, Number.NaN];

test("a sprite at rest keeps its own scale; rising fast, it stretches tall over that scale", () => {
  const s = sprite();
  const { f } = feel(s);
  f.step(0.1);
  expect(last(s)).toEqual([2, 2, 1]);
  s.y = 60;
  f.step(0.1);
  const [x, y] = last(s);
  expect(y).toBeGreaterThan(2);
  expect(x).toBeLessThan(2);
});

test("its body's velocity says how it moves when it has one: sideways, it stretches wide", () => {
  const s = sprite({ velocity: { x: -600, y: 0 } });
  feel(s).f.step(0.1);
  const [x, y] = last(s);
  expect(x).toBeGreaterThan(2);
  expect(y).toBeLessThan(2);
});

test("landing squashes it until the squash eases out", () => {
  const s = sprite();
  const { f } = feel(s);
  f.land(1);
  f.step(0.02);
  const [x, y] = last(s);
  expect(x).toBeGreaterThan(2);
  expect(y).toBeLessThan(2);
  for (let i = 0; i < 60; i++) f.step(0.05);
  expect(last(s)).toEqual([2, 2, 1]);
});

test("blinking draws it fainter for as long as it lasts", () => {
  const s = sprite();
  const { f } = feel(s);
  f.blink(0.5);
  let faint = false;
  for (let i = 0; i < 5; i++) {
    f.step(0.05);
    faint ||= s.alpha < 1;
  }
  expect(f.isBlinking).toBe(true);
  expect(faint).toBe(true);
  for (let i = 0; i < 10; i++) f.step(0.1);
  expect(f.isBlinking).toBe(false);
  expect(s.alpha).toBe(1);
});

test("a frame with no play time (a hit-stop) holds its shape", () => {
  const s = sprite();
  const { f } = feel(s);
  f.step(0.1);
  s.y = 60;
  f.step(0.1);
  const held = last(s);
  s.y = 20;
  f.step(0);
  expect(last(s)).toEqual(held);
});

test("ending it puts the sprite back at its base and frees the engine's side once", () => {
  const s = sprite();
  const { f, freed, gone } = feel(s);
  f.blink(1);
  s.y = 60;
  f.step(0.1);
  f.destroy();
  f.destroy();
  expect([s.scaleX, s.scaleY, s.alpha]).toEqual([2, 2, 1]);
  expect(freed.sort()).toEqual(["blink", "squash"]);
  expect(gone).toEqual([f]);
});

test("a destroyed sprite ends its feel without being drawn again", () => {
  const s = sprite();
  const { f, freed, gone } = feel(s);
  s.active = false;
  f.step(0.1);
  expect(s.drawn).toEqual([]);
  expect(freed.sort()).toEqual(["blink", "squash"]);
  expect(gone).toEqual([f]);
});
