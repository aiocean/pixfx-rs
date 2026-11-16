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
  // it lies behind each fighter as the two glide back and forth
  "trail",
  // it gathers at the caster and holds; the caster orders it at the foe, far off, and its shots strike there
  "volley",
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
  // the sky falls on the foe: the keystone with the eye lands on its head at 1.78 s (shadow/winter_ordeal.rs BLOW)
  "shadow/winter-ordeal": { role: "strike", at: "ground", impact: 1.78, power: 30, spell: "collapse" },
  // the icicle drops at 0.944 s (duel.js SPELLS.icicle) and freezes where it strikes
  "ice/icicle-fall": { role: "strike", at: "ground", impact: 0.944, power: 28, spell: "icicle", freeze: 0.9 },
  // a lotus of feathers bursts from the palm at arm's length (duel.js SPELLS.lotus), its cue at 0
  "holy/feather-burst": { role: "strike", at: "palm", impact: 0, power: 18, spell: "lotus", distance: 34 },
  // the swords gather behind the caster and fly at the foe's chest, bursting on the first body they meet (the scene
  // strikes them on the fighters the page declares); `blows` only stand in when no strike comes
  // the cut lands as the blade points through hitX, hitY (`hit_time` in arcane/sword_slash.rs)
  "arcane/sword-slash": { role: "strike", at: "ground", impact: 0.404, power: 20, spell: "slash", directional: true },
  "arcane/sword-formation": { role: "volley", hold: 1.2, aim: "converge", blows: [0.55, 0.9, 1.25], power: 10, spell: "seal", directional: true },
  "projectiles/laser-bolt": { role: "projectile", speed: 230, power: 13, spell: "bolt", directional: true },
  // a flying sword alone, as a volley flies it: 300 pixels a second, a trail under it from behind its grip
  // a trail on each fighter, following the way it glides
  "projectiles/trail": { role: "trail" },
  "arcane/flying-sword": { role: "projectile", speed: 300, power: 12, spell: "bolt", directional: true, trail: { id: "projectiles/trail", opts: { secs: 0.28, width: 5, skip: 6 } } },
  // the ball wanders round an origin that drifts at the foe (duel.js SPELLS.orb)
  "lightning/ball-lightning": { role: "projectile", orb: true, speed: 30, power: 14, spell: "orb", life: 5.5, distance: 72, mark: "burn" },
  "water/water-jet": { role: "stream", power: 4, every: 0.45, life: 1.8, spell: "bolt", directional: true },
  // hex-shield's four bolts strike at these times (arcane/hex_shield.rs HITS), from the side its canvas extends to
  "arcane/hex-shield": { role: "ward", blows: [1.05, 1.38, 1.62, 2.05], spell: "ward", directional: true },
  // the xianxia shields take one blow on their right side, mirrored to face the foe: golden-bell at 1.4 s
  // (holy/golden_bell.rs HIT), lotus at 1.6 s (holy/lotus.rs HIT), restriction at 1.8 s (arcane/restriction.rs HIT,
  // where `hitX` and `hitY` default)
  "holy/golden-bell": { role: "ward", blows: [1.4], spell: "ward", directional: true },
  "holy/lotus": { role: "ward", blows: [1.6], spell: "ward", directional: true },
  "arcane/restriction": { role: "ward", blows: [1.8], spell: "ward", directional: true },
  // the bagua mirror's shield form throws the blow back as a beam at 1.6 s (arcane/bagua_mirror.rs HIT)
  "arcane/bagua-mirror": { role: "ward", blows: [1.6], spell: "ward", directional: true },
  // the blood shield takes its blow at 1.6 s (shadow/blood_shield.rs HIT, where `hitX` and `hitY` default)
  "shadow/blood-shield": { role: "ward", blows: [1.6], spell: "ward", directional: true },
  // the strike form holds behind the caster, then is thrown and bursts on the foe's chest (one shot)
  "arcane/bagua-mirror-strike": { role: "volley", hold: 1.2, aim: "converge", blows: [0.6], power: 14, spell: "seal", directional: true },
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
  "scifi/jetpack": { role: "flight", power: 11, spell: "dive", life: 1.4, carried: true },
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
  ["trail", { role: "trail" }],
  // a projectile is drawn pointing along +x (what a volley flies, turned to its heading), so it faces the way it is
  // thrown: flipped when the caster faces left
  ["projectile", { role: "projectile", speed: 200, power: 12, spell: "bolt", directional: true }],
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
