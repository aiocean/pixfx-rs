// The screen's answer to the fight, driven the way the hero drives it: what the duel did, then real time passing.
import { describe, expect, test } from "bun:test";
import { advance, createFeel, numbers, react, shake } from "../assets/duel-feel.js";

const FRAME = 1 / 60;

/** Seconds of fight that `seconds` of real time buys. */
function fightTime(feel, seconds) {
  let t = 0;
  for (let i = 0; i < Math.round(seconds / FRAME); i++) t += advance(feel, FRAME);
  return t;
}

describe("feel", () => {
  test("a blow stops the fight for a few frames, then it runs on at full speed", () => {
    const feel = createFeel();
    react(feel, [{ type: "hit", by: 0, damage: 12, x: 50, y: 20 }]);
    expect(advance(feel, FRAME)).toBe(0);
    const stopped = 0.5 - fightTime(feel, 0.5);
    expect(stopped).toBeGreaterThan(0.04);
    expect(stopped).toBeLessThan(0.13);
    expect(advance(feel, FRAME)).toBe(FRAME);
  });

  test("a harder blow stops the fight longer and shakes the screen harder", () => {
    const lost = (damage) => {
      const feel = createFeel();
      react(feel, [{ type: "hit", by: 0, damage, x: 0, y: 0 }]);
      const trauma = feel.trauma;
      return [1 - fightTime(feel, 1), trauma];
    };
    const [lightStop, lightShake] = lost(4);
    const [heavyStop, heavyShake] = lost(20);
    expect(heavyStop).toBeGreaterThan(lightStop);
    expect(heavyShake).toBeGreaterThan(lightShake);
  });

  test("a knockout plays out in slow motion, then the fight runs at full speed", () => {
    const feel = createFeel();
    react(feel, [{ type: "ko", x: 0, y: 0 }]);
    expect(fightTime(feel, 1)).toBeLessThan(0.5);
    fightTime(feel, 1);
    expect(advance(feel, FRAME)).toBe(FRAME);
  });

  test("the screen settles after a shake", () => {
    const feel = createFeel();
    react(feel, [{ type: "hit", by: 0, damage: 25, x: 0, y: 0 }]);
    const most = () => shake(feel, () => 0.999).map(Math.abs);
    expect(Math.max(...most())).toBeGreaterThan(0);
    fightTime(feel, 1);
    expect(most()).toEqual([0, 0]);
  });

  test("damage floats up from the blow and fades away", () => {
    const feel = createFeel();
    react(feel, [{ type: "hit", by: 1, damage: 9.4, x: 80, y: 20 }]);
    const [first] = numbers(feel);
    expect(first).toMatchObject({ x: 80, text: "9", by: 1 });
    fightTime(feel, 0.4);
    const [later] = numbers(feel);
    expect(later.y).toBeGreaterThan(first.y);
    fightTime(feel, 1);
    expect(numbers(feel)).toEqual([]);
  });
});
