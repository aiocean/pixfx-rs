import { expect, test } from "bun:test";
import { actorPlace } from "../src/actor";

// a 16 x 24 texture, trimmed to 10 x 20 cut at (100, 50) of its source, 4 px in and 3 px down its untrimmed box
const texture = { frame: { x: 100, y: 50, width: 10, height: 20 }, orig: { width: 16, height: 24 }, trim: { x: 4, y: 3 } };
// anchored at its feet
const sprite = { x: 200, y: 300, anchor: { x: 0.5, y: 1 }, scale: { x: 1, y: 1 } };

test("a texture shows its trimmed pixels where Pixi draws them, cut from where its source holds them", () => {
  const { at, cut } = actorPlace(sprite, texture);
  expect(at).toEqual({ x: 196, y: 279, w: 10, h: 20, shownW: 10, shownH: 20, flipX: false });
  expect(cut).toEqual({ x: 100, y: 50, w: 10, h: 20 });
});

test("a negative x scale mirrors the trimmed pixels about the anchor", () => {
  expect(actorPlace({ ...sprite, scale: { x: -1, y: 1 } }, texture).at).toMatchObject({ x: 194, w: 10, flipX: true });
});

test("an untrimmed texture scaled up shows every pixel at the sprite's scale", () => {
  const { at } = actorPlace({ ...sprite, scale: { x: 2, y: 2 } }, { ...texture, trim: null });
  expect(at).toMatchObject({ x: 184, y: 252, shownW: 20, shownH: 40 });
});
