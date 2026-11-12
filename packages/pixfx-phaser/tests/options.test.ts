import { expect, test } from "bun:test";
import { engineOptions, parsePlayOptions } from "../src/options";

test("undefined gives defaults", () => {
  expect(parsePlayOptions(undefined)).toEqual({ scrollFactor: 1 });
});

test("bad fields throw naming the field", () => {
  expect(() => parsePlayOptions({ seed: -1 })).toThrow(/seed/);
  expect(() => parsePlayOptions({ palette: [1] })).toThrow(/palette/);
  expect(() => parsePlayOptions({ scrollFactor: "x" })).toThrow(/scrollFactor/);
});

test("extras pass through", () => {
  const p = parsePlayOptions({ density: 3, tints: [1, 2] });
  expect(p).toMatchObject({ density: 3, tints: [1, 2], scrollFactor: 1 });
});

test("engineOptions strips scrollFactor/depth and adds margin", () => {
  const p = parsePlayOptions({ seed: 7, depth: { back: 1, front: 2 }, scrollFactor: 0.5, density: 3 });
  expect(engineOptions(p, 4)).toEqual({ seed: 7, density: 3, margin: 4 });
});
