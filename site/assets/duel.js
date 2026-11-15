// The hero's duel: two cultivators fight on the ground and in the air, with spells that are pixfx effects. This
// module is the game only: rules, spells, and each fighter's mind. Each fighter's body (how it moves, falls, gets up
// and reels from a blow) is pixfx-fighter's `Body`, which `createDuel` is given; the module draws nothing.
// `step` returns what the engine should show (play, move or stop an effect) and what just happened that the screen
// should feel (`hit`, `clash`, `land`, `bump`, `ko`, `ultimate`, each with where and how hard); landing.js plays the effects and
// gives the impacts their weight (hit-stop, shake, numbers).
//
// Units are game pixels and seconds. x grows to the right; heights (`air`, effect y) are measured up from the ground.

/** Effects the arena itself shows when they are in the registry. */
export const DECOR = { lamp: "fire/brazier", night: "weather/fireflies" };

// A fighter's sizes and speeds (height, chest, half width, walk, jump, duck) are its body's: `f.spec`, as
// pixfx-fighter's `Body` gives them. The rules and the minds reason with them.

/** Flying: the highest a fighter goes, and the qi it costs a second. */
const FLY = { top: 96, cost: 3 };
/** Rage is a small whole-number pool: a point for each blow landed; when full, the school's ultimate. */
export const RAGE = 4;
/** Status effects: frozen (no moving, no acting) and shocked (a short paralysis), in seconds. */
const FREEZE = 0.9;
/**
 * Statuses that stack (after Halls of Torment). Chill: each chilling hit adds a stack (at most `max`), all of them
 * fading `time` seconds after the last; each slows walking by `slow`, and the last freezes and clears them. Shock:
 * each shocking hit adds a stack (at most `max`); a shocked fighter twitches every `every × fast^(stacks − 1)`
 * seconds, spending a stack, losing `share` of the caster's attack (no defence) and held for `hold` seconds.
 */
const CHILL = { max: 3, time: 2.5, slow: 0.85 };
const SHOCK = { max: 5, every: 0.6, fast: 0.854, share: 0.03, hold: 0.12 };
/** Skills that spend the foe's stacks of a status to hit harder: this much more power per stack. */
/** While the foe carries a status, the share of draws that come from the skills that spend it. */
const PAY_DRAW = 1 / 3;
const PAYOFF = { palm: { status: "shock", per: 0.12 }, burst: { status: "shock", per: 0.12 }, shatter: { status: "chill", per: 0.15 }, lotus: { status: "chill", per: 0.15 } };

/**
 * Grit (after Vampire Survivors' freeze resistance and Valhalla's stagger cap): each hard control a fighter takes
 * (a freeze, the shock's hold, the bell's stun) adds `per`; grit falls `fall` a second. A control lasts
 * `seconds / (1 + grit)`, and from grit `thaw` a freeze is only a chill stack.
 */
const GRIT = { per: 1, fall: 0.4, thaw: 2 };

/** f takes a hard control of `kind` for `seconds`, cut by its grit; returns how long it holds. */
export function control(duel, f, kind, seconds) {
  if (f.hp <= 0) return 0;
  if (kind === "freeze" && f.grit >= GRIT.thaw) {
    addChill(duel, f, 1);
    return 0;
  }
  const time = seconds / (1 + f.grit);
  f.grit += GRIT.per;
  if (kind === "freeze") f.frozen = Math.max(f.frozen, time);
  else if (kind === "hold") f.shocked = Math.max(f.shocked, time);
  else {
    f.stun = Math.max(f.stun, time);
    f.shocked = Math.max(f.shocked, time);
  }
  return time;
}

/** Seconds between shock ticks with `stacks` stacks. */
export function shockInterval(stacks) {
  return SHOCK.every * SHOCK.fast ** (stacks - 1);
}

/** f takes `n` chill stacks; the last one it can hold freezes it. */
export function addChill(duel, f, n) {
  if (f.hp <= 0) return;
  f.chill = Math.min(CHILL.max, f.chill + n);
  f.chillT = CHILL.time;
  if (f.chill >= CHILL.max) {
    f.chill = 0;
    f.chillT = 0;
    control(duel, f, "freeze", FREEZE);
  }
}

/** f takes `n` shock stacks from `by`. */
export function addShock(duel, f, by, n) {
  if (f.hp <= 0) return;
  if (!f.shock) f.shock = { stacks: 0, tick: 0, by: by.index };
  f.shock.stacks = Math.min(SHOCK.max, f.shock.stacks + n);
  if (f.shock.tick <= 0) f.shock.tick = shockInterval(f.shock.stacks);
  f.shock.by = by.index;
}
/**
 * How a blow turns into damage, the way xianxia hero battlers count it: attack times the skill's percentage, cut by
 * the defence that matches its type (physical or magic), then a crit roll, a little spread, and a dodge roll before
 * it all. Spell `damage` is the skill's power: power 10 is 100% of attack. Life (`hp`) is kept as a share of the
 * fighter's full life, 0 to 100; hits report the real amounts.
 */
const DEFENCE = 500;
const SPREAD = 0.03;
/** Fury: each blow a fury fighter lands raises its attack, up to a cap of stacks, for the rest of the round. */
const FURY = { per: 0.04, max: 5 };
/** Defence broken by a spell: this much defence lost, for the spell's `defDown` seconds. */
const DEF_DOWN = 0.3;
/** Burn: a share of the caster's attack every tick, ignoring defence. */
const BURN = { time: 3, every: 0.5, share: 0.05 };
/** A ward (after Valhalla's hit shields) holds 1 to `charges` charges for `time` seconds; each erases one whole blow. */
const WARD = { charges: 3, time: 6 };
/**
 * The rotating skill bar: `slots` skills at a time, each drawn from the school's pool with its own casts (1 to 4,
 * equal odds). A skill drawn with fewer casts hits harder (CAST_POWER, by casts). A spent skill empties its slot,
 * which counts down `wait + spread × roll` seconds and then draws a skill not on the bar.
 */
const BAR = { slots: 3, casts: 4, wait: 3, spread: 2 };
const CAST_POWER = [1.4, 1.2, 1.1, 1.0];
/**
 * The pairs that evolve, per school: a spent skill whose partner is on the bar refills at once as its evolved form,
 * `EVOLVED` times as strong and inflicting one more stack. An evolved skill, once spent, empties its slot.
 */
const EVOLVES = {
  thunder: [["bolt", "seal"], ["palm", "step"], ["quake", "burst"]],
  frost: [["bolt", "icicle"], ["shatter", "flurry"], ["lotus", "jet"]],
};
const EVOLVED = 1.5;

/** How hard a skill drawn with `casts` casts hits, as a multiple of its power. */
export function castsPower(casts) {
  return CAST_POWER[casts - 1] ?? 1;
}

/**
 * Each school's fighter: full life, attack, the two defences, crit chance and crit damage, dodge, and its passives.
 * Thunder is a body and sword cultivator: tough, hits harder the longer it fights (fury), and once a round stands
 * up from a killing blow with 1 life (unyielding). Frost is a spell cultivator: frailer, harder to pin down, crits
 * harder, shields itself, and every second spell it casts charges its rage (attuned).
 */
export const STATS = {
  thunder: { hp: 2500, atk: 345, pdef: 130, mdef: 110, crit: 0.18, critDamage: 1.6, dodge: 0.05, fury: true, unyielding: true, attuned: false },
  frost: { hp: 2450, atk: 370, pdef: 105, mdef: 165, crit: 0.15, critDamage: 1.7, dodge: 0.11, fury: false, unyielding: false, attuned: true },
};

const QI_REGEN = 3;
const MEDITATE_REGEN = 16;
/**
 * Each fighter thinks with pixfx-mind, a brain of lasting postures (pressing, zoning, punishing, defending...).
 * Its temperament, after Sirlin's Playing to Win: attacking at the first chance is predictable, so after it commits
 * to an attack it takes a breath in neutral (`tempo` in its school, seconds, spread by `spread`) and attacks again
 * sooner only to punish an opening. An attack is worth what it is likely to land, less what it costs if it misses:
 * the hardest blow the foe, free to answer, can land on it before it recovers, weighed at `risk`, never below
 * `floor` of its worth. Ahead on life it weighs that cost up to `lead` times more. What it has seen an attack do
 * counts as `prior` tries at first. It sees what the foe starts `reaction` seconds late and thinks every `think`.
 */
const TEMPERAMENT = { spread: 0.8, risk: 0.6, floor: 0.3, lead: 0.8, prior: 6, reaction: 0.16, think: 0.12 };
/** Qi a block costs. */
const GUARD_COST = 8;
/** The share of a blocked blow's damage that still lands: damage taken is never cut by more than 80%. */
const CHIP = 0.2;
/**
 * Guard by the size of the blow (after Halls of Torment): a guarded blow of `power` is turned aside with chance
 * `min(0.5r, 0.5√r)`, at most 1, where `r = GUARD / power`: a blade's cut always, a heavy blast about 2 in 5.
 */
const GUARD = 20;

/** The chance a guard turns aside a blow of `power`. */
export function blockChance(power) {
  if (!(power > 0)) return 1;
  const r = GUARD / power;
  return Math.min(1, 0.5 * r, 0.5 * Math.sqrt(r));
}
const KNOCKBACK = 130;
const ROUND_PAUSE = 3.2;

const CAST_CIRCLE = "arcane/rune-circle";
/** How long a barrier flares after it turns a blow aside, in seconds. */
const RING = 0.22;
/** How urgent a threat must be before a fighter spends its barrier on it. */
const BARRIER_THREAT = 10;
const CLASH = "impacts/parry";

/**
 * Each spell: what it costs, how long the fighter is committed before (`windup`) and after (`recover`) it lands,
 * its cooldown, and its own numbers. `fx` is the effect that shows it; a spell whose effect is not in the registry
 * is left out of the fight (slash and block need none).
 */
export const SPELLS = {
  slash: { kind: "melee", type: "physical", cost: 0, windup: 0.2, recover: 0.32, cooldown: 0.7, range: 24, damage: 5 },
  // Thunder: a flash of lightning that carries the body through the foe and out the other side
  step: { kind: "dash", type: "physical", fx: "spawn/poof", cost: 24, windup: 0.26, recover: 0.32, cooldown: 3.4, distance: 72, damage: 15 },
  // Thunder: a gold seal opens on the ground under the foe, and a thunderclap bursts from it
  seal: {
    kind: "zone", type: "magic", fx: "explosions/plasma-explosion", cost: 30, windup: 0.3, recover: 0.3, cooldown: 2.4,
    sigil: 0.4, impact: 0.08, radius: 16, damage: 22, notice: 0, shock: true,
  },
  // Frost: a lotus of ice shards blooms from the palm at close range, and cracks the foe's guard
  lotus: { kind: "blast", type: "magic", fx: "holy/feather-burst", cost: 24, windup: 0.3, recover: 0.3, cooldown: 2.2, range: 40, damage: 18, defDown: 3 },
  // Frost: a pale fire that does not warm, lit under the foe; it burns whoever stands in it
  frostfire: {
    kind: "void", type: "magic", fx: "fire/spirit-flame", cost: 33, windup: 0.4, recover: 0.3, cooldown: 5,
    life: 2.6, height: 0, pull: 0, reach: 18, under: 18, tick: 5, damage: 8, chill: true,
  },
  // Frost: a blue seal under the foe that shatters into a burst of cold, freezing it
  shatter: {
    kind: "zone", type: "magic", fx: "explosions/plasma-explosion", cost: 33, windup: 0.35, recover: 0.3, cooldown: 3,
    sigil: 0.35, impact: 0.08, radius: 16, damage: 25, notice: 0, freeze: 0.6,
  },
  // from above: drop onto the foe sword first, and crack the ground if it lands there
  dive: { kind: "melee", type: "physical", dive: true, cost: 12, windup: 0.22, recover: 0.4, cooldown: 1.6, range: 26, damage: 11, speed: 260, slamFx: "explosions/dirt-explosion" },
  bolt: { kind: "bolt", type: "magic", fx: "projectiles/laser-bolt", cost: 18, windup: 0.28, recover: 0.22, cooldown: 1.3, speed: 230, damage: 13, shock: true },
  orb: {
    kind: "orb", type: "magic", fx: "lightning/ball-lightning", hitFx: "explosions/plasma-explosion", burn: true,
    cost: 45, windup: 0.6, recover: 0.3, cooldown: 4.5, speed: 26, life: 5.5, damage: 14, radius: 10,
  },
  // icicle-fall drops its icicle at FREE + fall time: 0.64 + sqrt(2 * (80 - 36) / 950) s, and only starts to tremble,
  // which is when a fighter notices it overhead, at 0.3 s (pixelfx ice/icicle-fall.js TREMBLE)
  icicle: { kind: "zone", type: "magic", defDown: 4, fx: "ice/icicle-fall", cost: 33, windup: 0.42, recover: 0.28, cooldown: 2.2, impact: 0.944, radius: 16, damage: 28, notice: 0.3, grounded: true, freeze: FREEZE },
  // implosion sucks in until 0.8 s and pops at 1.02 s (pixelfx explosions/implosion.js SUCK, STILL)
  collapse: {
    kind: "zone", type: "magic", silence: 1.5, fx: "explosions/implosion", cost: 39, windup: 0.5, recover: 0.3, cooldown: 2.4,
    impact: 1.02, radius: 18, damage: 25, notice: 0.15, pull: 60, reach: 52, pullUntil: 0.8,
  },
  // a thunderclap from the open palm, at arm's length
  palm: { kind: "blast", type: "physical", stealRage: 1, fx: "explosions/plasma-explosion", cost: 27, windup: 0.32, recover: 0.35, cooldown: 2.8, range: 38, damage: 20, shock: true },
  // ultimates, cast with full rage. Thunder: three thunderclaps fall one after another where the foe will be.
  tribulation: {
    kind: "zone", type: "magic", fx: "explosions/plasma-explosion", ultimate: true, cost: 0, windup: 0.7, recover: 0.5, cooldown: 0,
    impact: 0.08, radius: 12, damage: 14, notice: 0, strikes: 3, every: 0.7, aimAhead: 0.5,
  },
  // Frost: a great collapse that drags the foe in from far off and freezes it where it pops.
  prison: {
    kind: "zone", type: "magic", silence: 2, fx: "explosions/implosion", ultimate: true, cost: 0, windup: 0.7, recover: 0.5, cooldown: 0,
    impact: 1.02, radius: 26, damage: 31, notice: 0.1, pull: 70, reach: 110, pullUntil: 0.9, seals: true, freeze: FREEZE * 1.4,
  },
  // void-orb hangs 62 px over its origin (pixelfx shadow/void-orb.js Y) and loops every 3.2 s
  void: {
    kind: "void", type: "magic", fx: "shadow/void-orb", endFx: "explosions/implosion",
    cost: 60, windup: 0.9, recover: 0.45, cooldown: 11, life: 3.2, height: 62, pull: 34, reach: 90, under: 22, tick: 4, damage: 14,
  },
  heal: { kind: "heal", fx: "holy/feather-burst", cost: 35, windup: 0.85, recover: 0.3, cooldown: 9, amount: 20 },
  // a shell of frost qi that takes blows for the caster until it breaks
  ward: { kind: "ward", fx: "holy/feather-burst", cost: 45, windup: 0.5, recover: 0.25, cooldown: 8 },
  // the defences: a dome of qi for a moment that turns every blow aside. Thunder's golden bell rings back at a
  // blade or palm that strikes it, stunning and throwing off the striker; Frost's ice mirror sends bolts back
  bell: { kind: "barrier", fx: "arcane/rune-circle", cost: 18, windup: 0.06, recover: 0.1, cooldown: 6.5, time: 2, counter: { damage: 9, stun: 0.6 } },
  mirror: { kind: "barrier", fx: "arcane/rune-circle", cost: 18, windup: 0.06, recover: 0.1, cooldown: 6.5, time: 2.2, reflect: true },
  blink: { kind: "blink", fx: "spawn/poof", cost: 20, windup: 0.06, recover: 0.12, cooldown: 1.8, distance: 56 },
  // the bar's newer spells. Thunder: a quick spark off the fingertips, light but fast
  spark: { kind: "bolt", type: "magic", fx: "projectiles/laser-bolt", cost: 10, windup: 0.18, recover: 0.18, cooldown: 0.9, speed: 300, damage: 8, shock: true },
  // Thunder: a palm that bursts with everything the foe's shocks have gathered
  burst: { kind: "blast", type: "physical", fx: "explosions/plasma-explosion", cost: 30, windup: 0.36, recover: 0.4, cooldown: 3, range: 30, damage: 24 },
  // Thunder: the ground under the foe cracks and blows up; small-explosion flashes at once, so the seal's mark is
  // what warns of it
  quake: {
    kind: "zone", type: "physical", fx: "explosions/small-explosion", cost: 26, windup: 0.3, recover: 0.3, cooldown: 2.6,
    sigil: 0.3, impact: 0.05, radius: 14, damage: 18, notice: 0, grounded: true,
  },
  // Thunder: a dome of hex cells that takes blows for the caster
  hex: { kind: "ward", fx: "arcane/hex-shield", cost: 40, windup: 0.45, recover: 0.25, cooldown: 8 },
  // Frost: a sliver of ice, light and quick, that chills where it strikes
  shard: { kind: "bolt", type: "magic", fx: "projectiles/laser-bolt", cost: 10, windup: 0.18, recover: 0.18, cooldown: 0.9, speed: 300, damage: 7, chill: true },
  // Frost: a jet of freezing water from the palm, at middle reach
  jet: { kind: "blast", type: "magic", fx: "water/water-jet", cost: 22, windup: 0.3, recover: 0.3, cooldown: 2.4, range: 56, damage: 14, chill: true },
  // Frost: a short flurry of snow under the foe: two ticks that chill whoever stands in it, then its blow
  flurry: {
    kind: "void", type: "magic", fx: "weather/snow", cost: 30, windup: 0.4, recover: 0.3, cooldown: 5,
    life: 1.2, height: 0, pull: 0, reach: 22, under: 22, tick: 3, damage: 10, chill: true,
  },
};

/**
 * The two schools. A school's spells are its `basics` (the blade, the dive and the blink, always at hand), its
 * `pool`, from which its three bar slots draw at random, and its `ultimate`, charged by rage. Each spell runs on its
 * own cooldown. Then the distance each
 * school likes to fight at, how eager it is, how long it takes a breath after each attack (`tempo`, seconds), the
 * distance it keeps when its qi runs low (`dry`: Thunder presses in to sword's length, Frost stays at middle distance
 * while its qi gathers again), the height it likes to hover at while it has the qi (`sky`; Frost casts from the air,
 * Thunder fights on the ground), and its colours.
 */
export const SCHOOLS = {
  thunder: {
    name: "Thunder", basics: ["slash", "dive", "blink"], pool: ["bolt", "palm", "step", "seal", "orb", "bell", "heal", "spark", "burst", "quake", "hex"], ultimate: "tribulation", signature: "blast", postures: [1, 1, 1, 1, 1, 1, 1, 1], range: 64, dry: SPELLS.slash.range - 4, aggression: 1.15, tempo: 0.8, sky: 22,
    bolt: ["#2a1a08", "#6a4410", "#b07a1a", "#e8b030", "#ffd84a", "#ffe98a", "#fff6c4", "#ffffff"],
    circle: ["#2a1a08", "#6a4410", "#b07a1a", "#e8b030", "#ffd84a", "#fff6c4", "#ffffff"],
    // every effect a Thunder spell plays is drawn in gold and white, so it is plain whose it is
    tint: {
      "explosions/plasma-explosion": {
        palette: ["#1a0e04", "#4a2a08", "#9a5a10", "#e0a020", "#ffd84a", "#fff3b0", "#ffffff"],
        accent: ["#2a1408", "#7a3a10", "#e07a1a", "#ffc23d", "#fff0b0"],
      },
      "spawn/poof": { palette: ["#3a2a10", "#7a5a20", "#c09040", "#ffd84a", "#fff3b0", "#ffffff"], stars: ["#ffd84a", "#fff3b0", "#ffffff"] },
      "holy/feather-burst": { palette: ["#6a4410", "#b07a1a", "#e8b030", "#ffd84a", "#fff6c4", "#ffffff"] },
      "explosions/dirt-explosion": { fire: ["#4a2a08", "#9a5a10", "#e0a020", "#ffc23d", "#ffd84a", "#ffe98a", "#fff6c4", "#ffffff"] },
    },
  },
  frost: {
    name: "Frost", basics: ["slash", "dive", "blink"], pool: ["bolt", "lotus", "shatter", "icicle", "collapse", "frostfire", "void", "mirror", "ward", "shard", "jet", "flurry"], ultimate: "prison", signature: "zone", postures: [1, 1, 1, 1, 1, 1, 1, 1], range: 100, dry: 80, aggression: 1.1, tempo: 2, sky: 48,
    bolt: ["#151a3d", "#212f6b", "#2c58aa", "#3a8bd8", "#62c8f4", "#a6ecff", "#dcfbff", "#ffffff"],
    circle: ["#151a3d", "#212f6b", "#2c58aa", "#3a8bd8", "#62c8f4", "#dcfbff", "#ffffff"],
    // every effect a Frost spell plays is drawn in ice blue and white
    tint: {
      "explosions/plasma-explosion": {
        palette: ["#0a1430", "#16306a", "#2c58aa", "#4a9ae0", "#8fd8ff", "#dcfbff", "#ffffff"],
        accent: ["#0a1830", "#1a4a80", "#3a8bd8", "#a6ecff", "#ffffff"],
      },
      "explosions/implosion": { palette: ["#060a18", "#10224a", "#1e4a8a", "#3a8bd8", "#8fd8ff", "#dcfbff", "#ffffff"] },
      "holy/feather-burst": { palette: ["#1e3f80", "#2c58aa", "#62c8f4", "#a6ecff", "#dcfbff", "#ffffff"], glow: "#a6ecff" },
      "fire/spirit-flame": { palette: ["#0a1430", "#16306a", "#2c58aa", "#3a8bd8", "#62c8f4", "#a6ecff", "#dcfbff", "#ffffff"] },
      "spawn/poof": { palette: ["#16306a", "#2c58aa", "#62c8f4", "#a6ecff", "#dcfbff", "#ffffff"], stars: ["#a6ecff", "#dcfbff", "#ffffff"] },
      // a flurry falls on a patch round the foe, not the whole sky
      "weather/snow": { width: 48, height: 72 },
    },
  },
};
// a school's barrier rings in its circle's colours, and so does a blow it turns aside
for (const school of Object.values(SCHOOLS)) {
  school.tint["arcane/rune-circle"] = { palette: school.circle };
  school.tint["impacts/parry"] = { palette: school.circle.slice(2) };
}

/** ball-lightning's ball relative to its origin at time t (pixelfx lightning/ball-lightning.js `at`, P = 4). */
export function orbOffset(t) {
  const a = (t / 4) * Math.PI * 2;
  return [38 * Math.sin(a) + 9 * Math.sin(2 * a + 1), -42 + 13 * Math.sin(2 * a + 0.4) + 6 * Math.sin(3 * a + 0.5)];
}

/** A small seeded generator (mulberry32), so a seed replays the same fight. */
function random(seed) {
  let s = seed >>> 0;
  return () => {
    s = (s + 0x6d2b79f5) >>> 0;
    let t = s;
    t = Math.imul(t ^ (t >>> 15), t | 1);
    t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

/**
 * A new duel in the arena from `left` to `right` (game pixels), standing on `ground`. `has(id)` says whether an
 * effect is in the registry. `Body` is pixfx-fighter's body and `Mind` pixfx-mind's brain (both from the engine's wasm
 * package): each fighter gets one of each. `minds`, for tests, may seat another mind in a corner: `[make0, make1]`,
 * each `(duel, fighter) => { think(duel, me, foe, dt), tell(outcome), reset(), hold(seconds), explain(on),
 * posture }` or undefined for the brain.
 */
export function createDuel({ seed = 1, left, right, ground, has, sky = FLY.top, Body, Mind, minds = [] }) {
  // `sky`: how high above the ground a fighter may fly, below whatever hangs over the arena
  const top = Math.min(FLY.top, sky);
  const duel = { seed, rng: random(seed), left, right, ground, has, top, arena: { left, right, top }, Body, time: 0, round: 0, wins: [0, 0], key: 0, shots: [], pause: 0, out: [] };
  const usable = (name) => {
    const fx = SPELLS[name].fx;
    return !fx || has(fx);
  };
  duel.fighters = [fighter(0, "thunder", usable, new Body(0)), fighter(1, "frost", usable, new Body(0))];
  for (const f of duel.fighters) {
    const make = minds[f.index] ?? ((d, me) => brain(d, me, Mind));
    f.mind = make(duel, f);
  }
  for (const name of Object.values(DECOR)) if (has(name)) duel.decor = true;
  startRound(duel);
  return duel;
}

/** The body's fields, read and written on the fighter as if they were its own. */
const BODY_FIELDS = ["x", "air", "vx", "vy", "face", "stun", "wake", "land", "squat", "leap", "duck", "flash", "hitT", "hitPower", "frozen", "shocked", "poseT"];

/** Fighter `index` of `school`, with the school's spells that are `usable`. */
function fighter(index, school, usable, body) {
  const { basics, pool, ultimate } = SCHOOLS[school];
  // every spell it may cast this fight: the brain is told them all once; one off the bar is never ready
  const spells = [...basics, ...pool, ultimate].filter(usable);
  const f = { index, school, spells, pool: pool.filter(usable), bar: [], stats: STATS[school], hp: 100, qi: 100, act: null, guard: 0, medit: false, fly: false, hover: 0, rage: 0, sealed: 0, charges: 0, shieldT: 0, barrier: 0, barrierSpell: null, barrierKey: null, rang: 0, burn: null, defDown: 0, silence: 0, fury: 0, rose: false, dodged: 0, chill: 0, chillT: 0, shock: null, grit: 0, cool: {}, coolFull: {}, intent: { move: 0 }, aura: null, attune: 0, mind: null, body, spec: body.spec };
  for (const k of BODY_FIELDS) {
    Object.defineProperty(f, k, { enumerable: true, get: () => body[k], set: (v) => { body[k] = v; } });
  }
  for (const k of ["pose", "down"]) Object.defineProperty(f, k, { enumerable: true, get: () => body[k] });
  return f;
}

function startRound(duel) {
  const span = duel.right - duel.left;
  duel.round++;
  duel.pause = 0;
  duel.winner = null;
  for (const s of duel.shots) stopShot(duel, s);
  duel.shots = [];
  for (const f of duel.fighters) {
    Object.assign(f, { hp: 100, qi: 100, act: null, guard: 0, medit: false, fly: false, hover: 0, rage: 0, sealed: 0, charges: 0, shieldT: 0, barrier: 0, barrierSpell: null, barrierKey: null, rang: 0, burn: null, defDown: 0, silence: 0, fury: 0, rose: false, dodged: 0, chill: 0, chillT: 0, shock: null, grit: 0, cool: {}, coolFull: {}, intent: { move: 0 } });
    f.mind.reset();
    f.bar = [];
    for (let i = 0; i < BAR.slots; i++) draw(duel, f, i, i === 0);
    f.body.startRound(duel.left + span * (f.index === 0 ? 0.22 : 0.78));
    if (f.aura !== null) emit(duel, { type: "stop", key: f.aura });
    f.aura = null;
  }
}

/** Advance the duel by `dt` seconds; returns the effect commands for this step. */
export function step(duel, dt) {
  duel.out = [];
  // fixed sub-steps, so the fight does not depend on the frame rate
  let left = Math.min(dt, 0.25);
  while (left > 1e-9) {
    const h = Math.min(left, 1 / 60);
    tick(duel, h);
    left -= h;
  }
  return duel.out;
}

function tick(duel, dt) {
  duel.time += dt;
  if (duel.winner !== null) {
    duel.pause += dt;
    for (const f of duel.fighters) settle(duel, f, dt);
    if (duel.pause >= ROUND_PAUSE) startRound(duel);
    return;
  }
  const [a, b] = duel.fighters;
  for (const [me, foe] of [[a, b], [b, a]]) {
    const choice = me.mind.think(duel, me, foe, dt);
    if (choice) apply(duel, me, choice);
    body(duel, me, foe, dt);
  }
  if (a.hp > 0 && b.hp > 0) duel.Body.separate(a.body, b.body, duel.arena);
  for (const s of [...duel.shots]) shot(duel, s, dt);
  for (const f of duel.fighters) if (f.hp <= 0 && duel.winner === null) knockOut(duel, f);
}

// ---------- bodies: the rules' side of moving, casting, getting hit ----------

/** One step of fighter me: its body's clocks, the rules' own timers, then its body moves as the rules drive it. */
function body(duel, me, foe, dt) {
  me.body.tick(dt, me.hp > 0);
  me.sealed = Math.max(0, me.sealed - dt);
  me.defDown = Math.max(0, me.defDown - dt);
  me.silence = Math.max(0, me.silence - dt);
  me.dodged = Math.max(0, me.dodged - dt);
  me.grit = Math.max(0, me.grit - GRIT.fall * dt);
  me.chillT = Math.max(0, me.chillT - dt);
  if (me.chillT === 0) me.chill = 0;
  shocking(duel, me, dt);
  me.shieldT = Math.max(0, me.shieldT - dt);
  if (me.shieldT === 0) me.charges = 0;
  me.rang = Math.max(0, me.rang - dt);
  if (me.barrier > 0) {
    me.barrier = Math.max(0, me.barrier - dt);
    if (me.barrier === 0) dropBarrier(duel, me);
  }
  burning(duel, me, dt);
  me.guard = Math.max(0, me.guard - dt);
  for (const k of Object.keys(me.cool)) me.cool[k] = Math.max(0, me.cool[k] - dt);
  me.bar.forEach((slot, i) => {
    if (slot.empty === undefined) return;
    slot.empty = Math.max(0, slot.empty - dt);
    if (slot.empty === 0) draw(duel, me, i);
  });
  me.qi = Math.min(100, me.qi + (me.medit ? MEDITATE_REGEN : me.fly ? -FLY.cost : QI_REGEN) * dt);
  // out of qi, or knocked out of the sky: flight ends and it falls
  if (me.fly && (me.qi <= 1 || me.body.reeling())) me.fly = false;
  // a dive drops straight at the foe's height
  const diving = me.act?.phase === "windup" && SPELLS[me.act.spell].dive;
  const motion = diving ? { dive: SPELLS.dive.speed, floor: foe.air } : me.fly ? { fly: me.hover } : "gravity";
  const drive = { alive: me.hp > 0, acting: me.act !== null, meditating: me.medit, guarding: me.guard > 0, chilled: false, intent: me.intent.move * CHILL.slow ** me.chill, motion, foeX: foe.x };
  for (const e of me.body.step(dt, drive, duel.arena, duel.rng)) emit(duel, e);

  if (me.act) {
    me.act.t += dt;
    const spell = SPELLS[me.act.spell];
    if (me.act.phase === "windup" && me.act.t >= spell.windup) {
      release(duel, me, foe, spell);
      // a blow rung back by a golden bell breaks the striker's own action
      if (me.act) {
        me.act = { ...me.act, phase: "recover", t: 0 };
        if (me.act.circle !== undefined) emit(duel, { type: "stop", key: me.act.circle });
      }
    } else if (me.act.phase === "recover" && me.act.t >= spell.recover) {
      me.act = null;
    }
  }
  pose(me);
}

/** Settle fighter f's pose from what the rules have it doing; the body decides when it is off its feet or reeling. */
function pose(f) {
  const doing = f.act ? f.act.phase : f.guard > 0 ? "guard" : f.medit ? "meditate" : f.fly ? "fly" : f.intent.move !== 0 ? "walk" : "idle";
  f.body.updatePose(doing, f.act !== null && SPELLS[f.act.spell].kind === "melee", f.hp > 0);
}

function begin(duel, me, name, target) {
  const spell = SPELLS[name];
  me.qi -= spell.cost;
  if (spell.ultimate) {
    me.rage = 0;
    emit(duel, { type: "ultimate", by: me.index, x: me.x, y: me.air + me.spec.chest });
  }
  me.cool[name] = me.coolFull[name] = spell.cooldown + spell.windup;
  // a bar skill spends a cast, and strikes with the power it was drawn with even if that empties its slot
  const i = me.bar.findIndex((slot) => slot.spell === name);
  const power = i < 0 ? 1 : me.bar[i].power;
  // an evolved skill inflicts one more stack of what it inflicts
  const stacks = i >= 0 && me.bar[i].evolved ? 2 : 1;
  if (i >= 0) spend(duel, me, i);
  me.act = { spell: name, phase: "windup", t: 0, target, power, stacks };
  setMeditating(duel, me, false);
  // longer casts stand in a rune circle while they gather
  if (spell.kind !== "melee" && spell.kind !== "blink" && spell.kind !== "dash" && spell.kind !== "barrier" && duel.has(CAST_CIRCLE)) {
    me.act.circle = play(duel, CAST_CIRCLE, me.x, duel.ground, { palette: SCHOOLS[me.school].circle, seed: me.index + 1 });
  }
}

/** The moment a spell lands: it hits, flies, falls or heals. */
function release(duel, me, foe, spell) {
  const g = duel.ground;
  // the blow, with the power of the bar skill it was cast from
  const hit = (spell.damage ?? 0) * (me.act.power ?? 1);
  if (me.stats.attuned && spell.kind !== "melee" && !spell.ultimate && spell.cost > 0) {
    me.attune++;
    if (me.attune % 2 === 0) gainRage(me, 1);
  }
  if (spell.kind === "melee") {
    me.vx += me.face * 50;
    if (spell.dive) {
      me.fly = false;
      // a dive that ends on the ground cracks it
      if (me.air < 2 && spell.slamFx && duel.has(spell.slamFx)) {
        cast(duel, me, spell.slamFx, me.x + me.face * 6, g);
        emit(duel, { type: "land", x: me.x, strength: 1 });
      }
    }
    const dx = Math.abs(foe.x - me.x);
    if (dx <= spell.range + foe.spec.halfWidth && Math.abs(foe.air - me.air) < me.spec.chest + 4 && foe.hp > 0) {
      if (blocks(duel, foe, me.x, hit)) clash(duel, (me.x + foe.x) / 2, g - me.spec.chest - me.air, [me, foe]);
      else if (foe.act?.spell === "slash" && foe.act.phase === "windup" && foe.act.t > SPELLS.slash.windup * 0.6) {
        // both blades swing at once: they meet
        clash(duel, (me.x + foe.x) / 2, g - me.spec.chest - me.air, [me, foe]);
        foe.act = null;
      } else hurt(duel, foe, hit, me.face, me.act.spell, me.act.stacks);
    }
  } else if (spell.kind === "bolt") {
    const y = me.air + me.spec.chest;
    const key = play(duel, spell.fx, me.x + me.face * 8, g - y, { palette: SCHOOLS[me.school].bolt, flip: me.face < 0, seed: me.index + 7 });
    duel.shots.push({ kind: "bolt", owner: me.index, x: me.x + me.face * 8, y, vx: me.face * spell.speed, t: 0, key, spell, name: me.act.spell, power: hit, stacks: me.act.stacks ?? 1 });
  } else if (spell.kind === "orb") {
    const x = me.x + me.face * 10;
    const key = play(duel, spell.fx, x, g, { seed: me.index + 3 });
    duel.shots.push({ kind: "orb", owner: me.index, x, t: 0, key, spell, name: me.act.spell, power: hit, stacks: me.act.stacks ?? 1 });
  } else if (spell.kind === "zone") {
    const x = clamp(me.act.target, duel.left + 6, duel.right - 6);
    // what hangs in the air (a collapse) gathers at the foe's height; what falls (an icicle) hits the floor
    const air = spell.grounded ? 0 : foe.air;
    // a barrage falls strike after strike, each where the foe will be when it lands
    // a sealed spell first marks the spot with the caster's seal, which the foe can see and leave
    const sigil = spell.sigil ?? 0;
    const mark = sigil > 0 && duel.has(CAST_CIRCLE) ? play(duel, CAST_CIRCLE, x, g - air, { palette: SCHOOLS[me.school].circle, seed: me.index + 11 }) : null;
    for (let k = 0; k < (spell.strikes ?? 1); k++) {
      duel.shots.push({ kind: "zone", name: me.act.spell, owner: me.index, x, air, t: -k * (spell.every ?? 0) - sigil, lead: sigil, key: k === 0 ? mark : null, spell, follow: k > 0, power: hit, stacks: me.act.stacks ?? 1 });
    }
    streak(duel, me, x, air + zoneHeight(spell, foe));
  } else if (spell.kind === "blast") {
    const x = me.x + me.face * spell.range * 0.7;
    cast(duel, me, spell.fx, x, g - me.spec.chest - me.air);
    if (Math.abs(foe.x - me.x) <= spell.range + foe.spec.halfWidth && Math.abs(foe.air - me.air) < foe.spec.body && foe.hp > 0) {
      if (blocks(duel, foe, me.x, hit)) clash(duel, foe.x - me.face * 4, g - foe.spec.chest - foe.air, [foe]);
      else if (hurt(duel, foe, hit, me.face, me.act.spell, me.act.stacks) && spell.shock) addShock(duel, foe, me, me.act.stacks ?? 1);
    }
  } else if (spell.kind === "void") {
    const x = clamp(foe.x, duel.left + 10, duel.right - 10);
    const key = cast(duel, me, spell.fx, x, g, { seed: me.index + 5 });
    duel.shots.push({ kind: "void", name: me.act.spell, owner: me.index, x, t: 0, tick: 0, key, spell, power: hit, stacks: me.act.stacks ?? 1 });
    streak(duel, me, x, spell.height ? spell.height : 4);
  } else if (spell.kind === "dash") {
    // gone in a flash and back a stride past the foe, cutting through it on the way
    const from = me.x;
    const to = clamp(me.x + me.face * spell.distance, duel.left + me.spec.halfWidth, duel.right - me.spec.halfWidth);
    cast(duel, me, spell.fx, from, g - me.air);
    me.x = to;
    cast(duel, me, spell.fx, to, g - me.air);
    emit(duel, { type: "streak", by: me.index, x0: from, y0: me.air + me.spec.chest, x1: to, y1: me.air + me.spec.chest });
    const lo = Math.min(from, to) - foe.spec.halfWidth;
    const hi = Math.max(from, to) + foe.spec.halfWidth;
    if (foe.x > lo && foe.x < hi && Math.abs(foe.air - me.air) < foe.spec.body && foe.hp > 0) {
      if (blocks(duel, foe, from, hit)) clash(duel, foe.x, g - foe.spec.chest - foe.air, [foe]);
      else hurt(duel, foe, hit, me.face, me.act.spell, me.act.stacks);
    }
  } else if (spell.kind === "heal") {
    cleanse(me);
    const gained = Math.min(100 - me.hp, spell.amount);
    me.hp += gained;
    emit(duel, { type: "heal", by: me.index, amount: Math.round((gained / 100) * me.stats.hp), x: me.x, y: me.air + me.spec.chest });
    cast(duel, me, spell.fx, me.x, g - me.spec.body - me.air);
  } else if (spell.kind === "ward") {
    cleanse(me);
    me.charges = 1 + Math.floor(duel.rng() * WARD.charges);
    me.shieldT = WARD.time;
    emit(duel, { type: "ward", by: me.index, amount: me.charges, x: me.x, y: me.air + me.spec.chest });
    cast(duel, me, spell.fx, me.x, g - me.spec.body - me.air);
  } else if (spell.kind === "barrier") {
    dropBarrier(duel, me);
    me.barrier = spell.time;
    me.barrierSpell = me.act.spell;
    me.barrierKey = cast(duel, me, spell.fx, me.x, g - me.air, { seed: me.index + 21 });
    emit(duel, { type: "barrier", by: me.index, x: me.x, y: me.air + me.spec.chest });
  } else if (spell.kind === "blink") {
    cast(duel, me, spell.fx, me.x, g - me.air);
    me.x = clamp(me.act.target, duel.left + me.spec.halfWidth, duel.right - me.spec.halfWidth);
    cast(duel, me, spell.fx, me.x, g - me.air);
  }
}

/**
 * Does f block a blow of `damage` from fromX? A block costs qi and still lets a little damage through (chip), so
 * no fight stalls behind a guard; with too little qi left the guard breaks and the blow lands.
 */
function blocks(duel, f, fromX, damage = 0) {
  if (f.guard === 0 || f.body.reeling() || Math.sign(fromX - f.x) !== f.face) return false;
  if (f.qi < GUARD_COST) {
    f.guard = 0;
    return false;
  }
  f.qi -= GUARD_COST;
  // a blow too heavy for the guard breaks through whole
  if (duel.rng() >= blockChance(damage)) return false;
  f.hp = Math.max(0, f.hp - damage * CHIP);
  return true;
}

/**
 * Does f's barrier turn this blow aside? An ultimate breaks through it. A golden bell rings back at a blade, palm
 * or dash that struck it from close by: the striker takes a blow of its own, is stunned and thrown off.
 */
function turnsAside(duel, f, dir, spell) {
  const s = SPELLS[spell];
  if (f.barrier === 0 || s?.ultimate) return false;
  turnAside(duel, f, f.x - dir * 8, duel.ground - f.spec.chest - f.air);
  const counter = SPELLS[f.barrierSpell].counter;
  const by = duel.fighters[1 - f.index];
  const close = Math.abs(by.x - f.x) < 60 && Math.abs(by.air - f.air) < f.spec.body;
  if (counter && close && (s?.kind === "melee" || s?.kind === "blast" || s?.kind === "dash")) {
    emit(duel, { type: "streak", by: f.index, x0: f.x, y0: f.air + f.spec.chest, x1: by.x, y1: by.air + by.spec.chest });
    if (hurt(duel, by, counter.damage, Math.sign(by.x - f.x) || 1, f.barrierSpell)) {
      control(duel, by, "stun", counter.stun);
    }
  }
  return true;
}

/** The barrier rings where a blow struck it. */
function turnAside(duel, f, x, y) {
  f.rang = RING;
  emit(duel, { type: "block", by: f.index, x: f.x, y: f.air + f.spec.chest });
  if (duel.has(CLASH)) cast(duel, f, CLASH, x, y + 30, { height: 30, seed: f.index + 31 });
}

function dropBarrier(duel, f) {
  if (f.barrierKey !== null) emit(duel, { type: "stop", key: f.barrierKey });
  f.barrier = 0;
  f.barrierKey = null;
}

/**
 * f takes a blow of `power` from `spell`, pushed toward `dir`; the step reports it as a `hit` command (or a
 * `dodge`). Returns whether it landed.
 */
function hurt(duel, f, power, dir, spell, stacks = 1) {
  if (f.hp <= 0 || f.body.untouchable()) return false;
  if (turnsAside(duel, f, dir, spell)) return false;
  const by = duel.fighters[1 - f.index];
  // a ward's charge erases the whole blow
  if (f.charges > 0) {
    f.charges--;
    clash(duel, f.x, duel.ground - f.spec.chest - f.air, [f]);
    return false;
  }
  const s = SPELLS[spell];
  // an ultimate cannot be dodged
  if (!s?.ultimate && duel.rng() < f.stats.dodge) {
    f.dodged = 0.3;
    f.vx -= dir * 40;
    emit(duel, { type: "dodge", x: f.x, y: f.air + f.spec.chest });
    return false;
  }
  // a payoff spends the stacks of its status on f, a little more power for each
  const pay = PAYOFF[spell];
  if (pay) {
    const stacks = pay.status === "shock" ? (f.shock?.stacks ?? 0) : f.chill;
    power *= 1 + pay.per * stacks;
    if (pay.status === "shock") f.shock = null;
    else {
      f.chill = 0;
      f.chillT = 0;
    }
  }
  const crit = duel.rng() < by.stats.crit;
  const amount = damageOf(by, f, power, s?.type ?? "magic", crit, duel.rng());
  const share = take(duel, f, amount);
  // a frost blow freezes where it strikes: it holds the target in place rather than throwing it
  const heavy = duel.Body.heavy(power, !!s?.freeze);
  const ranged = s?.kind === "bolt" || s?.kind === "orb";
  emit(duel, { type: "hit", by: by.index, spell, power, launch: heavy, ranged, damage: amount, crit, share, x: f.x, y: f.air + f.spec.chest });
  // rage for the blow: an attuned fighter's spells were paid for when cast, so only its blade earns here
  if (!s?.ultimate && !(by.stats.attuned && s?.kind !== "melee")) gainRage(by, 1);
  by.mind.tell({ type: "landed", skill: spell });
  if (by.stats.fury) by.fury = Math.min(FURY.max, by.fury + 1);
  // a heavy blow throws it up, where it can be hit again before it lands; a lighter one staggers it where it stands
  f.body.hit(power, dir, !!s?.freeze, f.fly);
  f.fly = false;
  // struck while recovering from a move: the move was punished, and its mind learns so
  if (f.act?.phase === "recover") f.mind.tell({ type: "punished", skill: f.act.spell });
  // a hit breaks a cast: what was gathering is lost
  if (f.act?.circle !== undefined && f.act.phase === "windup") emit(duel, { type: "stop", key: f.act.circle });
  f.act = null;
  f.guard = 0;
  setMeditating(duel, f, false);
  // what the spell leaves behind
  if (s?.burn) f.burn = { t: BURN.time, tick: BURN.every, per: Math.round(by.stats.atk * (1 + FURY.per * by.fury) * BURN.share), by: by.index };
  if (s?.defDown) f.defDown = s.defDown;
  if (s?.silence) f.silence = s.silence;
  if (s?.chill) addChill(duel, f, stacks);
  if (s?.stealRage && f.rage > 0) {
    f.rage -= s.stealRage;
    gainRage(by, s.stealRage);
  }
  return true;
}

/** The damage `by` does to `f` with a blow of `power` and `type`; `roll` in [0, 1) spreads it. */
export function damageOf(by, f, power, type, crit, roll) {
  const atk = by.stats.atk * (1 + FURY.per * by.fury);
  const def = (type === "physical" ? f.stats.pdef : f.stats.mdef) * (f.defDown > 0 ? 1 - DEF_DOWN : 1);
  const spread = 1 - SPREAD / 2 + SPREAD * roll;
  return Math.max(1, Math.round(atk * (power / 10) * (DEFENCE / (DEFENCE + def)) * (crit ? by.stats.critDamage : 1) * spread));
}

/**
 * f loses `amount` life (a ward's charges stop blows in `hurt`, never this), and an unyielding fighter stands up once a round from
 * a killing blow with 1 life left. Returns the share of full life lost.
 */
function take(duel, f, amount) {
  let share = (amount / f.stats.hp) * 100;
  if (share >= f.hp && f.stats.unyielding && !f.rose) {
    f.rose = true;
    share = f.hp - 100 / f.stats.hp;
    emit(duel, { type: "unyielding", by: f.index, x: f.x, y: f.air + f.spec.chest });
  }
  f.hp = Math.max(0, f.hp - share);
  return share;
}

/** A burn ticks: a share of its caster's attack, past defence, until it runs out. */
function burning(duel, f, dt) {
  if (!f.burn) return;
  f.burn.t -= dt;
  f.burn.tick -= dt;
  if (f.burn.tick <= 0 && f.hp > 0) {
    f.burn.tick += BURN.every;
    const share = take(duel, f, f.burn.per);
    emit(duel, { type: "tick", by: f.burn.by, damage: f.burn.per, share, x: f.x, y: f.air + f.spec.chest });
  }
  if (f.burn.t <= 0) f.burn = null;
}

function gainRage(f, n) {
  f.rage = Math.min(RAGE, f.rage + n);
}

/** With `chance`, a shock paralyses f for a moment. */
/** f's shock stacks twitch, one a tick, faster the more there are. */
function shocking(duel, f, dt) {
  if (!f.shock) return;
  f.shock.tick -= dt;
  if (f.shock.tick > 0 || f.hp <= 0) return;
  const by = duel.fighters[f.shock.by];
  const amount = Math.round(by.stats.atk * SHOCK.share);
  const share = take(duel, f, amount);
  emit(duel, { type: "tick", by: by.index, damage: amount, share, x: f.x, y: f.air + f.spec.chest });
  control(duel, f, "hold", SHOCK.hold);
  f.shock.stacks--;
  if (f.shock.stacks <= 0) f.shock = null;
  else f.shock.tick = shockInterval(f.shock.stacks);
}

function clash(duel, x, y, fighters) {
  emit(duel, { type: "clash", x, y: duel.ground - y });
  if (duel.has(CLASH)) play(duel, CLASH, x, y + 30, { height: 30, seed: Math.floor(duel.rng() * 1e6) });
  for (const f of fighters) f.vx += Math.sign(f.x - x || 1) * KNOCKBACK;
}

function knockOut(duel, f) {
  const other = duel.fighters[1 - f.index];
  emit(duel, { type: "ko", x: f.x, y: f.air + f.spec.chest });
  // the knockout throws the loser off its feet, if the blow has not already
  f.body.knockOut(other.x);
  f.fly = false;
  other.fly = false;
  duel.winner = other.index;
  duel.wins[other.index]++;
  f.act = null;
  setMeditating(duel, f, false);
  other.act = null;
  setMeditating(duel, other, false);
}

/** Between rounds: the loser falls and lies where it lands; the winner comes down and stands over it. */
function settle(duel, f, dt) {
  const foe = duel.fighters[1 - f.index];
  for (const e of f.body.settle(dt, f.index === duel.winner, f.hp > 0, foe.x, duel.arena, duel.rng)) emit(duel, e);
}

function setMeditating(duel, f, on) {
  if (f.medit === on) return;
  f.medit = on;
  if (on && duel.has("fire/spirit-flame")) {
    f.aura = play(duel, "fire/spirit-flame", f.x, duel.ground - 8, { seed: f.index + 11 });
  } else if (!on && f.aura !== null) {
    emit(duel, { type: "stop", key: f.aura });
    f.aura = null;
  }
}

// ---------- spells in flight ----------

function shot(duel, s, dt) {
  s.t += dt;
  const foe = duel.fighters[1 - s.owner];
  const g = duel.ground;
  const { spell } = s;
  if (s.kind === "bolt") {
    s.x += s.vx * dt;
    emit(duel, { type: "move", key: s.key, x: s.x, y: g - s.y });
    // two bolts meet head on, or a bolt meets an orb: both are spent
    for (const o of duel.shots) {
      if (o === s || o.owner === s.owner) continue;
      if (o.kind === "bolt" && Math.abs(o.x - s.x) < 6 && Math.abs(o.y - s.y) < 8) {
        clash(duel, (o.x + s.x) / 2, g - s.y, []);
        endShot(duel, o);
        return endShot(duel, s);
      }
      if (o.kind === "orb") {
        const [bx, by] = orbAt(o);
        if (Math.hypot(bx - s.x, by + s.y) < 10) {
          burst(duel, o.spell.hitFx, bx, g + by);
          endShot(duel, o);
          return endShot(duel, s);
        }
      }
    }
    // an ice mirror sends the bolt back the way it came, now its own
    if (foe.barrier > 0 && SPELLS[foe.barrierSpell].reflect && overlaps(foe, s.x, s.y, 9)) {
      s.owner = foe.index;
      s.vx = -s.vx;
      s.x += Math.sign(s.vx) * 10;
      turnAside(duel, foe, s.x, g - s.y);
      // the effect cannot turn round in flight: the old bolt goes out and a new one flies back, facing its new way
      // and in the colours of the one who sent it back
      stopShot(duel, s);
      s.key = play(duel, spell.fx, s.x, g - s.y, { palette: SCHOOLS[foe.school].bolt, flip: s.vx < 0, seed: foe.index + 7 });
      return;
    }
    if (overlaps(foe, s.x, s.y, 3)) {
      if (blocks(duel, foe, s.x - s.vx, s.power)) clash(duel, s.x, g - s.y, []);
      else if (hurt(duel, foe, s.power, Math.sign(s.vx), s.name, s.stacks) && spell.shock) addShock(duel, foe, duel.fighters[s.owner], s.stacks);
      return endShot(duel, s);
    }
    if (s.x < duel.left - 40 || s.x > duel.right + 40) endShot(duel, s);
  } else if (s.kind === "orb") {
    // the orb's origin drifts toward the foe; the ball itself wanders round it on its own loop
    s.x += Math.sign(foe.x - s.x) * spell.speed * dt;
    emit(duel, { type: "move", key: s.key, x: s.x, y: g });
    const [bx, by] = orbAt(s);
    if (overlaps(foe, bx, -by, spell.radius)) {
      if (blocks(duel, foe, bx, s.power)) clash(duel, bx, g + by, []);
      else hurt(duel, foe, s.power, Math.sign(foe.x - bx) || 1, s.name, s.stacks);
      burst(duel, spell.hitFx, bx, g + by);
      return endShot(duel, s);
    }
    if (s.t >= spell.life) endShot(duel, s);
  } else if (s.kind === "zone") {
    // a strike of a barrage aims where the foe was a moment before it falls, so a foe on the move gets out from under it
    if (s.follow && !s.aimed && s.t >= -spell.aimAhead) {
      s.aimed = true;
      s.x = clamp(foe.x, duel.left + 6, duel.right - 6);
      s.air = spell.grounded ? 0 : foe.air;
    }
    if (!s.shown && s.t >= 0) {
      s.shown = true;
      if (s.key !== null) emit(duel, { type: "stop", key: s.key });
      s.key = null;
      cast(duel, duel.fighters[s.owner], spell.fx, s.x, g - zoneHeight(spell, foe) - s.air);
    }
    if (s.t < 0) return;
    // a collapse drags the foe toward its heart while it sucks in
    if (spell.pull && s.t < spell.pullUntil && Math.abs(s.x - foe.x) < spell.reach && foe.hp > 0) {
      const dx = s.x - foe.x;
      foe.x += Math.sign(dx) * Math.min(Math.abs(dx), spell.pull * dt);
    }
    // a prison locks the space round it until it falls: no blinking out
    if (spell.seals && !s.fell && Math.abs(s.x - foe.x) < spell.reach) foe.sealed = 2 * dt;
    if (!s.fell && s.t >= spell.impact) {
      s.fell = true;
      const reaches = Math.abs(foe.x - s.x) < spell.radius + foe.spec.halfWidth && (spell.grounded ? foe.air < 10 : Math.abs(foe.air - s.air) < 22);
      if (reaches && hurt(duel, foe, s.power, Math.sign(foe.x - s.x) || 1, s.name, s.stacks)) {
        if (spell.freeze) control(duel, foe, "freeze", spell.freeze);
        if (spell.shock) addShock(duel, foe, duel.fighters[s.owner], s.stacks);
      }
    }
    if (s.t >= spell.impact + 0.2) duel.shots.splice(duel.shots.indexOf(s), 1);
  } else if (s.kind === "void") {
    // the vortex drags the foe in, and burns while it stands under it
    const dx = s.x - foe.x;
    if (Math.abs(dx) < spell.reach && foe.hp > 0) foe.x += Math.sign(dx) * Math.min(Math.abs(dx), spell.pull * dt);
    // it burns whoever stands under it, a tick every half second
    s.tick += dt;
    if (s.tick >= 0.5) {
      s.tick -= 0.5;
      if (Math.abs(dx) < spell.under && foe.hp > 0) {
        const by = duel.fighters[s.owner];
        const amount = damageOf(by, foe, spell.tick, spell.type, false, duel.rng());
        const share = take(duel, foe, amount);
        emit(duel, { type: "tick", by: by.index, damage: amount, share, x: foe.x, y: foe.air + foe.spec.chest });
        // a cold fire chills whoever stands in it, a stack a tick
        if (spell.chill) addChill(duel, foe, s.stacks);
      }
    }
    if (s.t >= spell.life) {
      burst(duel, spell.endFx, s.x, g - spell.height);
      if (Math.abs(dx) < spell.under) hurt(duel, foe, s.power, -Math.sign(dx) || 1, s.name, s.stacks);
      endShot(duel, s);
    }
  }
}

/** Pressed against either edge of the arena. */
function orbAt(s) {
  const [ox, oy] = orbOffset(s.t);
  return [s.x + ox, oy];
}

/** Does a point (x, height above ground) touch fighter f, give or take r? */
/** How high a zone strikes: what hangs in the air (a collapse) at the foe's chest, what falls (an icicle) at the floor. */
function zoneHeight(spell, foe) {
  return spell.grounded ? 0 : foe.spec.chest;
}

function overlaps(f, x, y, r) {
  if (f.body.untouchable()) return false;
  return Math.abs(x - f.x) < f.spec.halfWidth + r && y > f.air - r && y < f.air + (f.duck > 0 ? f.spec.duck : f.spec.body) + r;
}

function burst(duel, fx, x, y) {
  if (fx && duel.has(fx)) play(duel, fx, x, y, { seed: Math.floor(duel.rng() * 1e6) });
}

function endShot(duel, s) {
  stopShot(duel, s);
  duel.shots.splice(duel.shots.indexOf(s), 1);
}

function stopShot(duel, s) {
  if (s.key !== null) emit(duel, { type: "stop", key: s.key });
}

// ---------- minds ----------

/** Can `me` cast `name` now: in its kit, on its bar if it is a bar skill, off cooldown, paid for. */
export const ready = (me, name) =>
  me.spells.includes(name) && (!me.pool.includes(name) || me.bar.some((slot) => slot.spell === name && slot.left > 0)) && !(me.cool[name] > 0) && me.qi >= SPELLS[name].cost && (!SPELLS[name].ultimate || me.rage >= RAGE) &&
  !(name === "blink" && me.sealed > 0) &&
  // silenced, only the blade answers
  !(me.silence > 0 && SPELLS[name].kind !== "melee");

/**
 * What fighter f's skill bar shows, slot by slot (1, 2, 3, then the ultimate): whether any spell in it can be cast
 * now, the cooldown it has `left` out of the `full` cooldown it began, and how charged it is (an ultimate charges
 * with rage; a slot with none of its spells in the fight is `empty`).
 */
export function skillBar(f) {
  const slots = f.bar.map((slot) =>
    slot.empty !== undefined
      ? { spell: null, empty: true, ultimate: false, ready: false, left: slot.empty, full: slot.full, charge: 1, casts: 0, power: 1, evolved: false }
      : {
        spell: slot.spell,
        empty: false,
        ultimate: false,
        ready: f.hp > 0 && ready(f, slot.spell),
        left: f.cool[slot.spell] ?? 0,
        full: f.coolFull[slot.spell] ?? 0,
        charge: 1,
        casts: slot.left,
        power: slot.power,
        evolved: slot.evolved,
      });
  while (slots.length < BAR.slots) slots.push({ spell: null, empty: true, ultimate: false, ready: false, left: 0, full: 0, charge: 1, casts: 0, power: 1, evolved: false });
  const ult = SCHOOLS[f.school].ultimate;
  const owned = f.spells.includes(ult);
  slots.push({ spell: owned ? ult : null, empty: !owned, ultimate: true, ready: owned && f.hp > 0 && ready(f, ult), left: f.cool[ult] ?? 0, full: f.coolFull[ult] ?? 0, charge: f.rage / RAGE, casts: 0, power: 1, evolved: false });
  return slots;
}

/**
 * Fill slot `i` of f's bar with a skill from its pool that is not on the bar, with 1 to 4 casts. A round's first
 * draw is of the school's signature kind; while the foe carries a status, one draw in `PAY_DRAW` is a skill that
 * spends it.
 */
export function draw(duel, f, i, first = false) {
  const held = f.bar.filter((slot, j) => j !== i && slot.spell).map((slot) => slot.spell);
  let free = f.pool.filter((name) => !held.includes(name));
  const foe = duel.fighters[1 - f.index];
  const carried = (status) => (status === "shock" ? (foe.shock?.stacks ?? 0) : foe.chill) > 0;
  const want = first
    ? free.filter((name) => SPELLS[name].kind === SCHOOLS[f.school].signature)
    : duel.rng() < PAY_DRAW ? free.filter((name) => PAYOFF[name] && carried(PAYOFF[name].status)) : [];
  if (want.length > 0) free = want;
  if (free.length === 0) {
    // nothing left to draw: wait and look again
    f.bar[i] = { empty: BAR.wait, full: BAR.wait };
    return;
  }
  const spell = free[Math.floor(duel.rng() * free.length)];
  const casts = 1 + Math.floor(duel.rng() * BAR.casts);
  f.bar[i] = { spell, casts, left: casts, power: castsPower(casts), evolved: false };
}

/** A heal or a ward washes off what clings to the caster: burn, silence, defence-down, chill and shock. */
function cleanse(f) {
  Object.assign(f, { burn: null, silence: 0, defDown: 0, chill: 0, chillT: 0, shock: null });
}

/**
 * One cast of slot `i`'s skill; the last empties the slot, which counts down before it draws again, unless the skill's
 * partner is on the bar: then it refills at once, evolved.
 */
export function spend(duel, f, i) {
  const slot = f.bar[i];
  if (!slot || slot.empty !== undefined) return;
  slot.left--;
  if (slot.left > 0) return;
  const pair = EVOLVES[f.school]?.find((names) => names.includes(slot.spell));
  const partner = pair?.find((name) => name !== slot.spell);
  if (!slot.evolved && partner && f.bar.some((other, j) => j !== i && other.spell === partner)) {
    const casts = 1 + Math.floor(duel.rng() * BAR.casts);
    f.bar[i] = { spell: slot.spell, casts, left: casts, power: EVOLVED * castsPower(casts), evolved: true };
    return;
  }
  const wait = BAR.wait + BAR.spread * duel.rng();
  f.bar[i] = { empty: wait, full: wait };
}

// ---------- the brain: what a fighter sees, and doing what it chose ----------

/** The kind pixfx-mind knows each spell kind as. */
const KIND = { melee: "melee", dash: "dash", blast: "blast", bolt: "bolt", orb: "seeker", zone: "area", void: "field", heal: "heal", ward: "ward", barrier: "barrier", blink: "blink" };

/** A spell as pixfx-mind reads it: what it does, never what it is called (a heal's `damage` is what it heals). */
function skillOf(name) {
  const s = SPELLS[name];
  const reach = s.kind === "dash" || s.kind === "blink" ? s.distance : s.kind === "melee" || s.kind === "blast" ? s.range : s.reach ?? 0;
  return {
    name, kind: s.dive ? "dive" : KIND[s.kind], cost: s.cost, windup: s.windup, recover: s.recover,
    damage: (s.kind === "heal" ? s.amount : s.damage) ?? 0, reach, radius: s.radius ?? 0, speed: s.speed ?? 0,
    lands: s.kind === "void" ? s.life : (s.sigil ?? 0) + (s.impact ?? 0), strikes: s.strikes ?? 1, ultimate: !!s.ultimate, grounded: !!s.grounded,
    inflicts: { freeze: s.freeze ?? 0, shock: !!s.shock, silence: s.silence ?? 0, defDown: s.defDown ?? 0, burn: !!s.burn, chill: !!s.chill, pull: s.pull ?? 0 },
  };
}
const SKILLS = Object.fromEntries(Object.keys(SPELLS).map((name) => [name, skillOf(name)]));

/** Fighter f's brain: pixfx-mind's `Mind` with its school's skills and temperament, in the shape `minds` takes. */
function brain(duel, f, Mind) {
  if (!Mind) throw new Error("createDuel: pass Mind (pixfx-mind's brain, from the engine's wasm package)");
  const school = SCHOOLS[f.school];
  const profile = { ...TEMPERAMENT, aggression: school.aggression, tempo: school.tempo, range: school.range, dry: school.dry, sky: school.sky, postureScale: school.postures };
  const { halfWidth, chest, body, duck, walk } = f.spec;
  const mind = new Mind(profile, { halfWidth, chest, body, duck, walk }, f.spells.map((name) => SKILLS[name]), (duel.seed * 2 + f.index) >>> 0);
  mind.meet(duel.fighters[1 - f.index].spells.map((name) => SKILLS[name]));
  return {
    think(duel, me, foe, dt) {
      const thought = mind.think(senseOf(duel, me, foe), dt);
      if (!thought) return null;
      me.thought = thought;
      return thought.choice;
    },
    tell: (outcome) => mind.tell(outcome),
    reset: () => mind.reset(),
    hold: (seconds) => mind.hold(seconds),
    explain: (on) => mind.explain(on),
    get posture() {
      return mind.posture;
    },
  };
}

/** What fighter me could perceive this tick, as pixfx-mind reads it: the true state; it sees late on its own. */
function senseOf(duel, me, foe) {
  const free = !(me.act || me.body.reeling() || me.squat > 0 || me.duck > 0 || me.frozen > 0 || me.shocked > 0 || me.hp <= 0);
  return {
    time: duel.time,
    me: {
      x: me.x, air: me.air, face: me.face, hp: me.hp, qi: me.qi, rage: me.rage, rageFull: RAGE, flying: me.fly, hover: me.hover, vx: me.vx,
      canAct: free, silenced: me.silence > 0, sealed: me.sealed > 0, barrier: me.barrier > 0, shield: me.charges > 0,
      seekerOut: duel.shots.some((s) => s.kind === "orb" && s.owner === me.index),
    },
    foe: {
      x: foe.x, air: foe.air, flying: foe.fly, hp: foe.hp, qi: foe.qi, doing: doingOf(foe), outFor: foe.body.outFor(), openIn: foe.body.openIn(),
      frozen: foe.frozen, shocked: foe.shocked, barrier: foe.barrier > 0,
      ready: foe.spells.map((name) => ready(foe, name)),
    },
    arena: duel.arena,
    threats: threatsOf(duel, me),
    ready: me.spells.map((name) => ready(me, name)),
  };
}

/** What fighter f is doing, as pixfx-mind reads it. */
function doingOf(f) {
  if (f.act) {
    const s = SKILLS[f.act.spell];
    const spell = SPELLS[f.act.spell];
    return { type: "act", kind: s.kind, phase: f.act.phase, t: f.act.t, windup: spell.windup, recover: spell.recover, reach: s.reach, damage: s.damage };
  }
  if (f.guard > 0) return { type: "guard" };
  if (f.medit) return { type: "meditate" };
  if (f.fly) return { type: "fly" };
  if (f.intent.move !== 0) return { type: "move", dir: f.intent.move };
  return { type: "idle" };
}

/** The foe's shots that could reach me, each with how long it has been there to see. */
function threatsOf(duel, me) {
  const out = [];
  for (const s of duel.shots) {
    if (s.owner === me.index) continue;
    const damage = s.spell.damage ?? 0;
    if (s.kind === "bolt") out.push({ age: s.t, damage, shape: { type: "projectile", x: s.x, y: s.y, vx: s.vx } });
    else if (s.kind === "orb") {
      const [x, y] = orbAt(s);
      out.push({ age: s.t, damage, shape: { type: "seeker", x, y: -y, radius: s.spell.radius } });
    } else if (s.kind === "zone" && !s.fell) {
      // a barrage still falling: its next strike aims where I stand
      if (s.follow && s.t < 0) out.push({ age: s.t + s.spell.every + s.spell.aimAhead, damage, shape: { type: "barrage", landsIn: -s.t } });
      // a sealed spell is seen from the moment its seal shows, a falling one once it trembles
      else out.push({ age: s.t + (s.lead ?? 0) - s.spell.notice, damage, shape: { type: "area", x: s.x, radius: s.spell.radius, reach: s.spell.reach ?? 0, landsIn: s.spell.impact - s.t, grounded: !!s.spell.grounded } });
    } else if (s.kind === "void") out.push({ age: s.t, damage, shape: { type: "field", x: s.x, reach: s.spell.reach } });
  }
  return out;
}

/** Fighter me does what its mind chose. */
function apply(duel, me, choice) {
  me.intent = { move: 0 };
  if (choice.type !== "meditate") setMeditating(duel, me, false);
  if (choice.type === "move") me.intent = { move: choice.dir };
  else if (choice.type === "jump") {
    // crouch first, then spring
    if (me.air === 0) {
      me.squat = me.spec.jumpSquat;
      me.leap = me.spec.jumpSpeed;
    }
  } else if (choice.type === "guard") me.guard = choice.hold;
  else if (choice.type === "duck") me.duck = choice.hold;
  else if (choice.type === "fly") {
    me.fly = choice.height > 0;
    me.hover = choice.height;
    if (me.fly && me.air === 0) me.vy = me.spec.jumpSpeed * 0.6;
  } else if (choice.type === "meditate") setMeditating(duel, me, true);
  else if (choice.type === "cast") begin(duel, me, choice.skill, choice.target ?? undefined);
}

// ---------- effect commands ----------

function play(duel, fx, x, y, opts) {
  const key = ++duel.key;
  emit(duel, { type: "play", key, fx, x, y, opts });
  return key;
}

/** Play `fx` for fighter `me`'s spell, in its school's colours. */
function cast(duel, me, fx, x, y, opts = {}) {
  return play(duel, fx, x, y, { ...SCHOOLS[me.school].tint?.[fx], seed: Math.floor(duel.rng() * 1e6), ...opts });
}

/** A streak of `me`'s qi from where it casts to where the spell lands (heights up from the ground). */
function streak(duel, me, x, y) {
  emit(duel, { type: "streak", by: me.index, x0: me.x + me.face * 6, y0: me.air + me.spec.chest, x1: x, y1: y });
}

function emit(duel, command) {
  duel.out.push(command);
}

function clamp(v, lo, hi) {
  return Math.max(lo, Math.min(hi, v));
}
