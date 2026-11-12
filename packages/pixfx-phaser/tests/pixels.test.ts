import { expect, test } from "bun:test";
import { colorToRgba, lightToRgba, snap } from "../src/pixels";

test("lightToRgba: lit pixels opaque, unlit transparent", () => {
  const out = new Uint8ClampedArray(8);
  lightToRgba(new Uint32Array([0x00000000, 0x00102030]), out);
  expect([...out]).toEqual([0, 0, 0, 0, 0x30, 0x20, 0x10, 255]);
});

test("lightToRgba reports whether anything is lit", () => {
  const out = new Uint8ClampedArray(4);
  expect(lightToRgba(new Uint32Array([0]), out)).toBe(false);
  expect(lightToRgba(new Uint32Array([0x00000001]), out)).toBe(true);
});

test("colorToRgba unpacks ABGR and reports visibility", () => {
  const out = new Uint8ClampedArray(4);
  expect(colorToRgba(new Uint32Array([0x80102030]), out)).toBe(true);
  expect([...out]).toEqual([0x30, 0x20, 0x10, 0x80]);
});

test("colorToRgba of all-zero words is false", () => {
  const out = new Uint8ClampedArray(8);
  expect(colorToRgba(new Uint32Array([0, 0]), out)).toBe(false);
  expect([...out]).toEqual([0, 0, 0, 0, 0, 0, 0, 0]);
});

test("snap rounds to whole pixels", () => {
  expect(snap(10.6)).toBe(11);
  expect(snap(10.4)).toBe(10);
});
