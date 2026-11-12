import { expect, test } from "bun:test";
import { stepSeconds } from "../src/clock";

test("converts ms to seconds scaled by timeScale", () => {
  expect(stepSeconds(16, 1)).toBe(0.016);
  expect(stepSeconds(16, 0.5)).toBe(0.008);
});

test("non-finite or non-positive inputs give 0", () => {
  expect(stepSeconds(16, 0)).toBe(0);
  expect(stepSeconds(16, -1)).toBe(0);
  expect(stepSeconds(16, NaN)).toBe(0);
  expect(stepSeconds(NaN, 1)).toBe(0);
  expect(stepSeconds(-5, 1)).toBe(0);
  expect(stepSeconds(Infinity, 1)).toBe(0);
});
