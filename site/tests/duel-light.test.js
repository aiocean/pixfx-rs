// The light the duel throws on its world: what the duel's commands ask of the engine's `Lights` (how lights fade
// and draw is pixfx-light's, tested there). The lights here keep what they are asked, as `Lights.list` would.
import { describe, expect, test } from "bun:test";
import { kindle, lampGlows } from "../assets/duel-light.js";

const GROUND = 150;
const GOLD = ["#000000", "#402000", "#805000", "#c08000", "#ffc000", "#ffe080", "#ffffff"];
const oneShot = () => false;

function lights() {
  const list = [];
  return {
    list,
    add: (l) => list.push({ key: null, life: null, ...l }),
    move: (key, x, y) => list.filter((l) => l.key === key).forEach((l) => Object.assign(l, { x, y })),
    putOut: (key, within) => list.filter((l) => l.key === key).forEach((l) => (l.life = within)),
  };
}

describe("light", () => {
  test("a spell lights its spot in its palette's colour, for a flash", () => {
    const l = lights();
    kindle(l, [{ type: "play", key: 1, fx: "explosions/plasma-explosion", x: 80, y: 120, opts: { palette: GOLD } }], GROUND, oneShot);
    expect(l.list).toEqual([{ key: 1, x: 80, y: 120, reach: 70, color: GOLD[4], strength: 0.42, life: 0.5 }]);
  });

  test("the scene's own effects, which carry no palette, cast no light", () => {
    const l = lights();
    kindle(l, [{ type: "play", key: 1, fx: "critters/wisp", x: 10, y: 10, opts: { seed: 3 } }], GROUND, oneShot);
    expect(l.list).toEqual([]);
  });

  test("a looping spell's light follows it and burns until it is stopped", () => {
    const l = lights();
    const loop = () => true;
    kindle(l, [{ type: "play", key: 7, fx: "fire/spirit-flame", x: 50, y: 140, opts: { palette: GOLD } }], GROUND, loop);
    expect(l.list[0].life).toBeNull();
    kindle(l, [{ type: "move", key: 7, x: 70, y: 140 }], GROUND, loop);
    expect(l.list[0]).toMatchObject({ x: 70, y: 140 });
    kindle(l, [{ type: "stop", key: 7 }], GROUND, loop);
    expect(l.list[0].life).toBe(0.2);
  });

  test("a blow flashes where it lands, measured up from the ground", () => {
    const l = lights();
    kindle(l, [{ type: "hit", by: 0, damage: 300, share: 3, x: 90, y: 20 }], GROUND, oneShot);
    expect(l.list[0]).toMatchObject({ key: null, x: 90, y: GROUND - 20, life: 0.16 });
  });

  test("each brazier glows warm over its lamp, flickering", () => {
    const [a, b] = lampGlows([20, 300], GROUND, 1.5);
    expect(a).toMatchObject({ x: 20, y: GROUND - 18, color: "#ff9a3c" });
    expect(b.x).toBe(300);
    expect(a.reach).not.toBe(lampGlows([20], GROUND, 1.6)[0].reach);
  });
});
