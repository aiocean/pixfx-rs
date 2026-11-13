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

test("every effect takes an outline: on or off, a word, or a ring, handed to the engine as given", () => {
  for (const outline of [true, false, "off", "white", "#ff00aa", { color: "black", width: 2, corners: true }]) {
    expect(engineOptions(parsePlayOptions({ outline }), 0)).toEqual({ outline, margin: 0 });
  }
  expect(() => parsePlayOptions({ outline: { width: 4 } })).toThrow(/outline/);
  expect(() => parsePlayOptions({ outline: { colour: "#fff" } })).toThrow(/outline/);
  expect(() => parsePlayOptions({ outline: 2 })).toThrow(/outline/);
});

test("a palette is a preset's name or a list of colours", () => {
  expect(engineOptions(parsePlayOptions({ palette: "ice" }), 0)).toEqual({ palette: "ice", margin: 0 });
  expect(engineOptions(parsePlayOptions({ palette: ["#000", "#fff"] }), 0)).toEqual({ palette: ["#000", "#fff"], margin: 0 });
  expect(() => parsePlayOptions({ palette: 3 })).toThrow(/palette/);
  expect(() => parsePlayOptions({ palette: "lava" })).toThrow(/palette/);
});
