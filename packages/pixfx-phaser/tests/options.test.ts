import { expect, test } from "bun:test";
import { parsePlayOptions } from "../src/options";

test("Phaser's options are read here; the rest go to the engine as given", () => {
  const p = parsePlayOptions({ seed: 7, depth: { back: 1, front: 2 }, scrollFactor: 0.5, density: 3, palette: "ice" });
  expect(p).toEqual({ scrollFactor: 0.5, depth: { back: 1, front: 2 }, engine: { seed: 7, density: 3, palette: "ice" } });
});

test("with no options an effect scrolls with the world and leaves its depths to the default", () => {
  expect(parsePlayOptions(undefined)).toEqual({ scrollFactor: 1, depth: undefined, engine: {} });
});

test("a bad Phaser option throws, naming it", () => {
  expect(() => parsePlayOptions({ scrollFactor: "fast" })).toThrow(/scrollFactor/);
  expect(() => parsePlayOptions({ depth: { back: 1 } })).toThrow(/depth/);
  expect(() => parsePlayOptions(5)).toThrow(/options/);
});
