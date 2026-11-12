import { expect, test } from "bun:test";
import { FxRunner } from "../src/runner";
import { FakeEngine } from "./fake-engine";

// fake/spark: 8×6 pixels, 30 fps, lasts 1 s.
function make() {
  const engine = new FakeEngine();
  const fx = engine.create("fake/spark", {});
  return { engine, runner: new FxRunner(fx, 30) };
}

const BYTES = 8 * 6 * 4;

test("the first step draws frame 0 into four RGBA buffers: back colour, back light, front colour, front light", () => {
  const { runner } = make();
  const s = runner.step(0);
  if (s.kind !== "frame") throw new Error(`expected a frame, got ${s.kind}`);
  expect(s.frame.map((b) => b.byteLength)).toEqual([BYTES, BYTES, BYTES, BYTES]);
  // The fake stamps every byte with its draw count: back is draw 1, front is draw 2.
  expect(s.frame.map((b) => new Uint8ClampedArray(b)[0])).toEqual([1, 1, 2, 2]);
});

test("a step draws only when the effect reached a new frame of its fps", () => {
  const { runner } = make();
  runner.step(0);
  expect(runner.step(0.016).kind).toBe("same"); // t 0.016: frame 0
  expect(runner.step(0.02).kind).toBe("frame"); // t 0.036: frame 1
  expect(runner.step(0.016).kind).toBe("same"); // t 0.052: frame 1
  expect(runner.step(0.016).kind).toBe("frame"); // t 0.068: frame 2
});

test("a step that reaches the end reports done and draws nothing", () => {
  const { runner } = make();
  runner.step(0);
  expect(runner.step(1.1).kind).toBe("done");
});

test("stop takes effect at the next step: the effect fades, then is done", () => {
  const { runner } = make();
  runner.step(0);
  runner.stop();
  expect(runner.step(0.25).kind).not.toBe("done");
  expect(runner.step(0.25).kind).toBe("done");
});

test("given-back buffers are reused, so a long run allocates two sets", () => {
  const { runner } = make();
  let held = runner.step(0);
  for (let i = 0; i < 25; i++) {
    const s = runner.step(1 / 30);
    if (s.kind !== "frame") continue;
    if (held.kind === "frame") runner.give(held.frame);
    held = s;
  }
  expect(runner.allocated).toBe(2);
});

test("buffers of the wrong size or detached are dropped, not reused", () => {
  const { runner } = make();
  const s = runner.step(0);
  if (s.kind !== "frame") throw new Error("expected a frame");
  const detached = structuredClone(s.frame, { transfer: [...s.frame] });
  runner.give(s.frame); // now detached: byteLength 0
  runner.give([new ArrayBuffer(4), new ArrayBuffer(4), new ArrayBuffer(4), new ArrayBuffer(4)]);
  runner.step(1 / 30);
  expect(runner.allocated).toBe(2);
  runner.give(detached);
  runner.step(1 / 30);
  expect(runner.allocated).toBe(2);
});

test("free frees the engine instance once", () => {
  const { engine, runner } = make();
  runner.free();
  runner.free();
  expect(engine.live).toBe(0);
});
