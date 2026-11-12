import { expect, test } from "bun:test";
import { defaultWorkers } from "../src/load";

test("default worker count: one per core the main thread leaves free, 1 to 4", () => {
  expect(defaultWorkers(undefined)).toBe(1);
  expect(defaultWorkers(1)).toBe(1);
  expect(defaultWorkers(2)).toBe(1);
  expect(defaultWorkers(4)).toBe(3);
  expect(defaultWorkers(10)).toBe(4);
});
