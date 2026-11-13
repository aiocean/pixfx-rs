import { expect, test } from "bun:test";
import { engineOptions, parsePlayOptions } from "../src/options";

test("engineOptions strips scrollFactor/depth and adds margin", () => {
  const p = parsePlayOptions({ seed: 7, depth: { back: 1, front: 2 }, scrollFactor: 0.5, density: 3 });
  expect(engineOptions(p, 4)).toEqual({ seed: 7, density: 3, margin: 4 });
});
