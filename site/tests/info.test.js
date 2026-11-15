// What the duel's tooltips say about a skill on the bar and a buff under the bars.
import { describe, expect, test } from "bun:test";
import { RARITY, SCHOOLS, SPELLS, TRAITS, buffsOf, createDuel } from "../assets/duel.js";
import { BUFF_INFO, SPELL_INFO, TRAIT_INFO, buffInfo, skillInfo } from "../assets/duel-info.js";
import { Body, Mind, effects } from "./engine.js";

const all = new Set(effects());
const fresh = () => createDuel({ seed: 2, left: 0, right: 220, ground: 150, has: (id) => all.has(id), Body, Mind });

describe("tooltips", () => {
  test("every skill a school can cast has a name and a line saying what it does", () => {
    for (const school of Object.values(SCHOOLS)) {
      for (const name of [...school.basics, ...school.pool, school.ultimate]) {
        expect([name, typeof SPELL_INFO[name]?.name, typeof SPELL_INFO[name]?.does]).toEqual([name, "string", "string"]);
      }
    }
    for (const t of Object.keys(TRAITS)) expect([t, typeof TRAIT_INFO[t]]).toEqual([t, "string"]);
  });

  test("a bar skill's tooltip: name and tier in the header, cost, cooldown and power as properties, traits as mods", () => {
    const f = fresh().fighters[0];
    f.bar[1] = { spell: "palm", rarity: "epic", traits: ["heavy", "swift"], evolved: true, power: 2.535 };
    const tip = skillInfo(f, 1);
    expect(tip.title).toBe(SPELL_INFO.palm.name);
    expect(tip.type).toBe("Epic · Evolved");
    expect(tip.tone).toBe("epic");
    expect(tip.props).toEqual([
      ["Cost", `${SPELLS.palm.cost} Qi`],
      ["Cooldown", `${(SPELLS.palm.cooldown * TRAITS.swift.cooldown).toFixed(1)} s`],
      ["Power", "254%"],
    ]);
    expect(tip.text).toBe(SPELL_INFO.palm.does);
    expect(tip.mods).toEqual([TRAIT_INFO.heavy, TRAIT_INFO.swift]);
  });

  test("a free skill says so; an empty slot says when it draws; the ultimate shows the rage it holds", () => {
    const f = fresh().fighters[1];
    f.bar[0] = { spell: "slash", rarity: "common", traits: [], evolved: false, power: 1 };
    expect(SPELLS.slash.cost).toBe(0);
    expect(skillInfo(f, 0).props[0]).toEqual(["Cost", "Free"]);
    expect(skillInfo(f, 0).type).toBe("Common");
    f.bar[2] = { empty: 1.4, full: 2 };
    expect(skillInfo(f, 2)).toEqual({ title: "Empty Slot", type: null, tone: null, props: [], text: "Draws a new skill in 1.4 s.", mods: [] });
    f.rage = 2;
    const ult = skillInfo(f, 3);
    expect(ult.title).toBe(SPELL_INFO[SCHOOLS.frost.ultimate].name);
    expect(ult.type).toBe("Ultimate");
    expect(ult.tone).toBe("ultimate");
    expect(ult.props).toContainEqual(["Rage", "2/4"]);
  });

  test("every buff the HUD can show has a tooltip, with its stacks", () => {
    for (const id of ["fury", "unyielding", "attune", "ward", "barrier", "grit", "burn", "chill", "shock", "frozen", "defDown", "silence", "sealed"]) {
      expect([id, typeof BUFF_INFO[id]?.name, typeof BUFF_INFO[id]?.does]).toEqual([id, "string", "string"]);
    }
    const f = fresh().fighters[0];
    f.fury = 3;
    const fury = buffsOf(f).find((b) => b.id === "fury");
    const tip = buffInfo(fury);
    expect(tip).toEqual({ title: BUFF_INFO.fury.name, type: "Buff", tone: "good", props: [["Stacks", "3"]], text: BUFF_INFO.fury.does, mods: [] });
    f.chill = 2;
    const chill = buffInfo(buffsOf(f).find((b) => b.id === "chill"));
    expect([chill.type, chill.tone]).toEqual(["Debuff", "bad"]);
  });

  test("the tiers' names match the rarities", () => {
    expect(Object.keys(RARITY)).toEqual(["common", "rare", "epic", "legendary"]);
  });
});
