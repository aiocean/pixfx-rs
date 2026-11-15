// What the HUD shows under each fighter's bars: its passives and what it carries, good first, then bad.
import { describe, expect, test } from "bun:test";
import { buffsOf, createDuel } from "../assets/duel.js";
import { Body, Mind, effects } from "./engine.js";

const all = new Set(effects());
const fresh = () => createDuel({ seed: 1, left: 0, right: 220, ground: 150, has: (id) => all.has(id), Body, Mind });
const ids = (f) => buffsOf(f).map((b) => b.id);

describe("buffs on the HUD", () => {
  test("a fresh fighter shows its school's passive: Thunder can stand up once, Frost charges rage from spells", () => {
    const [thunder, frost] = fresh().fighters;
    expect(buffsOf(thunder)).toEqual([{ id: "unyielding", good: true, stacks: 0 }]);
    expect(buffsOf(frost)).toEqual([{ id: "attune", good: true, stacks: 0 }]);
  });

  test("stacks show as numbers, and good ones come before bad ones", () => {
    const [thunder, frost] = fresh().fighters;
    Object.assign(thunder, { fury: 3, charges: 2, chill: 2, burn: { t: 1 }, shock: { stacks: 4 }, defDown: 1, silence: 1, sealed: 0.1, frozen: 0.5, grit: 1.6, barrier: 1 });
    expect(buffsOf(thunder)).toEqual([
      { id: "fury", good: true, stacks: 3 },
      { id: "unyielding", good: true, stacks: 0 },
      { id: "ward", good: true, stacks: 2 },
      { id: "barrier", good: true, stacks: 0 },
      { id: "grit", good: true, stacks: 1 },
      { id: "burn", good: false, stacks: 0 },
      { id: "chill", good: false, stacks: 2 },
      { id: "shock", good: false, stacks: 4 },
      { id: "frozen", good: false, stacks: 0 },
      { id: "defDown", good: false, stacks: 0 },
      { id: "silence", good: false, stacks: 0 },
      { id: "sealed", good: false, stacks: 0 },
    ]);
    frost.attune = 3;
    expect(buffsOf(frost)).toEqual([{ id: "attune", good: true, stacks: 1 }]);
  });

  test("an unyielding fighter that has stood up shows it no more; a knocked-out fighter shows nothing", () => {
    const [thunder, frost] = fresh().fighters;
    thunder.rose = true;
    expect(ids(thunder)).not.toContain("unyielding");
    frost.hp = 0;
    frost.chill = 2;
    expect(buffsOf(frost)).toEqual([]);
  });
});
