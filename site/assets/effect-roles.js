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
  // a trail on each fighter, following the way it glides
  "projectiles/trail": { role: "trail" },
  // power-up bursts 0.32 s in (feedback/power_up.rs BURST)
  "feedback/power-up": { role: "buff", impact: 0.32, power: 16, spell: "ward" },
  // the ring marks the caster as it gathers, and stays on it through the blow
  "feedback/outline": { role: "buff", impact: 0.3, power: 16, spell: "ward" },
  // teleport arrival flash at frame 6 (6 / 16 s = 0.375 s)
  "appear/teleport-beam": { role: "blink", impact: 0.375, power: 15, spell: "step" },
  // blink-out peak flare at frame 4 (4 / 15 s = 0.267 s)
  "disappear/blink-out": { role: "blink", impact: 0.267, power: 15, spell: "step" },
  // fade soft exit over 0.45 s
  "disappear/fade": { role: "blink", impact: 0.225, power: 15, spell: "step" },
  // incinerate peak flare at frame 4 (4 / 15 s = 0.267 s)
  "disappear/incinerate": { role: "strike", at: "ground", impact: 0.267, power: 22, spell: "seal" },
  // electrified snap arc at frame 8 (8 / 16 s = 0.5 s)
  "feedback/electrified": { role: "buff", impact: 0.5, power: 16, spell: "ward" },
  // soaked big drop crown splash at frame 9 (9 / 16 s = 0.5625 s)
  "feedback/soaked": { role: "buff", impact: 0.5625, power: 14, spell: "ward" },
  // encase stone collars crack at frame 6 (6 / 16 s = 0.375 s)
  "feedback/encase": { role: "strike", at: "ground", impact: 0.375, power: 18, spell: "seal" },
  // cloak-shimmer falter beat at frame 15 (15 / 15 s = 1.0 s)
  "feedback/cloak-shimmer": { role: "buff", impact: 1.0, power: 14, spell: "ward" },
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
