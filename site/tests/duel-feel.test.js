// The screen's answer to the fight, driven the way the hero drives it: what the duel did, then real time passing.
import { describe, expect, test } from "bun:test";
import { advance, createFeel, react } from "../assets/duel-feel.js";

const FRAME = 1 / 60;

/** Seconds of fight that `seconds` of real time buys. */
function fightTime(feel, seconds) {
  let t = 0;
  for (let i = 0; i < Math.round(seconds / FRAME); i++) t += advance(feel, FRAME);
  return t;
}

describe("feel", () => {
  test("a harder blow stops the fight longer and shakes the screen harder", () => {
    const lost = (hit) => {
      const feel = createFeel();
      react(feel, [{ type: "hit", by: 0, x: 0, y: 0, crit: false, ...hit }]);
      const trauma = feel.trauma;
      return [1 - fightTime(feel, 1), trauma];
    };
    const [lightStop, lightShake] = lost({ spell: "slash", power: 5, launch: false, share: 4 });
    const [heavyStop, heavyShake] = lost({ spell: "palm", power: 20, launch: true, share: 20 });
    expect(heavyStop).toBeGreaterThan(lightStop);
    expect(heavyShake).toBeGreaterThan(lightShake);
    // never past the cap, even a crit that throws
    expect(lost({ spell: "palm", power: 20, launch: true, crit: true, share: 30 })[0]).toBeLessThanOrEqual(13 / 60 + 1e-9);
  });
});
