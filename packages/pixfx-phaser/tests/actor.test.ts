import { expect, test } from "bun:test";
import { actorPlace } from "../src/actor";

// a 16 x 24 frame, trimmed to 10 x 20 cut at (100, 50) of its texture, 4 px in and 3 px down its untrimmed box
const frame = { cutX: 100, cutY: 50, cutWidth: 10, cutHeight: 20, x: 4, y: 3, realWidth: 16 };
// origin at its feet: the middle of the bottom of the untrimmed box
const sprite = { x: 200, y: 300, scaleX: 1, scaleY: 1, flipX: false, displayOriginX: 8, displayOriginY: 24 };

test("a frame shows its trimmed pixels where Phaser draws them, cut from where its texture holds them", () => {
  const { at, cut } = actorPlace(sprite, frame);
  expect(at).toEqual({ x: 196, y: 279, w: 10, h: 20, shownW: 10, shownH: 20, flipX: false });
  expect(cut).toEqual({ x: 100, y: 50, w: 10, h: 20 });
});

test("a flipped sprite's trimmed pixels mirror about its origin", () => {
  // untrimmed box 192..208; unflipped pixels 196..206 mirror to 194..204
  expect(actorPlace({ ...sprite, flipX: true }, frame).at).toMatchObject({ x: 194, w: 10, flipX: true });
});

test("a scaled sprite shows its pixels at its scale", () => {
  const { at } = actorPlace({ ...sprite, scaleX: 2, scaleY: 2 }, frame);
  expect(at).toMatchObject({ x: 192, y: 258, w: 10, h: 20, shownW: 20, shownH: 40 });
});
