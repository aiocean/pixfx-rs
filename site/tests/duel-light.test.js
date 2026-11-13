// The light the duel throws on its world, fed the way the hero feeds it: the duel's commands, then time passing.
import { describe, expect, test } from "bun:test";
import { createLights, fade, kindle } from "../assets/duel-light.js";

const GROUND = 150;
const GOLD = ["#000000", "#402000", "#805000", "#c08000", "#ffc000", "#ffe080", "#ffffff"];
const oneShot = () => false;

describe("light", () => {
  test("a spell lights its spot in its palette's colour, then goes out", () => {
    const lights = createLights();
    kindle(lights, [{ type: "play", key: 1, fx: "explosions/plasma-explosion", x: 80, y: 120, opts: { palette: GOLD } }], GROUND, oneShot);
    expect(lights.list).toHaveLength(1);
    expect(lights.list[0]).toMatchObject({ x: 80, y: 120, color: GOLD[4] });
    fade(lights, 1);
    expect(lights.list).toEqual([]);
  });

  test("the scene's own effects, which carry no palette, cast no light", () => {
    const lights = createLights();
    kindle(lights, [{ type: "play", key: 1, fx: "critters/wisp", x: 10, y: 10, opts: { seed: 3 } }], GROUND, oneShot);
    expect(lights.list).toEqual([]);
  });

  test("a looping spell's light follows it and lasts until it is stopped", () => {
    const lights = createLights();
    const loop = () => true;
    kindle(lights, [{ type: "play", key: 7, fx: "fire/spirit-flame", x: 50, y: 140, opts: { palette: GOLD } }], GROUND, loop);
    fade(lights, 5);
    kindle(lights, [{ type: "move", key: 7, x: 70, y: 140 }], GROUND, loop);
    expect(lights.list[0]).toMatchObject({ x: 70, y: 140 });
    kindle(lights, [{ type: "stop", key: 7 }], GROUND, loop);
    fade(lights, 0.5);
    expect(lights.list).toEqual([]);
  });

  test("a blow flashes where it lands, measured up from the ground", () => {
    const lights = createLights();
    kindle(lights, [{ type: "hit", by: 0, damage: 300, share: 3, x: 90, y: 20 }], GROUND, oneShot);
    expect(lights.list[0]).toMatchObject({ x: 90, y: GROUND - 20 });
    fade(lights, 0.3);
    expect(lights.list).toEqual([]);
  });
});
