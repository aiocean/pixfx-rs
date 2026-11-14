// The light the duel throws on its world: the list of lights the duel's commands make (how lights fade and draw
// is pixfx-light's, tested there). Putting a light out here sets its life, as the engine's `putOutLight` does for
// a light that has not lived yet.
import { describe, expect, test } from "bun:test";
import { kindle, lampGlows } from "../assets/duel-light.js";

const GROUND = 150;
const GOLD = ["#000000", "#402000", "#805000", "#c08000", "#ffc000", "#ffe080", "#ffffff"];
const oneShot = () => false;

/** The lights after `commands`, from `lights`. */
function after(lights, commands, isLoop = oneShot) {
  return kindle(lights, commands, GROUND, isLoop, (l, within) => ({ ...l, life: within }));
}

describe("light", () => {
  test("a spell lights its spot in its palette's colour, for a flash", () => {
    const l = after([], [{ type: "play", key: 1, fx: "explosions/plasma-explosion", x: 80, y: 120, opts: { palette: GOLD } }]);
    expect(l).toEqual([{ key: 1, x: 80, y: 120, reach: 70, color: GOLD[4], strength: 0.42, life: 0.5 }]);
  });

  test("the scene's own effects, which carry no palette, cast no light", () => {
    expect(after([], [{ type: "play", key: 1, fx: "critters/wisp", x: 10, y: 10, opts: { seed: 3 } }])).toEqual([]);
  });

  test("a looping spell's light follows it and burns until it is stopped", () => {
    const loop = () => true;
    let l = after([], [{ type: "play", key: 7, fx: "fire/spirit-flame", x: 50, y: 140, opts: { palette: GOLD } }], loop);
    expect(l[0].life).toBeNull();
    l = after(l, [{ type: "move", key: 7, x: 70, y: 140 }], loop);
    expect(l[0]).toMatchObject({ x: 70, y: 140 });
    l = after(l, [{ type: "stop", key: 7 }], loop);
    expect(l[0].life).toBe(0.2);
  });

  test("a blow flashes where it lands, measured up from the ground", () => {
    const l = after([], [{ type: "hit", by: 0, damage: 300, share: 3, x: 90, y: 20 }]);
    expect(l[0]).toMatchObject({ key: null, x: 90, y: GROUND - 20, life: 0.16 });
  });

  test("each brazier glows warm over its lamp, flickering", () => {
    const [a, b] = lampGlows([20, 300], GROUND, 1.5);
    expect(a).toMatchObject({ x: 20, y: GROUND - 18, color: "#ff9a3c" });
    expect(b.x).toBe(300);
    expect(a.reach).not.toBe(lampGlows([20], GROUND, 1.6)[0].reach);
  });
});
