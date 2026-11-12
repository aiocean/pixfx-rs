import { expect, test } from "bun:test";
import { EffectCore } from "../src/lifecycle";

// fake/spark's origin on its canvas.
const ORIGIN = { originX: 4, originY: 5 };

function make(x = 0, y = 0) {
  return new EffectCore(ORIGIN, x, y);
}

test("stop moves playing to stopping, once", () => {
  const core = make();
  expect(core.stop()).toBe(true);
  expect(core.state).toBe("stopping");
  expect(core.stop()).toBe(false);
});

test("destroy happens once, from any state; a destroyed effect cannot stop", () => {
  const core = make();
  core.stop();
  expect(core.destroy()).toBe(true);
  expect(core.destroy()).toBe(false);
  expect(core.state).toBe("destroyed");
  expect(core.stop()).toBe(false);
});

test("track follows the target plus offset", () => {
  const core = make();
  const target = { x: 10, y: 20, active: true };
  core.follow(target, 1, 2);
  expect(core.track()).toBe(true);
  expect(core.topLeft()).toEqual({ x: 11 - 4, y: 22 - 5 });
  target.x = 50;
  core.track();
  expect(core.topLeft()).toEqual({ x: 51 - 4, y: 22 - 5 });
});

test("an inactive target is dropped once and the position freezes", () => {
  const core = make();
  const target = { x: 10, y: 20, active: true };
  core.follow(target, 0, 0);
  core.track();
  target.active = false;
  target.x = 99;
  expect(core.track()).toBe(false);
  expect(core.topLeft()).toEqual({ x: 10 - 4, y: 20 - 5 });
  expect(core.track()).toBe(true);
});

test("topLeft snaps the position", () => {
  expect(make(10.6, 0).topLeft().x).toBe(11 - 4);
});

test("moveTo places the effect and drops the follow target", () => {
  const core = make();
  const target = { x: 10, y: 20, active: true };
  core.follow(target, 0, 0);
  core.moveTo(30.4, 40);
  expect(core.topLeft()).toEqual({ x: 30 - 4, y: 40 - 5 });
  core.track();
  expect(core.topLeft()).toEqual({ x: 30 - 4, y: 40 - 5 });
  expect(core.state).toBe("playing");
});
