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
const SHOCK = { time: 0.4, chance: 0.3 };
/**
 * How a blow turns into damage, the way xianxia hero battlers count it: attack times the skill's percentage, cut by
 * the defence that matches its type (physical or magic), then a crit roll, a little spread, and a dodge roll before
 * it all. Spell `damage` is the skill's power: power 10 is 100% of attack. Life (`hp`) is kept as a share of the
 * fighter's full life, 0 to 100; hits report the real amounts.
 */
const DEFENCE = 500;
const SPREAD = 0.1;
/** Fury: each blow a fury fighter lands raises its attack, up to a cap of stacks, for the rest of the round. */
const FURY = { per: 0.04, max: 5 };
/** Defence broken by a spell: this much defence lost, for the spell's `defDown` seconds. */
const DEF_DOWN = 0.3;
/** Burn: a share of the caster's attack every tick, ignoring defence. */
const BURN = { time: 3, every: 0.5, share: 0.05 };
/** A frost ward absorbs this share of full life, for this long. */
const WARD = { share: 0.18, time: 6 };

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
/** How long a fighter takes to notice something new, and how often it thinks. */
const REACTION = 0.16;
const THINK = 0.12;
/**
 * How a fighter picks its moments, after Sirlin's Playing to Win: attacking at the first chance is predictable, so
 * after it commits to an attack it takes a breath in neutral (`tempo` in its school, seconds, spread by `spread`)
 * and attacks again sooner only to punish an opening. An attack is worth what it is likely to land, less what it
 * costs if it misses: the hardest blow the foe, free to answer, can land on it before it recovers, weighed at `risk`.
 * Standing in reach doing nothing is no safer, so that cost lowers an attack's worth but never below `floor` of it.
 * Ahead on life a fighter weighs the cost up to `lead` times more and plays safe; behind, less, and takes its chances.
 */
const MIND = { spread: 0.8, risk: 0.6, floor: 0.3, lead: 0.8 };
/** What a fighter has learnt of how often each of its attacks lands on this foe counts as this many tries at first. */
const PRIOR = 6;
/** Qi a block costs. */
const GUARD_COST = 8;
/** The share of a blocked blow's damage that still lands. */
const CHIP = 0.15;
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
    cost: 90, windup: 0.9, recover: 0.45, cooldown: 11, life: 3.2, height: 62, pull: 34, reach: 90, under: 22, tick: 4, damage: 14,
  },
  heal: { kind: "heal", fx: "holy/feather-burst", cost: 60, windup: 0.85, recover: 0.3, cooldown: 9, amount: 20 },
  // a shell of frost qi that takes blows for the caster until it breaks
  ward: { kind: "ward", fx: "holy/feather-burst", cost: 45, windup: 0.5, recover: 0.25, cooldown: 8 },
  // the defences: a dome of qi for a moment that turns every blow aside. Thunder's golden bell rings back at a
  // blade or palm that strikes it, stunning and throwing off the striker; Frost's ice mirror sends bolts back
  bell: { kind: "barrier", fx: "arcane/rune-circle", cost: 18, windup: 0.06, recover: 0.1, cooldown: 6.5, time: 2, counter: { damage: 9, stun: 0.6 } },
  mirror: { kind: "barrier", fx: "arcane/rune-circle", cost: 18, windup: 0.06, recover: 0.1, cooldown: 6.5, time: 2.2, reflect: true },
  blink: { kind: "blink", fx: "spawn/poof", cost: 20, windup: 0.06, recover: 0.12, cooldown: 1.8, distance: 56 },
};

/**
 * The two schools. A school's spells are its `basics` (the blade, the dive and the blink, each on its own cooldown)
 * and its skill `slots`, in the order its skill bar shows them: 1, 2, 3 and the ultimate. The spells in a slot share
 * one cooldown: casting any of them puts the whole slot on the cooldown of the one cast. Then the distance each
 * school likes to fight at, how eager it is, how long it takes a breath after each attack (`tempo`, seconds), the
 * distance it keeps when its qi runs low (`dry`: Thunder presses in to sword's length, Frost stays at middle distance
 * while its qi gathers again), the height it likes to hover at while it has the qi (`sky`; Frost casts from the air,
 * Thunder fights on the ground), and its colours.
 */
export const SCHOOLS = {
  thunder: {
    name: "Thunder", basics: ["slash", "dive", "blink"], slots: [["bolt", "palm", "step"], ["seal", "orb"], ["bell", "heal"], ["tribulation"]], range: 64, dry: SPELLS.slash.range - 4, aggression: 1.15, tempo: 0.8, sky: 22,
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
    name: "Frost", basics: ["slash", "dive", "blink"], slots: [["bolt", "lotus"], ["shatter", "icicle", "collapse", "frostfire", "void"], ["mirror", "ward"], ["prison"]], range: 100, dry: 80, aggression: 1.1, tempo: 2, sky: 48,
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
 * effect is in the registry. `Body` is pixfx-fighter's body (from the engine's wasm package): each fighter gets one.
 */
export function createDuel({ seed = 1, left, right, ground, has, sky = FLY.top, Body }) {
  // `sky`: how high above the ground a fighter may fly, below whatever hangs over the arena
  const top = Math.min(FLY.top, sky);
  const duel = { seed, rng: random(seed), left, right, ground, has, top, arena: { left, right, top }, Body, time: 0, round: 0, wins: [0, 0], key: 0, shots: [], pause: 0, out: [] };
  const usable = (name) => {
    const fx = SPELLS[name].fx;
    return !fx || has(fx);
  };
  duel.fighters = [fighter(0, "thunder", usable, new Body(0)), fighter(1, "frost", usable, new Body(0))];
  for (const name of Object.values(DECOR)) if (has(name)) duel.decor = true;
  startRound(duel);
  return duel;
}

/** The body's fields, read and written on the fighter as if they were its own. */
const BODY_FIELDS = ["x", "air", "vx", "vy", "face", "stun", "wake", "land", "squat", "leap", "duck", "flash", "hitT", "hitPower", "frozen", "shocked", "poseT"];

/** Fighter `index` of `school`, with the school's spells that are `usable`. */
function fighter(index, school, usable, body) {
  const { basics, slots } = SCHOOLS[school];
  const spells = [...basics, ...slots.flat()].filter(usable);
  // the cooldown each spell runs on: its slot's, or its own for a basic
  const timer = Object.fromEntries(spells.map((name) => {
    const i = slots.findIndex((slot) => slot.includes(name));
    return [name, i < 0 ? name : `slot${i}`];
  }));
  const f = { index, school, spells, timer, stats: STATS[school], hp: 100, qi: 100, act: null, guard: 0, medit: false, fly: false, hover: 0, rage: 0, sealed: 0, shield: 0, shieldT: 0, barrier: 0, barrierSpell: null, barrierKey: null, rang: 0, burn: null, defDown: 0, silence: 0, fury: 0, rose: false, dodged: 0, chilled: 0, cool: {}, coolFull: {}, think: 0, intent: { move: 0 }, aura: null, calm: 0, attune: 0, read: { tried: {}, landed: {} } , body, spec: body.spec };
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
    Object.assign(f, { hp: 100, qi: 100, act: null, guard: 0, medit: false, fly: false, hover: 0, rage: 0, sealed: 0, shield: 0, shieldT: 0, barrier: 0, barrierSpell: null, barrierKey: null, rang: 0, burn: null, defDown: 0, silence: 0, fury: 0, rose: false, dodged: 0, chilled: 0, cool: {}, coolFull: {}, think: 0.4 + duel.rng() * 0.3, intent: { move: 0 }, calm: 0 });
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
    mind(duel, me, foe, dt);
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
  me.calm = Math.max(0, me.calm - dt);
  me.sealed = Math.max(0, me.sealed - dt);
  me.defDown = Math.max(0, me.defDown - dt);
  me.silence = Math.max(0, me.silence - dt);
  me.dodged = Math.max(0, me.dodged - dt);
  me.chilled = Math.max(0, me.chilled - dt);
  me.shieldT = Math.max(0, me.shieldT - dt);
  if (me.shieldT === 0) me.shield = 0;
  me.rang = Math.max(0, me.rang - dt);
  if (me.barrier > 0) {
    me.barrier = Math.max(0, me.barrier - dt);
    if (me.barrier === 0) dropBarrier(duel, me);
  }
  burning(duel, me, dt);
  me.guard = Math.max(0, me.guard - dt);
  for (const k of Object.keys(me.cool)) me.cool[k] = Math.max(0, me.cool[k] - dt);
  me.qi = Math.min(100, me.qi + (me.medit ? MEDITATE_REGEN : me.fly ? -FLY.cost : QI_REGEN) * dt);
  // out of qi, or knocked out of the sky: flight ends and it falls
  if (me.fly && (me.qi <= 1 || me.body.reeling())) me.fly = false;
  // a dive drops straight at the foe's height
  const diving = me.act?.phase === "windup" && SPELLS[me.act.spell].dive;
  const motion = diving ? { dive: SPELLS.dive.speed, floor: foe.air } : me.fly ? { fly: me.hover } : "gravity";
  const drive = { alive: me.hp > 0, acting: me.act !== null, meditating: me.medit, guarding: me.guard > 0, chilled: me.chilled > 0, intent: me.intent.move, motion, foeX: foe.x };
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
  me.cool[me.timer[name]] = me.coolFull[me.timer[name]] = spell.cooldown + spell.windup;
  me.act = { spell: name, phase: "windup", t: 0, target };
  // an attack is a commitment: a breath in neutral follows it, and it counts toward what the fighter learns
  if (spell.damage) {
    me.calm = SCHOOLS[me.school].tempo * (1 - MIND.spread / 2 + MIND.spread * duel.rng());
    me.read.tried[name] = (me.read.tried[name] ?? 0) + 1;
  }
  setMeditating(duel, me, false);
  // longer casts stand in a rune circle while they gather
  if (spell.kind !== "melee" && spell.kind !== "blink" && spell.kind !== "dash" && spell.kind !== "barrier" && duel.has(CAST_CIRCLE)) {
    me.act.circle = play(duel, CAST_CIRCLE, me.x, duel.ground, { palette: SCHOOLS[me.school].circle, seed: me.index + 1 });
  }
}

/** The moment a spell lands: it hits, flies, falls or heals. */
function release(duel, me, foe, spell) {
  const g = duel.ground;
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
      if (blocks(foe, me.x, spell.damage)) clash(duel, (me.x + foe.x) / 2, g - me.spec.chest - me.air, [me, foe]);
      else if (foe.act?.spell === "slash" && foe.act.phase === "windup" && foe.act.t > SPELLS.slash.windup * 0.6) {
        // both blades swing at once: they meet
        clash(duel, (me.x + foe.x) / 2, g - me.spec.chest - me.air, [me, foe]);
        foe.act = null;
      } else hurt(duel, foe, spell.damage, me.face, me.act.spell);
    }
  } else if (spell.kind === "bolt") {
    const y = me.air + me.spec.chest;
    const key = play(duel, spell.fx, me.x + me.face * 8, g - y, { palette: SCHOOLS[me.school].bolt, flip: me.face < 0, seed: me.index + 7 });
    duel.shots.push({ kind: "bolt", owner: me.index, x: me.x + me.face * 8, y, vx: me.face * spell.speed, t: 0, key, spell });
  } else if (spell.kind === "orb") {
    const x = me.x + me.face * 10;
    const key = play(duel, spell.fx, x, g, { seed: me.index + 3 });
    duel.shots.push({ kind: "orb", owner: me.index, x, t: 0, key, spell });
  } else if (spell.kind === "zone") {
    const x = clamp(me.act.target, duel.left + 6, duel.right - 6);
    // what hangs in the air (a collapse) gathers at the foe's height; what falls (an icicle) hits the floor
    const air = spell.grounded ? 0 : foe.air;
    // a barrage falls strike after strike, each where the foe will be when it lands
    // a sealed spell first marks the spot with the caster's seal, which the foe can see and leave
    const sigil = spell.sigil ?? 0;
    const mark = sigil > 0 && duel.has(CAST_CIRCLE) ? play(duel, CAST_CIRCLE, x, g - air, { palette: SCHOOLS[me.school].circle, seed: me.index + 11 }) : null;
    for (let k = 0; k < (spell.strikes ?? 1); k++) {
      duel.shots.push({ kind: "zone", name: me.act.spell, owner: me.index, x, air, t: -k * (spell.every ?? 0) - sigil, lead: sigil, key: k === 0 ? mark : null, spell, follow: k > 0 });
    }
    streak(duel, me, x, air + zoneHeight(spell, foe));
  } else if (spell.kind === "blast") {
    const x = me.x + me.face * spell.range * 0.7;
    cast(duel, me, spell.fx, x, g - me.spec.chest - me.air);
    if (Math.abs(foe.x - me.x) <= spell.range + foe.spec.halfWidth && Math.abs(foe.air - me.air) < foe.spec.body && foe.hp > 0) {
      if (blocks(foe, me.x, spell.damage)) clash(duel, foe.x - me.face * 4, g - foe.spec.chest - foe.air, [foe]);
      else if (hurt(duel, foe, spell.damage, me.face, me.act.spell) && spell.shock) shock(duel, foe, SHOCK.chance);
    }
  } else if (spell.kind === "void") {
    const x = clamp(foe.x, duel.left + 10, duel.right - 10);
    const key = cast(duel, me, spell.fx, x, g, { seed: me.index + 5 });
    duel.shots.push({ kind: "void", name: me.act.spell, owner: me.index, x, t: 0, tick: 0, key, spell });
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
      if (blocks(foe, from, spell.damage)) clash(duel, foe.x, g - foe.spec.chest - foe.air, [foe]);
      else hurt(duel, foe, spell.damage, me.face, me.act.spell);
    }
  } else if (spell.kind === "heal") {
    const gained = Math.min(100 - me.hp, spell.amount);
    me.hp += gained;
    emit(duel, { type: "heal", by: me.index, amount: Math.round((gained / 100) * me.stats.hp), x: me.x, y: me.air + me.spec.chest });
    cast(duel, me, spell.fx, me.x, g - me.spec.body - me.air);
  } else if (spell.kind === "ward") {
    me.shield = Math.round(WARD.share * me.stats.hp);
    me.shieldT = WARD.time;
    emit(duel, { type: "ward", by: me.index, amount: me.shield, x: me.x, y: me.air + me.spec.chest });
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
function blocks(f, fromX, damage = 0) {
  if (f.guard === 0 || f.body.reeling() || Math.sign(fromX - f.x) !== f.face) return false;
  if (f.qi < GUARD_COST) {
    f.guard = 0;
    return false;
  }
  f.qi -= GUARD_COST;
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
      by.stun = Math.max(by.stun, counter.stun);
      by.shocked = Math.max(by.shocked, counter.stun);
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
function hurt(duel, f, power, dir, spell) {
  if (f.hp <= 0 || f.body.untouchable()) return false;
  if (turnsAside(duel, f, dir, spell)) return false;
  const by = duel.fighters[1 - f.index];
  const s = SPELLS[spell];
  // an ultimate cannot be dodged
  if (!s?.ultimate && duel.rng() < f.stats.dodge) {
    f.dodged = 0.3;
    f.vx -= dir * 40;
    emit(duel, { type: "dodge", x: f.x, y: f.air + f.spec.chest });
    return false;
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
  by.read.landed[spell] = (by.read.landed[spell] ?? 0) + 1;
  if (by.stats.fury) by.fury = Math.min(FURY.max, by.fury + 1);
  // a heavy blow throws it up, where it can be hit again before it lands; a lighter one staggers it where it stands
  f.body.hit(power, dir, !!s?.freeze, f.fly);
  f.fly = false;
  // a hit breaks a cast: what was gathering is lost
  if (f.act?.circle !== undefined && f.act.phase === "windup") emit(duel, { type: "stop", key: f.act.circle });
  f.act = null;
  f.guard = 0;
  setMeditating(duel, f, false);
  // what the spell leaves behind
  if (s?.burn) f.burn = { t: BURN.time, tick: BURN.every, per: Math.round(by.stats.atk * (1 + FURY.per * by.fury) * BURN.share), by: by.index };
  if (s?.defDown) f.defDown = s.defDown;
  if (s?.silence) f.silence = s.silence;
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
 * f loses `amount` life: its ward takes what it can first, and an unyielding fighter stands up once a round from
 * a killing blow with 1 life left. Returns the share of full life lost.
 */
function take(duel, f, amount) {
  const absorbed = Math.min(f.shield, amount);
  f.shield -= absorbed;
  let share = ((amount - absorbed) / f.stats.hp) * 100;
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
function shock(duel, f, chance) {
  if (f.hp > 0 && duel.rng() < chance) f.shocked = SHOCK.time;
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
      if (blocks(foe, s.x - s.vx, spell.damage)) clash(duel, s.x, g - s.y, []);
      else if (hurt(duel, foe, spell.damage, Math.sign(s.vx), "bolt")) shock(duel, foe, SHOCK.chance);
      return endShot(duel, s);
    }
    if (s.x < duel.left - 40 || s.x > duel.right + 40) endShot(duel, s);
  } else if (s.kind === "orb") {
    // the orb's origin drifts toward the foe; the ball itself wanders round it on its own loop
    s.x += Math.sign(foe.x - s.x) * spell.speed * dt;
    emit(duel, { type: "move", key: s.key, x: s.x, y: g });
    const [bx, by] = orbAt(s);
    if (overlaps(foe, bx, -by, spell.radius)) {
      if (blocks(foe, bx, spell.damage)) clash(duel, bx, g + by, []);
      else hurt(duel, foe, spell.damage, Math.sign(foe.x - bx) || 1, "orb");
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
      if (reaches && hurt(duel, foe, spell.damage, Math.sign(foe.x - s.x) || 1, s.name)) {
        if (spell.freeze) foe.frozen = spell.freeze;
        if (spell.shock) shock(duel, foe, spell.ultimate ? 1 : SHOCK.chance);
      }
    }
    if (s.t >= spell.impact + 0.2) duel.shots.splice(duel.shots.indexOf(s), 1);
  } else if (s.kind === "void") {
    // the vortex drags the foe in, and burns while it stands under it
    const dx = s.x - foe.x;
    if (Math.abs(dx) < spell.reach && foe.hp > 0) foe.x += Math.sign(dx) * Math.min(Math.abs(dx), spell.pull * dt);
    // frostfire slows whoever stands in it
    if (spell.chill && Math.abs(dx) < spell.under) foe.chilled = 0.1;
    // it burns whoever stands under it, a tick every half second
    s.tick += dt;
    if (s.tick >= 0.5) {
      s.tick -= 0.5;
      if (Math.abs(dx) < spell.under && foe.hp > 0) {
        const by = duel.fighters[s.owner];
        const amount = damageOf(by, foe, spell.tick, spell.type, false, duel.rng());
        const share = take(duel, foe, amount);
        emit(duel, { type: "tick", by: by.index, damage: amount, share, x: foe.x, y: foe.air + foe.spec.chest });
      }
    }
    if (s.t >= spell.life) {
      burst(duel, spell.endFx, s.x, g - spell.height);
      if (Math.abs(dx) < spell.under) hurt(duel, foe, spell.damage, -Math.sign(dx) || 1, s.name);
      endShot(duel, s);
    }
  }
}

/** Pressed against either edge of the arena. */
function atWall(duel, f) {
  return f.x <= duel.left + f.spec.halfWidth + 2 || f.x >= duel.right - f.spec.halfWidth - 2;
}

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

/**
 * A fighter thinks every THINK seconds while it is free: it sees only what has been on screen for REACTION seconds,
 * scores everything it could do (get out of the way, attack with each ready spell, move to its range, meditate),
 * and does the best, with a little noise so no two rounds play the same.
 */
function mind(duel, me, foe, dt) {
  me.think -= dt;
  if (me.think > 0 || me.act || me.body.reeling() || me.squat > 0 || me.duck > 0 || me.frozen > 0 || me.shocked > 0 || me.hp <= 0) return;
  me.think = THINK * (0.8 + duel.rng() * 0.4);
  const threats = [...dangers(duel, me), ...blows(duel, me, foe)];
  const options = [...threats, ...attacks(duel, me, foe), ...positions(duel, me, foe)];
  // something is about to land: raise the barrier, which answers any of it
  const barrier = me.spells.find((name) => SPELLS[name].kind === "barrier");
  const threat = Math.max(0, ...threats.map((o) => o.score));
  if (barrier && me.barrier === 0 && ready(me, barrier) && threat > BARRIER_THREAT) options.push({ spell: barrier, score: threat * 1.25 });
  let best = null;
  for (const o of options) {
    o.score *= 0.9 + duel.rng() * 0.2;
    if (!best || o.score > best.score) best = o;
  }
  me.intent = { move: 0 };
  if (best.spell !== "meditate") setMeditating(duel, me, false);
  if (best.spell === "move") me.intent = { move: best.move };
  else if (best.spell === "jump") {
    // crouch first, then spring
    if (me.air === 0) {
      me.squat = me.spec.jumpSquat;
      me.leap = me.spec.jumpSpeed;
    }
  } else if (best.spell === "guard") me.guard = best.hold;
  else if (best.spell === "duck") me.duck = best.hold;
  else if (best.spell === "fly") {
    me.fly = best.hover > 0;
    me.hover = best.hover;
    if (me.fly && me.air === 0) me.vy = me.spec.jumpSpeed * 0.6;
  }
  else if (best.spell === "meditate") setMeditating(duel, me, true);
  else if (best.spell !== "wait") begin(duel, me, best.spell, best.target);
}

const ready = (me, name) =>
  me.spells.includes(name) && !(me.cool[me.timer[name]] > 0) && me.qi >= SPELLS[name].cost && (!SPELLS[name].ultimate || me.rage >= RAGE) &&
  !(name === "blink" && me.sealed > 0) &&
  // silenced, only the blade answers
  !(me.silence > 0 && SPELLS[name].kind !== "melee");

/**
 * What fighter f's skill bar shows, slot by slot (1, 2, 3, then the ultimate): whether any spell in it can be cast
 * now, the cooldown it has `left` out of the `full` cooldown it began, and how charged it is (an ultimate charges
 * with rage; a slot with none of its spells in the fight is `empty`).
 */
export function skillBar(f) {
  return SCHOOLS[f.school].slots.map((slot, i) => {
    const owned = slot.filter((name) => f.spells.includes(name));
    const ultimate = owned.some((name) => SPELLS[name].ultimate);
    return {
      empty: owned.length === 0,
      ultimate,
      ready: f.hp > 0 && owned.some((name) => ready(f, name)),
      left: f.cool[`slot${i}`] ?? 0,
      full: f.coolFull[`slot${i}`] ?? 0,
      charge: ultimate ? f.rage / RAGE : 1,
    };
  });
}

/** What is coming at me, and the ways out of each. */
function dangers(duel, me) {
  const out = [];
  const blink = (from) => {
    if (!ready(me, "blink")) return;
    const d = SPELLS.blink.distance;
    // away from the danger, unless that is a wall
    let to = me.x + Math.sign(me.x - from || 1) * d;
    if (to < duel.left + 10 || to > duel.right - 10) to = me.x - Math.sign(me.x - from || 1) * d;
    out.push({ spell: "blink", target: to, score: 0 });
    return out[out.length - 1];
  };
  for (const s of duel.shots) {
    if (s.owner === me.index) continue;
    // a barrage still falling: its next strike aims where I stand, so keep on the move, toward the open side
    if (s.follow && s.t < 0 && s.t > -s.spell.every - s.spell.aimAhead + REACTION) {
      const way = me.x - duel.left > duel.right - me.x ? -1 : 1;
      out.push({ spell: "move", move: me.vx !== 0 ? Math.sign(me.vx) * (atWall(duel, me) ? -1 : 1) : way, score: s.spell.damage * 1.4 });
      continue;
    }
    // a sealed spell is seen from the moment its seal shows
    if (s.t + (s.lead ?? 0) < REACTION) continue;
    if (s.kind === "bolt") {
      const toward = Math.sign(me.x - s.x) === Math.sign(s.vx);
      const tti = Math.abs(me.x - s.x) / Math.abs(s.vx);
      if (!toward || tti > 0.6 || s.y < me.air - 3 || s.y > me.air + me.spec.body + 3) continue;
      const urgency = s.spell.damage * (1.2 - tti);
      if (Math.sign(s.x - me.x) === me.face) out.push({ spell: "guard", hold: tti + 0.15, score: urgency * 1.1 });
      if (me.air === 0 && !me.fly && tti < 0.24 && tti > 0.08) out.push({ spell: "jump", score: urgency * 1.25 });
      // a bolt over a ducking body's height: drop under it
      if (me.air === 0 && !me.fly && s.y > me.spec.duck + 3 && tti < 0.4) out.push({ spell: "duck", hold: tti + 0.12, score: urgency * 1.2 });
      // on the sword: rise over it, or sink under it
      if (me.qi > 12 && tti > 0.15) {
        const over = s.y + 4;
        if (over < duel.top) out.push({ spell: "fly", hover: over, score: urgency * 1.15 });
        if (me.fly && s.y > me.spec.body + 2) out.push({ spell: "fly", hover: 0, score: urgency * 1.1 });
      }
      if (ready(me, "bolt") && tti > 0.3) out.push({ spell: "bolt", target: null, score: urgency * 0.9 });
      const b = blink(s.x);
      if (b) b.score = urgency * 0.7;
    } else if (s.kind === "orb") {
      const [bx, by] = orbAt(s);
      const d = Math.hypot(bx - me.x, -by - (me.air + me.spec.chest));
      if (d > 48) continue;
      const urgency = s.spell.damage * (1.4 - d / 48);
      out.push({ spell: "move", move: Math.sign(me.x - bx) || 1, score: urgency * 1.0 });
      if (ready(me, "bolt")) out.push({ spell: "bolt", target: null, score: urgency * 1.15 });
      const b = blink(bx);
      if (b) b.score = urgency * 0.9;
    } else if (s.kind === "zone" && !s.fell && s.t + (s.lead ?? 0) >= s.spell.notice) {
      const dx = me.x - s.x;
      if (Math.abs(dx) > (s.spell.reach ?? 0) + s.spell.radius + me.spec.halfWidth + 4) continue;
      const left = s.spell.impact - s.t;
      const urgency = s.spell.damage * (1.5 - left);
      const way = Math.abs(dx) < 1 ? (s.x - duel.left > duel.right - s.x ? -1 : 1) : Math.sign(dx);
      out.push({ spell: "move", move: way, score: urgency * 1.3 });
      if (s.spell.grounded && me.qi > 12) out.push({ spell: "fly", hover: 24, score: urgency * 1.35 });
      const b = blink(s.x);
      if (b) b.score = urgency * (left < 0.3 ? 1.4 : 0.6);
    } else if (s.kind === "void") {
      const dx = me.x - s.x;
      if (Math.abs(dx) > s.spell.reach) continue;
      const urgency = 14 * (1.2 - Math.abs(dx) / s.spell.reach);
      out.push({ spell: "move", move: Math.sign(dx) || 1, score: urgency * 1.2 });
      const b = blink(s.x);
      if (b) b.score = urgency * 1.1;
    }
  }
  return out;
}

/**
 * The foe winding up a blow that reaches me (a slash or a palm): step out of reach, block, blink away, or, if my
 * own blade lands first, strike first. Seen only once the wind-up has shown for REACTION seconds.
 */
function blows(duel, me, foe) {
  const act = foe.act;
  if (!act || act.phase !== "windup" || act.t < REACTION) return [];
  const spell = SPELLS[act.spell];
  if (spell.kind !== "melee" && spell.kind !== "blast") return [];
  const d = Math.abs(foe.x - me.x);
  if (d > spell.range + me.spec.halfWidth + 6) return [];
  const left = spell.windup - act.t;
  const urgency = spell.damage * 1.2;
  const out = [];
  // backing off works if I can walk out of reach before it lands
  const out_ = spell.range + me.spec.halfWidth + 2 - d;
  if (out_ / me.spec.walk < left + 0.05) out.push({ spell: "move", move: Math.sign(me.x - foe.x) || 1, score: urgency * 1.2 });
  if (Math.sign(foe.x - me.x) === me.face) out.push({ spell: "guard", hold: left + 0.2, score: urgency * 0.9 });
  if (ready(me, "slash") && SPELLS.slash.windup < left && d < SPELLS.slash.range + foe.spec.halfWidth) {
    out.push({ spell: "slash", score: urgency * 1.1 });
  }
  if (ready(me, "blink")) {
    const to = me.x + Math.sign(me.x - foe.x || 1) * SPELLS.blink.distance;
    if (to > duel.left + 10 && to < duel.right - 10) out.push({ spell: "blink", target: to, score: urgency * 0.8 });
  }
  return out;
}

/** Each attack I could start now, scored by the damage I expect from it for what it costs me. */
function attacks(duel, me, foe) {
  const school = SCHOOLS[me.school];
  const d = Math.abs(foe.x - me.x);
  // how long the foe cannot answer: it is gathering a spell, recovering from one, or staggered
  const busy = foe.body.outFor() + (foe.act ? SPELLS[foe.act.spell][foe.act.phase] - foe.act.t + (foe.act.phase === "windup" ? SPELLS[foe.act.spell].recover : 0) : 0);
  // a foe on the floor cannot be touched: a blow is only worth starting if it lands once the foe is up (a meaty)
  const open = foe.body.openIn();
  const free = !foe.body.reeling() && !foe.act && foe.frozen === 0 && foe.shocked === 0;
  // an opening to punish: the foe down, staggered, recovering from a move, or held in a long cast; a blow it is
  // winding up is no opening (striking into it only trades)
  const fact = foe.act ? SPELLS[foe.act.spell] : null;
  const striking = fact && (fact.kind === "melee" || fact.kind === "blast" || fact.kind === "dash");
  const opening =
    foe.body.outFor() + foe.frozen + foe.shocked +
    (!fact ? 0 : foe.act.phase === "recover" ? fact.recover - foe.act.t : striking ? 0 : fact.windup - foe.act.t + fact.recover);
  // ahead on life, a miss costs more than it gains; behind, less
  const lead = clamp((me.hp - foe.hp) / 40, -1, 1);
  const out = [];
  const value = (name, chance, lands = SPELLS[name].windup) => {
    const s = SPELLS[name];
    if (lands < open) return 0;
    // in a breath after its last attack, a fighter only punishes: the foe must still be open when this lands
    if (me.calm > 0 && opening < lands) return 0;
    // a raised barrier turns everything but an ultimate aside: wait it out
    if (foe.barrier > 0 && !s.ultimate) chance *= 0.05;
    // what it has seen this attack do against this foe
    const tried = me.read.tried[name] ?? 0;
    chance = (chance * PRIOR + (me.read.landed[name] ?? 0)) / (PRIOR + tried);
    const gain = (s.damage ?? 0) * chance * school.aggression;
    const risk = free ? MIND.risk * punisher(foe, me, s.recover) * (1 - chance) * (1 + MIND.lead * lead) : 0;
    return Math.max(gain * MIND.floor, gain - risk) / (0.5 + s.windup + s.cost / 40);
  };
  if (ready(me, "dive") && me.air - foe.air > 14 && d < 44) {
    out.push({ spell: "dive", score: value("dive", busy > SPELLS.dive.windup ? 0.9 : foe.guard > 0 ? 0.25 : 0.6) });
  }
  if (ready(me, "slash") && d < SPELLS.slash.range + 8 && Math.abs(foe.air - me.air) < me.spec.chest) {
    out.push({ spell: "slash", score: value("slash", busy > SPELLS.slash.windup ? 0.95 : foe.guard > 0 ? 0.15 : 0.55) });
  }
  // a bolt flies level from my chest: it must cross the foe's body
  const level = me.air + me.spec.chest > foe.air - 2 && me.air + me.spec.chest < foe.air + foe.spec.body;
  if (ready(me, "bolt") && d > 24 && level) {
    const travel = d / SPELLS.bolt.speed + SPELLS.bolt.windup;
    // a bolt into a bolt the foe is gathering only cancels it
    const trade = foe.act?.spell === "bolt" && foe.act.phase === "windup";
    const chance = trade ? 0.1 : busy > travel ? 0.9 : foe.guard > 0 ? 0.2 : 0.45;
    out.push({ spell: "bolt", score: value("bolt", chance, travel) });
  }
  if (ready(me, "orb") && d > 40 && !duel.shots.some((s) => s.kind === "orb" && s.owner === me.index)) {
    // a slow ball that hunts the foe down: from far off it holds the space between and seldom misses in the end
    out.push({ spell: "orb", score: value("orb", d > 80 ? 0.8 : 0.5) });
  }
  for (const name of ["tribulation", "prison"]) {
    if (!ready(me, name)) continue;
    const lead = SPELLS[name].windup + SPELLS[name].impact;
    const pace = foe.act || foe.body.reeling() || foe.medit ? 0 : foe.intent.move * foe.spec.walk;
    if (lead < open) continue;
    out.push({ spell: name, target: foe.x + pace * lead * 0.8, score: value(name, 0.8, lead) * (SPELLS[name].strikes ?? 1) + 3 });
  }
  for (const name of me.spells) {
    if (SPELLS[name].kind !== "zone" || SPELLS[name].ultimate || !ready(me, name)) continue;
    // aim where the foe will be when it lands
    const lead = SPELLS[name].windup + (SPELLS[name].sigil ?? 0) + SPELLS[name].impact;
    const pace = foe.act || foe.body.reeling() || foe.medit ? 0 : foe.intent.move * foe.spec.walk;
    const target = foe.x + pace * lead * 0.8;
    // an icicle only finds a foe standing on the floor
    const grounded = SPELLS[name].grounded ? (foe.air < 10 && !foe.fly ? 1 : 0.1) : 1;
    out.push({ spell: name, target, score: value(name, (busy > lead * 0.6 ? 0.85 : SPELLS[name].pull ? 0.5 : 0.4) * grounded, lead) });
  }
  for (const name of me.spells) {
    const s = SPELLS[name];
    if (s.kind === "blast" && ready(me, name) && d < s.range + 4) {
      out.push({ spell: name, score: value(name, busy > s.windup ? 0.9 : foe.guard > 0 ? 0.2 : 0.55) });
    }
    // a fire or a vortex under the foe: worth it on a foe that cannot leave it soon
    if (s.kind === "void" && ready(me, name) && (busy > 0.4 || foe.frozen > 0 || foe.hp < 75)) {
      // it lands its blow when it ends, burning on the way
      out.push({ spell: name, score: value(name, 0.7, s.windup + s.life) + 4 });
    }
    // a lightning step: the foe within a stride, on my level
    if (s.kind === "dash" && ready(me, name) && d > 12 && d < s.distance - 10 && Math.abs(foe.air - me.air) < me.spec.chest) {
      out.push({ spell: name, score: value(name, busy > s.windup ? 0.9 : foe.guard > 0 ? 0.2 : 0.6) });
    }
  }
  if (ready(me, "ward") && me.shield === 0 && me.hp < 90) {
    const safe = d > 50 && !foe.act ? 1 : 0.6;
    out.push({ spell: "ward", score: (8 + 14 * (1 - me.hp / 100)) * safe });
  }
  if (ready(me, "heal") && me.hp < 50) {
    const safe = d > 60 && !foe.act ? 1 : busy > 1 ? 0.8 : 0.2;
    out.push({ spell: "heal", score: SPELLS.heal.amount * safe * (1 - me.hp / 100) });
  }
  return out;
}

/**
 * The hardest blow `foe` could land on `me` within `time` seconds (my recovery after a miss): an attack it has ready
 * that reaches me from where it stands and lands in time.
 */
function punisher(foe, me, time) {
  const d = Math.abs(foe.x - me.x);
  const level = Math.abs(foe.air - me.air);
  let most = 0;
  for (const name of foe.spells) {
    const s = SPELLS[name];
    if (!s.damage || s.ultimate || !ready(foe, name)) continue;
    let lands = Infinity;
    if (s.kind === "melee" && !s.dive && d < s.range + me.spec.halfWidth && level < me.spec.chest) lands = s.windup;
    else if (s.kind === "blast" && d < s.range + me.spec.halfWidth && level < me.spec.body) lands = s.windup;
    else if (s.kind === "dash" && d < s.distance && level < me.spec.chest) lands = s.windup;
    else if (s.kind === "bolt" && level < me.spec.body) lands = s.windup + d / s.speed;
    if (lands <= time + REACTION) most = Math.max(most, s.damage);
  }
  return most;
}

/** Moving to the distance I fight best at, meditating when it is quiet, or waiting. */
function positions(duel, me, foe) {
  const d = foe.x - me.x;
  // the distance a fighter keeps comes from what it can use: its school's range with spells that fly or fall,
  // arm's length with a palm blast, sword's length with nothing else
  const has = (...kinds) => me.spells.some((name) => kinds.includes(SPELLS[name].kind));
  // and with too little qi to cast, its school's `dry` distance
  const school = SCHOOLS[me.school];
  const casting = me.qi >= 25;
  const range = !casting ? school.dry : has("bolt", "orb", "zone", "void") ? school.range : has("blast", "dash") ? 34 : SPELLS.slash.range - 4;
  const off = Math.abs(d) - range;
  const out = [{ spell: "wait", score: 0.4 }];
  // after an exchange, or with the foe behind a barrier, back off to spacing and let it come (footsies): out of its
  // sword's reach at least, and most of the way back to its own range
  const reach = Math.max(SPELLS.slash.range + me.spec.halfWidth + 6, range * 0.8);
  if ((me.calm > 0 || foe.barrier > 0) && Math.abs(d) < reach && Math.abs(foe.air - me.air) < me.spec.chest && !foe.body.reeling()) {
    const back = -Math.sign(d) || 1;
    const room = back < 0 ? me.x - duel.left : duel.right - me.x;
    if (room > 14) out.push({ spell: "move", move: back, score: 2.4 });
  }
  // inside the foe's sword reach with no blade of my own ready: get out of it
  if (Math.abs(d) < SPELLS.slash.range + 4 && me.air - foe.air < me.spec.chest && !ready(me, "slash")) {
    const back = -Math.sign(d) || 1;
    const room = back < 0 ? me.x - duel.left : duel.right - me.x;
    if (room > 14) out.push({ spell: "move", move: back, score: 2.2 });
  }
  if (Math.abs(off) > 8) {
    let move = Math.sign(d) * Math.sign(off);
    // backed against a wall: step out sideways past the foe's reach instead
    if ((move < 0 && me.x - duel.left < 12) || (move > 0 && duel.right - me.x < 12)) move = 0;
    if (move !== 0) out.push({ spell: "move", move, score: 0.6 + Math.min(Math.abs(off), 60) / 40 });
  }
  // cornered with the foe inside my range: blink past it and take the open side
  const wall = Math.min(me.x - duel.left, duel.right - me.x);
  if (wall < 16 && off < -10 && ready(me, "blink")) {
    const behind = foe.x + Math.sign(foe.x - me.x) * 30;
    if (behind > duel.left + 10 && behind < duel.right - 10) out.push({ spell: "blink", target: behind, score: 2.5 });
  }
  // the height I want: my school's while I have the qi, the foe's when I must close in with a blade
  let want = me.qi > 40 && has("bolt", "orb", "zone", "void") ? SCHOOLS[me.school].sky : 0;
  if (!casting || Math.abs(d) < 40) want = foe.air > 16 && me.qi > 20 ? Math.min(duel.top, foe.air) : want;
  const flying = me.fly ? me.hover : 0;
  if (Math.abs(flying - want) > 8) out.push({ spell: "fly", hover: want, score: 1.2 });
  // light on its feet: on the ground and out of reach, it bounds about rather than walking flat
  if (me.air === 0 && !me.fly && Math.abs(d) > SPELLS.slash.range + 10 && duel.rng() < 0.5) out.push({ spell: "jump", score: 1.3 });
  const threatened = duel.shots.some((s) => s.owner !== me.index);
  if (me.qi < 45 && Math.abs(d) > 70 && !threatened && !foe.act && me.air === 0 && !me.fly) out.push({ spell: "meditate", score: 2.2 - me.qi / 40 });
  return out;
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
