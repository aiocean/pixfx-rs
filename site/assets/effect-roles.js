// The part each effect plays in a fight on its own page: two fighters stage it (bout.js) the way the hero's duel
// would use it. A role says who casts, where the effect appears and how the one it reaches reacts; the tuning next
// to it holds the numbers that effect needs (how fast a bolt flies, when a blow lands if the effect sends no cue).
//
// Units are game pixels and seconds, heights up from the ground, as in duel.js.

/** Every role a take can stage. */
export const ROLES = [
  // the caster summons it on the foe, which is thrown when it lands
  "strike",
  // it leaves the caster's palm and flies at the foe
  "projectile",
  // it pours from the caster's palm onto the foe for a while
  "stream",
  // a shell over the caster that the foe's blows break against
  "ward",
  // it lights the caster up, who then lands a stronger blow
  "buff",
  // it lies under the caster while it gathers a blow
  "ground-self",
  // it lies under the foe and hurts it while it stands there
  "ground-foe",
  // two blades meet in it
  "clash",
  // it carries the caster up, who dives onto the foe
  "flight",
  // it plays beside the two while they spar
  "ambient",
  // the caster vanishes in it and comes out behind the foe
  "blink",
  // the caster strikes the foe down and it destroys the foe's own body: the foe is gone
  "death",
];

/**
 * The role and tuning of each effect the engine has today. `impact`: seconds after the effect starts when its blow
 * lands if no cue says so (the effect's own cue time); `at`: where it appears; `power`: how hard the blow is (duel.js
 * spell damage); `life`: how long a lasting effect stays.
 */
const TABLE = {
  // plasma-explosion's cue is at 0 (explosions/plasma_explosion.rs cues): the thunderclap lands as it appears
  "explosions/plasma-explosion": { role: "strike", at: "chest", impact: 0, power: 22, spell: "seal" },
  "explosions/small-explosion": { role: "strike", at: "ground", impact: 0, power: 20, spell: "seal" },
  "explosions/dirt-explosion": { role: "strike", at: "ground", impact: 0, power: 18, spell: "seal" },
  // implosion sucks in and pops at 1.02 s (explosions/implosion.rs STILL); it drags the foe in while it sucks
  "explosions/implosion": { role: "strike", at: "chest", impact: 1.02, power: 25, spell: "collapse", pull: 40, pullUntil: 0.8 },
  // the icicle drops at 0.944 s (duel.js SPELLS.icicle) and freezes where it strikes
  "ice/icicle-fall": { role: "strike", at: "ground", impact: 0.944, power: 28, spell: "icicle", freeze: 0.9 },
  // a lotus of feathers bursts from the palm at arm's length (duel.js SPELLS.lotus), its cue at 0
  "holy/feather-burst": { role: "strike", at: "palm", impact: 0, power: 18, spell: "lotus", distance: 34 },
  "projectiles/laser-bolt": { role: "projectile", speed: 230, power: 13, spell: "bolt", directional: true },
  // the ball wanders round an origin that drifts at the foe (duel.js SPELLS.orb)
  "lightning/ball-lightning": { role: "projectile", orb: true, speed: 30, power: 14, spell: "orb", life: 5.5, distance: 72, mark: "burn" },
  "water/water-jet": { role: "stream", power: 4, every: 0.45, life: 1.8, spell: "bolt", directional: true },
  // hex-shield's four bolts strike at these times (arcane/hex_shield.rs HITS), from the side its canvas extends to
  "arcane/hex-shield": { role: "ward", blows: [1.05, 1.38, 1.62, 2.05], spell: "ward", directional: true },
  // power-up bursts 0.32 s in (feedback/power_up.rs BURST)
  "feedback/power-up": { role: "buff", impact: 0.32, power: 16, spell: "ward" },
  // the ring marks the caster as it gathers, and stays on it through the blow
  "feedback/outline": { role: "buff", impact: 0.3, power: 16, spell: "ward" },
  "arcane/rune-circle": { role: "ground-self", power: 18, spell: "void", distance: 40 },
  "nature/poison-cloud": { role: "ground-foe", life: 2.4, power: 8, spell: "frostfire", mark: "defDown" },
  // frostfire: a pale fire lit under the foe that burns and slows it (duel.js SPELLS.frostfire)
  "fire/spirit-flame": { role: "ground-foe", life: 2.4, power: 8, spell: "frostfire", chill: true, mark: "chilled" },
  // the vortex hangs over its origin and drags the foe in (duel.js SPELLS.void)
  "shadow/void-orb": { role: "ground-foe", life: 3.2, power: 14, spell: "void", pull: 34, mark: "silence" },
  // the sparks fly at `height` over the effect's origin (impacts/parry.rs cue)
  "impacts/parry": { role: "clash", spell: "slash" },
  "scifi/jetpack": { role: "flight", power: 11, spell: "dive", life: 1.4 },
  "spawn/poof": { role: "blink", power: 15, spell: "step" },
  "fire/campfire": { role: "ambient", at: "side" },
  "fire/candle": { role: "ambient", at: "side" },
  "fire/brazier": { role: "ambient", at: "side" },
  "critters/wisp": { role: "ambient", at: "above" },
  "pickups/coin-spin": { role: "ambient", at: "above" },
  "weather/fireflies": { role: "ambient", at: "screen" },
  "weather/snow": { role: "ambient", at: "screen" },
  "transitions/burn-away": { role: "ambient", at: "wipe" },
  "transitions/crt-off": { role: "ambient", at: "wipe" },
};

/** For an effect the table does not know yet: the first of its tags that names a role. */
const BY_TAG = [
  ["projectile", { role: "projectile", speed: 200, power: 12, spell: "bolt" }],
  ["stream", { role: "stream", power: 4, every: 0.45, life: 1.8, spell: "bolt" }],
  ["shield", { role: "ward", blows: [0.6], spell: "ward" }],
  ["barrier", { role: "ward", blows: [0.6], spell: "ward" }],
  ["buff", { role: "buff", impact: 0.3, power: 16, spell: "ward" }],
  ["clash", { role: "clash", spell: "slash" }],
  ["parry", { role: "clash", spell: "slash" }],
  ["flight", { role: "flight", power: 11, spell: "dive", life: 1.4 }],
  ["poison", { role: "ground-foe", life: 2.4, power: 8, spell: "frostfire" }],
  ["area", { role: "ground-foe", life: 2.4, power: 8, spell: "frostfire" }],
  ["teleport", { role: "blink", power: 15, spell: "step" }],
  ["explosion", { role: "strike", at: "ground", impact: 0, power: 20, spell: "seal" }],
  ["impact", { role: "strike", at: "ground", impact: 0, power: 18, spell: "seal" }],
  ["weather", { role: "ambient", at: "screen" }],
  ["transition", { role: "ambient", at: "wipe" }],
  ["prop", { role: "ambient", at: "side" }],
  ["critter", { role: "ambient", at: "above" }],
  ["pickup", { role: "ambient", at: "above" }],
  ["ambient", { role: "ambient", at: "side" }],
];

const DEFAULT = { role: "strike", at: "ground", impact: 0, power: 18, spell: "seal" };
/** For an effect that destroys the character it wraps (`meta.replacesActor`, as explosions/body-burst): a death. */
const DEATH = { role: "death", power: 30, spell: "seal" };

/**
 * The role effect `id` plays in a take and its tuning: `{ role, ...tuning }`. Known effects come from the table,
 * others by their tags (`meta.tags`), anything else is a strike. With the effect's `meta`, the tuning also carries
 * whether it loops (`loop`, so the take stops it) and the default of each option the staging reads (`height` of a
 * parry's sparks, `length` of a jet).
 */
export function roleFor(id, meta) {
  const tags = meta?.tags ?? [];
  const known = TABLE[id] ?? (meta?.replacesActor ? DEATH : undefined) ?? BY_TAG.find(([tag]) => tags.includes(tag))?.[1] ?? DEFAULT;
  const option = (name) => meta?.options?.find((o) => o.name === name)?.default;
  return {
    ...known,
    directional: known.directional ?? tags.includes("directional"),
    loop: meta?.loop ?? false,
    duration: meta?.duration ?? 2,
    height: option("height"),
    length: option("length"),
  };
}
