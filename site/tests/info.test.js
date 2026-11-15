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

  test("a bar skill's tooltip names it, its tier, its traits, its power, cost and cooldown", () => {
    const f = fresh().fighters[0];
    f.bar[1] = { spell: "palm", rarity: "epic", traits: ["heavy", "swift"], evolved: true, power: 2.535 };
    const tip = skillInfo(f, 1);
    expect(tip.title).toBe(SPELL_INFO.palm.name);
    expect(tip.tier).toBe("epic");
    const text = tip.lines.join("\n");
    expect(text).toContain("Epic");
    expect(text).toContain("Evolved");
    expect(text).toContain(TRAIT_INFO.heavy);
    expect(text).toContain(TRAIT_INFO.swift);
    expect(text).toContain("Power 254%");
    expect(text).toContain(`${SPELLS.palm.cost} qi`);
    expect(text).toContain(`${(SPELLS.palm.cooldown * TRAITS.swift.cooldown).toFixed(1)} s`);
    expect(text).toContain(SPELL_INFO.palm.does);
  });

  test("an empty slot says when it draws, and the ultimate says how much rage it needs", () => {
    const f = fresh().fighters[1];
    f.bar[2] = { empty: 1.4, full: 2 };
    expect(skillInfo(f, 2).title).toBe("Empty slot");
    expect(skillInfo(f, 2).lines.join(" ")).toContain("1.4 s");
    f.rage = 2;
    const ult = skillInfo(f, 3);
    expect(ult.title).toBe(SPELL_INFO[SCHOOLS.frost.ultimate].name);
    expect(ult.lines.join(" ")).toContain("Rage 2/4");
  });

  test("every buff the HUD can show has a tooltip, with its stacks", () => {
    for (const id of ["fury", "unyielding", "attune", "ward", "barrier", "grit", "burn", "chill", "shock", "frozen", "defDown", "silence", "sealed"]) {
      expect([id, typeof BUFF_INFO[id]?.name, typeof BUFF_INFO[id]?.does]).toEqual([id, "string", "string"]);
    }
    const f = fresh().fighters[0];
    f.fury = 3;
    const fury = buffsOf(f).find((b) => b.id === "fury");
    const tip = buffInfo(fury);
    expect(tip.title).toBe(BUFF_INFO.fury.name);
    expect(tip.good).toBe(true);
    expect(tip.lines.join(" ")).toContain("3 stacks");
  });

  test("the tiers' names match the rarities", () => {
    expect(Object.keys(RARITY)).toEqual(["common", "rare", "epic", "legendary"]);
  });
});
