import { expect, test } from "bun:test";
import { seedsParam, stressSeed } from "../demo/seeds";

test("unique seeds give every spawned effect its own seed", () => {
  const seeds = [0, 1, 2, 3].map((spawned) => stressSeed("unique", spawned, spawned % 3));
  expect(new Set(seeds).size).toBe(4);
});

test("shared seeds give every effect of one id the same seed, and ids different seeds", () => {
  expect(stressSeed("shared", 0, 1)).toBe(stressSeed("shared", 7, 1));
  expect(stressSeed("shared", 0, 0)).not.toBe(stressSeed("shared", 0, 1));
});

test("the seeds parameter is shared only when asked, unique otherwise", () => {
  expect(seedsParam("shared")).toBe("shared");
  expect(seedsParam(null)).toBe("unique");
  expect(seedsParam("whatever")).toBe("unique");
});
