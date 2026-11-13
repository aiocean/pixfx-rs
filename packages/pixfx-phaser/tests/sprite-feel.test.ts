import { expect, test } from "bun:test";
import { type FeelSprite, PixfxSpriteFeel, SpriteFeelCore } from "../src/sprite-feel";
import { FakeBlink, FakeSquash } from "./fake-engine";

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

function feel(s: FeelSprite) {
  const squash = new FakeSquash();
  const blink = new FakeBlink();
  const gone: PixfxSpriteFeel[] = [];
  const f = new PixfxSpriteFeel(s, new SpriteFeelCore(squash, blink), (g) => gone.push(g));
  return { f, squash, blink, gone };
}

test("a sprite at rest keeps its own scale; moving, it stretches over that scale", () => {
  const s = sprite();
  const { f } = feel(s);
  f.step(0.1);
  expect(s.drawn.at(-1)).toEqual([2, 2, 1]);
  s.y = 90;
  f.step(0.1);
  expect(s.drawn.at(-1)).toEqual([2 / 1.5, 3, 1]);
});

test("its body's velocity says how fast and which way it moves when it has one", () => {
  const s = sprite({ velocity: { x: 0, y: 300 } });
  const { f } = feel(s);
  f.step(0.1);
  expect(s.drawn.at(-1)).toEqual([2 / 1.5, 3, 1]);
});

test("moving sideways stretches it wide", () => {
  const dashing = sprite({ velocity: { x: -400, y: 20 } });
  feel(dashing).f.step(0.1);
  expect(dashing.drawn.at(-1)).toEqual([3, 2 / 1.5, 1]);

  const s = sprite();
  const { f } = feel(s);
  f.step(0.1);
  s.x = 80;
  f.step(0.1);
  expect(s.drawn.at(-1)).toEqual([3, 2 / 1.5, 1]);
});

test("landing squashes it until the squash eases out", () => {
  const s = sprite();
  const { f } = feel(s);
  f.land(1);
  f.step(0.05);
  expect(s.drawn.at(-1)).toEqual([4, 1, 1]);
  f.step(0.06);
  expect(s.drawn.at(-1)).toEqual([2, 2, 1]);
});

test("blinking draws it faint for as long as it lasts", () => {
  const s = sprite();
  const { f } = feel(s);
  f.blink(0.2);
  f.step(0.1);
  expect(f.isBlinking).toBe(true);
  expect(s.alpha).toBe(0.5);
  f.step(0.15);
  expect(f.isBlinking).toBe(false);
  expect(s.alpha).toBe(1);
});

test("a frame with no play time (a hit-stop) leaves it as drawn", () => {
  const s = sprite();
  const { f } = feel(s);
  s.y = 90;
  f.step(0.1);
  f.step(0.1);
  const n = s.drawn.length;
  f.step(0);
  expect(s.drawn.length).toBe(n);
});

test("ending it puts the sprite back at its base and frees the engine's side once", () => {
  const s = sprite();
  const { f, squash, blink, gone } = feel(s);
  f.blink(1);
  s.y = 90;
  f.step(0.1);
  f.destroy();
  f.destroy();
  expect([s.scaleX, s.scaleY, s.alpha]).toEqual([2, 2, 1]);
  expect(squash.freed && blink.freed).toBe(true);
  expect(gone).toEqual([f]);
});

test("a destroyed sprite ends its feel without being drawn again", () => {
  const s = sprite();
  const { f, squash, gone } = feel(s);
  s.active = false;
  f.step(0.1);
  expect(s.drawn).toEqual([]);
  expect(squash.freed).toBe(true);
  expect(gone).toEqual([f]);
});
