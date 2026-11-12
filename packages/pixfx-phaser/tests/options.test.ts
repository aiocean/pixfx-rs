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

test("light is a non-negative glow strength", () => {
  expect(parsePlayOptions({ light: 0 })).toMatchObject({ light: 0 });
  expect(parsePlayOptions({ light: 1.5 })).toMatchObject({ light: 1.5 });
  expect(() => parsePlayOptions({ light: true })).toThrow(/light/);
  expect(() => parsePlayOptions({ light: -1 })).toThrow(/light/);
});

test("margin is rejected: the plugin sizes it", () => {
  expect(() => parsePlayOptions({ margin: 3 })).toThrow(/margin/);
});
