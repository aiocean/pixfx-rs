// What the duel's tooltips say: each skill's name and what it does, each trait and buff, and the tooltip for a slot
// of a fighter's skill bar or a buff under its bars (lines of plain text; the page lays them out).
import { RAGE, SPELLS, TRAITS, skillBar } from "./duel.js";

/** Each skill's name and what it does, in a line. */
export const SPELL_INFO = {
  slash: { name: "Sword Slash", does: "A cut at close range. Free, and always at hand." },
  dive: { name: "Falling Sword", does: "Drops onto the foe sword first; cracks the ground if it lands." },
  blink: { name: "Blink", does: "Vanishes and reappears a few steps away." },
  bolt: { name: "Thunder Bolt", does: "A bolt of lightning that flies at the foe and shocks it." },
  palm: { name: "Thunder Palm", does: "A thunderclap from the open palm at arm's length: shocks, steals rage, and hits harder per shock stack." },
  step: { name: "Lightning Step", does: "Dashes through the foe, cutting it on the way." },
  seal: { name: "Thunder Seal", does: "Marks the ground under the foe, then strikes it with lightning that shocks." },
  orb: { name: "Ball Lightning", does: "A slow orb that seeks the foe, burning and shocking it." },
  bell: { name: "Golden Bell", does: "A dome that answers the next close blow with a stunning counter." },
  heal: { name: "Healing Light", does: "Heals and washes off burn, silence, defence down, chill and shock." },
  spark: { name: "Spark", does: "A quick, cheap bolt that shocks." },
  burst: { name: "Thunder Burst", does: "A blast at close range that hits harder per shock stack on the foe." },
  quake: { name: "Quake", does: "The ground under the foe cracks and blows up. Only hits on the ground." },
  hex: { name: "Hex Shield", does: "A dome of hex cells: 1 to 3 charges, each stops a whole blow. Cleanses." },
  tribulation: { name: "Heavenly Tribulation", does: "Ultimate: three thunderclaps fall one after another where the foe will be." },
  lotus: { name: "Ice Lotus", does: "A lotus of ice shards at close range that breaks the foe's defence, harder per chill stack." },
  shatter: { name: "Shatter", does: "A seal under the foe that bursts into cold and freezes it, harder per chill stack." },
  icicle: { name: "Icicle Fall", does: "An icicle drops on the foe, freezing it and breaking its defence. Only hits on the ground." },
  collapse: { name: "Collapse", does: "Space folds in: drags the foe in, then pops, silencing it." },
  frostfire: { name: "Frostfire", does: "A pale fire under the foe that burns and chills whoever stands in it." },
  void: { name: "Void Orb", does: "A dark orb that drags the foe in and burns it while it stands under it." },
  mirror: { name: "Ice Mirror", does: "A dome that sends the next bolt or orb back at its caster." },
  ward: { name: "Feather Ward", does: "1 to 3 charges, each stops a whole blow. Cleanses." },
  shard: { name: "Ice Shard", does: "A quick, cheap shard that chills." },
  jet: { name: "Water Jet", does: "A jet of freezing water at the foe that chills." },
  flurry: { name: "Flurry", does: "A short snowstorm on the foe that chills it while it stays." },
  prison: { name: "Ice Prison", does: "Ultimate: drags the foe in from far off, freezes and silences it, and locks blinking." },
};

/** What each trait does, in a line. */
export const TRAIT_INFO = {
  swift: `Swift: cooldown ${Math.round(TRAITS.swift.cooldown * 100)}%.`,
  heavy: `Heavy: power ×${TRAITS.heavy.power}.`,
  piercing: "Piercing: a guard cannot stop it.",
  lingering: `Lingering: +${TRAITS.lingering.stacks} stack of chill or shock.`,
  cheap: `Cheap: costs ${Math.round(TRAITS.cheap.cost * 100)}% qi.`,
};

/** Each buff's name and what it does, in a line (see `buffsOf`). */
export const BUFF_INFO = {
  fury: { name: "Fury", does: "Each blow landed adds +4% attack for the rest of the round, up to 5 stacks." },
  unyielding: { name: "Unyielding", does: "Once a round, gets back up from a killing blow with 1 life." },
  attune: { name: "Attuned", does: "Every second spell cast gives 1 rage." },
  ward: { name: "Ward", does: "Each charge stops a whole blow." },
  barrier: { name: "Barrier", does: "A dome stands round it: the bell counters a close blow, the mirror sends a bolt back." },
  grit: { name: "Grit", does: "Hard controls (freeze, stun, shock's hold) last shorter; from 2, a freeze is only a chill." },
  burn: { name: "Burn", does: "Loses life every half second." },
  chill: { name: "Chill", does: "Each stack slows walking; the third freezes. Fades 2.5 s after the last." },
  shock: { name: "Shock", does: "Twitches now and then, losing life each time, faster with more stacks." },
  frozen: { name: "Frozen", does: "Cannot move or act." },
  defDown: { name: "Defence Down", does: "Takes more damage from every blow." },
  silence: { name: "Silenced", does: "Only the sword answers: no spells." },
  sealed: { name: "Sealed", does: "Locked in place: cannot blink out." },
};

const TIER_NAME = { common: "Common", rare: "Rare", epic: "Epic", legendary: "Legendary" };

/**
 * The tooltip for slot `i` of fighter f's skill bar (0 to 2 the bar skills, 3 the ultimate): `{ title, tier, lines }`,
 * `tier` a rarity or null.
 */
export function skillInfo(f, i) {
  const slot = skillBar(f)[i];
  if (!slot) return null;
  if (!slot.spell) {
    return { title: "Empty slot", tier: null, lines: [`Draws a new skill in ${Math.max(0, slot.left).toFixed(1)} s.`] };
  }
  const spell = SPELLS[slot.spell];
  const info = SPELL_INFO[slot.spell];
  const swift = slot.traits.includes("swift") ? TRAITS.swift.cooldown : 1;
  const cheap = slot.traits.includes("cheap") ? TRAITS.cheap.cost : 1;
  const lines = [];
  if (slot.ultimate) lines.push(`Ultimate · Rage ${Math.min(RAGE, Math.round(slot.charge * RAGE))}/${RAGE}`);
  else lines.push(`${TIER_NAME[slot.rarity]}${slot.evolved ? " · Evolved" : ""}`);
  lines.push(info.does);
  if (!slot.ultimate) lines.push(`Power ${Math.round(slot.power * 100)}%`);
  const cost = Math.round(spell.cost * cheap);
  lines.push(`${cost > 0 ? `${cost} qi` : "Free"} · cooldown ${(spell.cooldown * swift).toFixed(1)} s`);
  for (const t of slot.traits) lines.push(TRAIT_INFO[t]);
  return { title: info.name, tier: slot.ultimate ? null : slot.rarity, lines };
}

/** The tooltip for a buff from `buffsOf`: `{ title, good, lines }`. */
export function buffInfo(b) {
  const info = BUFF_INFO[b.id];
  const lines = [info.does];
  if (b.stacks > 0) lines.push(`${b.stacks} ${b.stacks === 1 ? "stack" : "stacks"}`);
  return { title: info.name, good: b.good, lines };
}
