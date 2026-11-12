import { expect, test } from "bun:test";
import { clipRect } from "../src/layout";

const bounds = { x: 0, y: 0, w: 100, h: 100 };

test("effect fully inside is unchanged", () => {
  const r = { x: 10, y: 20, w: 30, h: 40 };
  expect(clipRect(bounds, r)).toEqual(r);
});

test("partial overlap is clipped", () => {
  expect(clipRect(bounds, { x: 90, y: -10, w: 30, h: 30 })).toEqual({ x: 90, y: 0, w: 10, h: 20 });
});

test("disjoint or touching gives null", () => {
  expect(clipRect(bounds, { x: 200, y: 0, w: 10, h: 10 })).toBeNull();
  expect(clipRect(bounds, { x: 100, y: 0, w: 10, h: 10 })).toBeNull();
});
