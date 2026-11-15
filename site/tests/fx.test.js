// What the duel tells the shells to play: a looping effect is always stopped, and an effect sits where its art stands.
import { describe, expect, test } from "bun:test";
import { SPELLS, createDuel, slotPower, step } from "../assets/duel.js";
import { Body, Mind, effects } from "./engine.js";

const { meta } = await import(new URL("../../packages/pixfx-web/wasm/pixfx_wasm.js", import.meta.url).href);
const all = new Set(effects());
const loops = (fx) => all.has(fx) && !!meta(fx).loop;
const idle = () => ({ think: () => null, tell() {}, reset() {}, hold() {} });

/** Fighter 0 casts `spell` once from its bar; the foe stands still 30 px away. Returns the duel and its fighters. */
function castOnce(spell) {
  let cast = false;
  const caster = () => ({ think: () => (cast ? null : ((cast = true), { type: "cast", skill: spell })), tell() {}, reset() {}, hold() {} });
  const duel = createDuel({ seed: 3, left: 0, right: 220, ground: 150, has: (id) => all.has(id), Body, Mind, minds: [caster, idle] });
  const [me, foe] = duel.fighters;
  me.bar = [{ spell, casts: 2, left: 2, rarity: "common", evolved: false, traits: [], power: slotPower(2, "common", [], false) }, { empty: 9, full: 9 }, { empty: 9, full: 9 }];
  if (!me.pool.includes(spell)) me.pool.push(spell);
  if (!me.spells.includes(spell)) me.spells.push(spell);
  me.qi = 100;
  foe.x = me.x + 30;
  me.face = 1;
  return { duel, me, foe };
}

describe("looping effects", () => {
  test("the water jet sprays for a moment and stops", () => {
    const { duel } = castOnce("jet");
    let key = null;
    let stopped = null;
    for (let i = 0; i < 3 * 60 && stopped === null; i++) {
      for (const c of step(duel, 1 / 60)) {
        if (c.type === "play" && c.fx === SPELLS.jet.fx) key = c.key;
        if (c.type === "stop" && c.key === key) stopped = duel.time;
      }
    }
    expect(key).not.toBeNull();
    expect(stopped).not.toBeNull();
  });

  test("in whole fights, every looping effect played is stopped within 12 s", () => {
    for (let seed = 1; seed <= 8; seed++) {
      const duel = createDuel({ seed, left: 0, right: 220, ground: 150, has: (id) => all.has(id), Body, Mind });
      const live = new Map();
      for (let i = 0; i < 90 * 60; i++) {
        for (const c of step(duel, 1 / 60)) {
          if (c.type === "play" && loops(c.fx)) live.set(c.key, { fx: c.fx, at: duel.time });
          if (c.type === "stop") live.delete(c.key);
        }
        for (const v of live.values()) expect([seed, v.fx, duel.time - v.at < 12]).toEqual([seed, v.fx, true]);
      }
    }
  }, 120_000);
});

describe("where effects stand", () => {
  test("the hex shield's dome stands on the caster's feet", () => {
    const { duel, me } = castOnce("hex");
    let at = null;
    for (let i = 0; i < 2 * 60 && at === null; i++) {
      for (const c of step(duel, 1 / 60)) if (c.type === "play" && c.fx === SPELLS.hex.fx) at = { y: c.y, feet: duel.ground - me.air };
    }
    expect(at).not.toBeNull();
    expect(at.y).toBe(at.feet);
  });
});
