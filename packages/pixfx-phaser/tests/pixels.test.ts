import { expect, test } from "bun:test";
import { snap } from "../src/pixels";

test("snap rounds to whole pixels", () => {
  expect(snap(10.6)).toBe(11);
  expect(snap(10.4)).toBe(10);
});
