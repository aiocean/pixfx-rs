import { expect, test } from "bun:test";
import { engineOptions, parsePlayOptions } from "../src/options";

test("engineOptions strips scrollFactor/depth and adds margin", () => {
  const p = parsePlayOptions({ seed: 7, depth: { back: 1, front: 2 }, scrollFactor: 0.5, density: 3 });
  expect(engineOptions(p, 4)).toEqual({ seed: 7, density: 3, margin: 4 });
});

test("an effect's own option must be a number, colour, list of colours or flag, and is named when not", () => {
  const p = parsePlayOptions({ stone: "#7a6a58", ramp: ["#000", "#fff"], width: 640, wet: true });
  expect(engineOptions(p, 0)).toEqual({ stone: "#7a6a58", ramp: ["#000", "#fff"], width: 640, wet: true, margin: 0 });
  expect(() => parsePlayOptions({ width: { px: 640 } })).toThrow(/width/);
});
