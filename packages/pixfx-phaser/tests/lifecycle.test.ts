import { expect, test } from "bun:test";
import { EffectCore } from "../src/lifecycle";
import { FakeEngine } from "./fake-engine";

function make(x = 0, y = 0) {
  const engine = new FakeEngine();
  const core = new EffectCore(engine.create("fake/spark", {}), x, y);
  return { engine, core };
}

test("completes exactly once", () => {
  const { core } = make();
  expect(core.tick(0.5)).toBe("running");
  expect(core.tick(0.6)).toBe("completed");
  expect(core.state).toBe("done");
  expect(core.tick(0.1)).toBe("running");
  expect(core.state).toBe("done");
});

test("stop on a done effect does nothing", () => {
  const { core } = make();
  core.tick(2);
  core.stop();
  expect(core.state).toBe("done");
});

test("stop on a playing effect moves to stopping and still completes", () => {
  const { core } = make();
  core.stop();
  expect(core.state).toBe("stopping");
  expect(core.tick(0.1)).toBe("running");
  expect(core.tick(0.5)).toBe("completed");
});

test("destroy frees once", () => {
  const { engine, core } = make();
  core.destroy();
  core.destroy();
  expect(engine.live).toBe(0);
  expect(core.state).toBe("destroyed");
});

test("follow tracks target plus offset each tick", () => {
  const { core } = make();
  const target = { x: 10, y: 20, active: true };
  core.follow(target, 1, 2);
  core.tick(0.1);
  expect(core.topLeft()).toEqual({ x: 11 - 4, y: 22 - 5 });
  target.x = 50;
  core.tick(0.1);
  expect(core.topLeft()).toEqual({ x: 51 - 4, y: 22 - 5 });
});

test("inactive target stops the effect and freezes position", () => {
  const { core } = make();
  const target = { x: 10, y: 20, active: true };
  core.follow(target, 0, 0);
  core.tick(0.1);
  target.active = false;
  target.x = 99;
  core.tick(0.1);
  expect(core.state).toBe("stopping");
  expect(core.topLeft()).toEqual({ x: 10 - 4, y: 20 - 5 });
});

test("topLeft snaps the position", () => {
  const { core } = make(10.6, 0);
  expect(core.topLeft().x).toBe(11 - 4);
});

test("unknown id throws and counts no instance", () => {
  const engine = new FakeEngine();
  expect(() => engine.create("nope/none", {})).toThrow(/nope\/none/);
  expect(engine.live).toBe(0);
});
