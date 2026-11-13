// The hero's duel: two cultivators fight on the ground and in the air, with spells that are pixfx effects. This
// module is the game only: rules, spells, and each fighter's mind. It draws nothing and never touches the engine.
// `step` returns what the engine should show (play, move or stop an effect) and what just happened that the screen
// should feel (`hit`, `clash`, `land`, `bump`, `ko`, `ultimate`, each with where and how hard); landing.js plays the effects and
// gives the impacts their weight (hit-stop, shake, numbers).
//
// Units are game pixels and seconds. x grows to the right; heights (`air`, effect y) are measured up from the ground.

/** Effects the arena itself shows when they are in the registry. */
export const DECOR = { lamp: "fire/brazier", night: "weather/fireflies" };

// Where things are on a fighter, in pixels above its feet.
const BODY = 34;
const CHEST = 20;
const HALF_WIDTH = 6;

const WALK = 52;
const JUMP_SPEED = 200;
const GRAVITY = 520;
/** Near the top of an arc gravity eases, so a jump hangs a moment before it falls (the apex hang). */
const APEX = { speed: 45, gravity: 0.45 };
/** Flying: how fast a fighter climbs or sinks toward the height it wants, the highest it goes, and its qi cost. */
const FLY = { rate: 7, max: 140, top: 64, cost: 5 };
/** A blow this strong or stronger throws the target up into the air. */
const LAUNCH = { damage: 12, speed: 130, perDamage: 5 };
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
 * harder, shields itself, and every spell it casts charges its rage (attuned).
 */
export const STATS = {
  thunder: { hp: 2500, atk: 330, pdef: 130, mdef: 110, crit: 0.18, critDamage: 1.6, dodge: 0.05, fury: true, unyielding: true, attuned: false },
  frost: { hp: 2450, atk: 370, pdef: 105, mdef: 165, crit: 0.15, critDamage: 1.7, dodge: 0.11, fury: false, unyielding: false, attuned: true },
};

/** Knocked into the arena's edge faster than this, a fighter bounces off it. */
const BOUNCE = 80;
/** Falling faster than this when it lands, a fighter's landing is felt. */
const HARD_LANDING = 140;
const QI_REGEN = 7;
const MEDITATE_REGEN = 24;
/** How long a fighter takes to notice something new, and how often it thinks. */
const REACTION = 0.16;
const THINK = 0.12;
const STAGGER = 0.38;
/** Qi a block costs. */
const GUARD_COST = 8;
/** The share of a blocked blow's damage that still lands. */
const CHIP = 0.15;
const KNOCKBACK = 70;
/** How close two bodies can stand: the space each one takes up, and a pixel between. */
const PERSONAL = 2 * HALF_WIDTH + 2;
const ROUND_PAUSE = 3.2;

const CAST_CIRCLE = "arcane/rune-circle";
const CLASH = "impacts/parry";

/**
 * Each spell: what it costs, how long the fighter is committed before (`windup`) and after (`recover`) it lands,
 * its cooldown, and its own numbers. `fx` is the effect that shows it; a spell whose effect is not in the registry
 * is left out of the fight (slash and block need none).
 */
export const SPELLS = {
  slash: { kind: "melee", type: "physical", cost: 0, windup: 0.2, recover: 0.32, cooldown: 0.7, range: 20, damage: 6 },
  // Thunder: a flash of lightning that carries the body through the foe and out the other side
  step: { kind: "dash", type: "physical", fx: "spawn/poof", cost: 16, windup: 0.26, recover: 0.32, cooldown: 3.4, distance: 72, damage: 11 },
  // Thunder: a gold seal opens on the ground under the foe, and a thunderclap bursts from it
  seal: {
    kind: "zone", type: "magic", fx: "explosions/plasma-explosion", cost: 20, windup: 0.3, recover: 0.3, cooldown: 2.4,
    sigil: 0.4, impact: 0.08, radius: 16, damage: 14, notice: 0, height: CHEST, shock: true,
  },
  // Frost: a lotus of ice shards blooms from the palm at close range, and cracks the foe's guard
  lotus: { kind: "blast", type: "magic", fx: "holy/feather-burst", cost: 16, windup: 0.3, recover: 0.3, cooldown: 2.2, range: 34, damage: 13, defDown: 3 },
  // Frost: a pale fire that does not warm, lit under the foe; it burns whoever stands in it
  frostfire: {
    kind: "void", type: "magic", fx: "fire/spirit-flame", cost: 22, windup: 0.4, recover: 0.3, cooldown: 5,
    life: 2.6, height: 0, pull: 0, reach: 18, under: 18, tick: 5, damage: 6, chill: true,
  },
  // Frost: a blue seal under the foe that shatters into a burst of cold, freezing it
  shatter: {
    kind: "zone", type: "magic", fx: "explosions/plasma-explosion", cost: 22, windup: 0.35, recover: 0.3, cooldown: 3,
    sigil: 0.35, impact: 0.08, radius: 16, damage: 18, notice: 0, height: CHEST, freeze: 0.6,
  },
  // from above: drop onto the foe sword first, and crack the ground if it lands there
  dive: { kind: "melee", type: "physical", dive: true, cost: 8, windup: 0.22, recover: 0.4, cooldown: 1.6, range: 22, damage: 13, speed: 260, slamFx: "explosions/dirt-explosion" },
  bolt: { kind: "bolt", type: "magic", fx: "projectiles/laser-bolt", cost: 12, windup: 0.28, recover: 0.22, cooldown: 1.3, speed: 230, damage: 9, shock: true },
  orb: {
    kind: "orb", type: "magic", fx: "lightning/ball-lightning", hitFx: "explosions/plasma-explosion", burn: true,
    cost: 30, windup: 0.6, recover: 0.3, cooldown: 4.5, speed: 26, life: 5.5, damage: 10, radius: 10,
  },
  // icicle-fall drops its icicle at FREE + fall time: 0.64 + sqrt(2 * (80 - 36) / 950) s, and only starts to tremble,
  // which is when a fighter notices it overhead, at 0.3 s (pixelfx ice/icicle-fall.js TREMBLE)
  icicle: { kind: "zone", type: "magic", defDown: 4, fx: "ice/icicle-fall", cost: 22, windup: 0.42, recover: 0.28, cooldown: 2.2, impact: 0.944, radius: 16, damage: 20, notice: 0.3, height: 0, grounded: true, freeze: FREEZE },
  // implosion sucks in until 0.8 s and pops at 1.02 s (pixelfx explosions/implosion.js SUCK, STILL)
  collapse: {
    kind: "zone", type: "magic", silence: 1.5, fx: "explosions/implosion", cost: 26, windup: 0.5, recover: 0.3, cooldown: 2.4,
    impact: 1.02, radius: 18, damage: 18, notice: 0.15, height: CHEST, pull: 60, reach: 52, pullUntil: 0.8,
  },
  // a thunderclap from the open palm, at arm's length
  palm: { kind: "blast", type: "physical", stealRage: 1, fx: "explosions/plasma-explosion", cost: 18, windup: 0.32, recover: 0.35, cooldown: 2.8, range: 30, damage: 14, shock: true },
  // ultimates, cast with full rage. Thunder: three thunderclaps fall one after another where the foe will be.
  tribulation: {
    kind: "zone", type: "magic", fx: "explosions/plasma-explosion", ultimate: true, cost: 0, windup: 0.7, recover: 0.5, cooldown: 0,
    impact: 0.08, radius: 12, damage: 10, notice: 0, height: CHEST, strikes: 3, every: 0.7, aimAhead: 0.5,
  },
  // Frost: a great collapse that drags the foe in from far off and freezes it where it pops.
  prison: {
    kind: "zone", type: "magic", silence: 2, fx: "explosions/implosion", ultimate: true, cost: 0, windup: 0.7, recover: 0.5, cooldown: 0,
    impact: 1.02, radius: 26, damage: 26, notice: 0.1, height: CHEST, pull: 70, reach: 110, pullUntil: 0.9, seals: true, freeze: FREEZE * 1.4,
  },
  // void-orb hangs 62 px over its origin (pixelfx shadow/void-orb.js Y) and loops every 3.2 s
  void: {
    kind: "void", type: "magic", fx: "shadow/void-orb", endFx: "explosions/implosion",
    cost: 60, windup: 0.9, recover: 0.45, cooldown: 11, life: 3.2, height: 62, pull: 34, reach: 90, under: 22, tick: 4, damage: 10,
  },
  heal: { kind: "heal", fx: "holy/feather-burst", cost: 40, windup: 0.85, recover: 0.3, cooldown: 9, amount: 20 },
  // a shell of frost qi that takes blows for the caster until it breaks
  ward: { kind: "ward", fx: "holy/feather-burst", cost: 30, windup: 0.5, recover: 0.25, cooldown: 8 },
  blink: { kind: "blink", fx: "spawn/poof", cost: 20, windup: 0.06, recover: 0.12, cooldown: 1.8, distance: 56 },
};

/**
 * The two schools: their spells, the distance each likes to fight at, how eager it is, the height it likes to hover
 * at while it has the qi (`sky`; Frost casts from the air, Thunder fights on the ground), and its colours.
 */
export const SCHOOLS = {
  thunder: {
    name: "Thunder", spells: ["slash", "dive", "palm", "step", "seal", "bolt", "orb", "blink", "heal", "tribulation"], range: 52, aggression: 1.15, sky: 0,
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
    name: "Frost", spells: ["slash", "dive", "lotus", "collapse", "shatter", "frostfire", "bolt", "icicle", "void", "blink", "ward", "prison"], range: 84, aggression: 1.1, sky: 26,
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
 * effect is in the registry.
 */
export function createDuel({ seed = 1, left, right, ground, has }) {
  const duel = { seed, rng: random(seed), left, right, ground, has, time: 0, round: 0, wins: [0, 0], key: 0, shots: [], pause: 0, out: [] };
  const usable = (name) => {
    const fx = SPELLS[name].fx;
    return !fx || has(fx);
  };
  duel.fighters = [
    fighter(0, "thunder", SCHOOLS.thunder.spells.filter(usable)),
    fighter(1, "frost", SCHOOLS.frost.spells.filter(usable)),
  ];
  for (const name of Object.values(DECOR)) if (has(name)) duel.decor = true;
  startRound(duel);
  return duel;
}

function fighter(index, school, spells) {
  return { index, school, spells, stats: STATS[school], x: 0, air: 0, vy: 0, vx: 0, face: 1, hp: 100, qi: 100, act: null, stun: 0, guard: 0, medit: false, fly: false, hover: 0, land: 0, rage: 0, frozen: 0, shocked: 0, sealed: 0, shield: 0, shieldT: 0, burn: null, defDown: 0, silence: 0, fury: 0, rose: false, dodged: 0, chilled: 0, cool: {}, think: 0, intent: { move: 0 }, aura: null, pose: "idle", poseT: 0, flash: 0 };
}

function startRound(duel) {
  const span = duel.right - duel.left;
  duel.round++;
  duel.pause = 0;
  duel.winner = null;
  for (const s of duel.shots) stopShot(duel, s);
  duel.shots = [];
  for (const f of duel.fighters) {
    Object.assign(f, { air: 0, vy: 0, vx: 0, hp: 100, qi: 100, act: null, stun: 0, guard: 0, medit: false, fly: false, hover: 0, land: 0, rage: 0, frozen: 0, shocked: 0, sealed: 0, shield: 0, shieldT: 0, burn: null, defDown: 0, silence: 0, fury: 0, rose: false, dodged: 0, chilled: 0, cool: {}, think: 0.4 + duel.rng() * 0.3, intent: { move: 0 }, flash: 0 });
    f.x = duel.left + span * (f.index === 0 ? 0.22 : 0.78);
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
  separate(duel, a, b);
  for (const s of [...duel.shots]) shot(duel, s, dt);
  for (const f of duel.fighters) if (f.hp <= 0 && duel.winner === null) knockOut(duel, f);
}

/**
 * Two bodies cannot stand in one place: when they overlap at the same height each is pushed half the overlap
 * apart, and neither keeps walking into the other. One far enough above the other passes over it.
 */
function separate(duel, a, b) {
  if (a.hp <= 0 || b.hp <= 0 || Math.abs(a.air - b.air) >= BODY - 6) return;
  const dx = b.x - a.x;
  const overlap = PERSONAL - Math.abs(dx);
  if (overlap <= 0) return;
  // exactly on top of each other: part them along the way they face
  const dir = dx !== 0 ? Math.sign(dx) : a.face;
  const lo = duel.left + HALF_WIDTH;
  const hi = duel.right - HALF_WIDTH;
  a.x = clamp(a.x - (dir * overlap) / 2, lo, hi);
  b.x = clamp(b.x + (dir * overlap) / 2, lo, hi);
  // against a wall one side cannot give: the other takes the rest
  const left = PERSONAL - Math.abs(b.x - a.x);
  if (left > 0) {
    if (a.x === lo || a.x === hi) b.x = clamp(b.x + dir * left, lo, hi);
    else a.x = clamp(a.x - dir * left, lo, hi);
  }
  if (Math.sign(a.vx) === dir) a.vx = 0;
  if (Math.sign(b.vx) === -dir) b.vx = 0;
}

// ---------- bodies: moving, casting, getting hit ----------

function body(duel, me, foe, dt) {
  me.flash = Math.max(0, me.flash - dt);
  me.poseT += dt;
  me.stun = Math.max(0, me.stun - dt);
  me.frozen = Math.max(0, me.frozen - dt);
  me.shocked = Math.max(0, me.shocked - dt);
  me.sealed = Math.max(0, me.sealed - dt);
  me.defDown = Math.max(0, me.defDown - dt);
  me.silence = Math.max(0, me.silence - dt);
  me.dodged = Math.max(0, me.dodged - dt);
  me.chilled = Math.max(0, me.chilled - dt);
  me.shieldT = Math.max(0, me.shieldT - dt);
  if (me.shieldT === 0) me.shield = 0;
  burning(duel, me, dt);
  me.guard = Math.max(0, me.guard - dt);
  for (const k of Object.keys(me.cool)) me.cool[k] = Math.max(0, me.cool[k] - dt);
  me.qi = Math.min(100, me.qi + (me.medit ? MEDITATE_REGEN : me.fly ? -FLY.cost : QI_REGEN) * dt);
  me.land = Math.max(0, me.land - dt);
  if (!me.act && me.stun === 0) me.face = foe.x >= me.x ? 1 : -1;
  // out of qi, or knocked out of the sky: flight ends and it falls
  if (me.fly && (me.qi <= 1 || me.stun > 0)) me.fly = false;

  // feet: walking or flying across (only when free), knockback, then up and down
  const free = !me.act && me.stun === 0 && !me.medit && me.frozen === 0 && me.shocked === 0;
  const walk = free && me.guard === 0 ? me.intent.move * WALK * (me.chilled > 0 ? 0.45 : 1) : 0;
  me.vx *= Math.exp(-6 * dt);
  const x = me.x + (walk + me.vx) * dt;
  const lo = duel.left + HALF_WIDTH, hi = duel.right - HALF_WIDTH;
  if ((x < lo || x > hi) && Math.abs(me.vx) > BOUNCE) {
    // thrown into the edge of the arena: bounce off it
    me.vx = -me.vx * 0.5;
    emit(duel, { type: "bump", x: x < lo ? lo : hi, y: me.air + CHEST, strength: Math.min(1, Math.abs(me.vx) / 120) });
  }
  me.x = clamp(x, lo, hi);
  const diving = me.act?.phase === "windup" && SPELLS[me.act.spell].dive;
  if (diving) {
    // a dive drops straight at the foe's height
    me.air = Math.max(foe.air, me.air - SPELLS.dive.speed * dt);
    me.vy = -SPELLS.dive.speed;
  } else if (me.fly) {
    me.vy = clamp((me.hover - me.air) * FLY.rate, -FLY.max, FLY.max);
    me.air = clamp(me.air + me.vy * dt, 0, FLY.top);
  } else if (me.air > 0 || me.vy > 0) {
    const g = Math.abs(me.vy) < APEX.speed ? GRAVITY * APEX.gravity : GRAVITY;
    const falling = -me.vy;
    me.vy -= g * dt;
    me.air = Math.max(0, me.air + me.vy * dt);
    if (me.air === 0) {
      if (falling > HARD_LANDING) {
        me.land = 0.14;
        emit(duel, { type: "land", x: me.x, strength: Math.min(1, falling / 300) });
      }
      me.vy = 0;
    }
  }

  if (me.act) {
    me.act.t += dt;
    const spell = SPELLS[me.act.spell];
    if (me.act.phase === "windup" && me.act.t >= spell.windup) {
      release(duel, me, foe, spell);
      me.act = { ...me.act, phase: "recover", t: 0 };
      if (me.act.circle !== undefined) emit(duel, { type: "stop", key: me.act.circle });
    } else if (me.act.phase === "recover" && me.act.t >= spell.recover) {
      me.act = null;
    }
  }
  me.pose = poseOf(me);
}

function poseOf(f) {
  if (f.hp <= 0) return f.air > 2 ? "tumble" : "down";
  if (f.frozen > 0) return "frozen";
  if (f.shocked > 0) return "shocked";
  if (f.stun > 0) return f.air > 4 ? "tumble" : "hurt";
  if (f.act) {
    const k = SPELLS[f.act.spell].kind;
    if (k === "melee") return f.act.phase === "windup" ? "draw" : "slash";
    return f.act.phase === "windup" ? "channel" : "cast";
  }
  if (f.guard > 0) return "guard";
  if (f.medit) return "meditate";
  if (f.land > 0) return "land";
  if (f.fly) return "fly";
  if (f.air > 0) return "jump";
  if (f.intent.move !== 0) return "walk";
  return "idle";
}

function begin(duel, me, name, target) {
  const spell = SPELLS[name];
  me.qi -= spell.cost;
  if (spell.ultimate) {
    me.rage = 0;
    emit(duel, { type: "ultimate", by: me.index, x: me.x, y: me.air + CHEST });
  }
  me.cool[name] = spell.cooldown + spell.windup;
  me.act = { spell: name, phase: "windup", t: 0, target };
  setMeditating(duel, me, false);
  // longer casts stand in a rune circle while they gather
  if (spell.kind !== "melee" && spell.kind !== "blink" && spell.kind !== "dash" && duel.has(CAST_CIRCLE)) {
    me.act.circle = play(duel, CAST_CIRCLE, me.x, duel.ground, { palette: SCHOOLS[me.school].circle, seed: me.index + 1 });
  }
}

/** The moment a spell lands: it hits, flies, falls or heals. */
function release(duel, me, foe, spell) {
  const g = duel.ground;
  if (me.stats.attuned && spell.kind !== "melee" && !spell.ultimate && spell.cost > 0) gainRage(me, 1);
  if (spell.kind === "melee") {
    me.vx += me.face * 50;
    if (spell.dive) {
      me.fly = false;
      // a dive that ends on the ground cracks it
      if (me.air < 2 && spell.slamFx && duel.has(spell.slamFx)) {
        cast(duel, me, spell.slamFx, me.x + me.face * 6, g, { size: 0.6 });
        emit(duel, { type: "land", x: me.x, strength: 1 });
      }
    }
    const dx = Math.abs(foe.x - me.x);
    if (dx <= spell.range + HALF_WIDTH && Math.abs(foe.air - me.air) < CHEST + 4 && foe.hp > 0) {
      if (blocks(foe, me.x, spell.damage)) clash(duel, (me.x + foe.x) / 2, g - CHEST - me.air, [me, foe]);
      else if (foe.act?.spell === "slash" && foe.act.phase === "windup" && foe.act.t > SPELLS.slash.windup * 0.6) {
        // both blades swing at once: they meet
        clash(duel, (me.x + foe.x) / 2, g - CHEST - me.air, [me, foe]);
        foe.act = null;
      } else hurt(duel, foe, spell.damage, me.face, me.act.spell);
    }
  } else if (spell.kind === "bolt") {
    const y = me.air + CHEST;
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
    streak(duel, me, x, air + (spell.height ?? 0));
  } else if (spell.kind === "blast") {
    const x = me.x + me.face * spell.range * 0.7;
    cast(duel, me, spell.fx, x, g - CHEST - me.air);
    if (Math.abs(foe.x - me.x) <= spell.range + HALF_WIDTH && Math.abs(foe.air - me.air) < BODY && foe.hp > 0) {
      if (blocks(foe, me.x, spell.damage)) clash(duel, foe.x - me.face * 4, g - CHEST - foe.air, [foe]);
      else {
        hurt(duel, foe, spell.damage, me.face, me.act.spell);
        if (spell.shock) shock(duel, foe, SHOCK.chance);
      }
    }
  } else if (spell.kind === "void") {
    const x = clamp(foe.x, duel.left + 10, duel.right - 10);
    const key = cast(duel, me, spell.fx, x, g, { seed: me.index + 5 });
    duel.shots.push({ kind: "void", name: me.act.spell, owner: me.index, x, t: 0, tick: 0, key, spell });
    streak(duel, me, x, spell.height ? spell.height : 4);
  } else if (spell.kind === "dash") {
    // gone in a flash and back a stride past the foe, cutting through it on the way
    const from = me.x;
    const to = clamp(me.x + me.face * spell.distance, duel.left + HALF_WIDTH, duel.right - HALF_WIDTH);
    cast(duel, me, spell.fx, from, g - me.air);
    me.x = to;
    cast(duel, me, spell.fx, to, g - me.air);
    emit(duel, { type: "streak", by: me.index, x0: from, y0: me.air + CHEST, x1: to, y1: me.air + CHEST });
    const lo = Math.min(from, to) - HALF_WIDTH;
    const hi = Math.max(from, to) + HALF_WIDTH;
    if (foe.x > lo && foe.x < hi && Math.abs(foe.air - me.air) < BODY && foe.hp > 0) {
      if (blocks(foe, from, spell.damage)) clash(duel, foe.x, g - CHEST - foe.air, [foe]);
      else hurt(duel, foe, spell.damage, me.face, me.act.spell);
    }
  } else if (spell.kind === "heal") {
    const gained = Math.min(100 - me.hp, spell.amount);
    me.hp += gained;
    emit(duel, { type: "heal", by: me.index, amount: Math.round((gained / 100) * me.stats.hp), x: me.x, y: me.air + CHEST });
    cast(duel, me, spell.fx, me.x, g - BODY - me.air);
  } else if (spell.kind === "ward") {
    me.shield = Math.round(WARD.share * me.stats.hp);
    me.shieldT = WARD.time;
    emit(duel, { type: "ward", by: me.index, amount: me.shield, x: me.x, y: me.air + CHEST });
    cast(duel, me, spell.fx, me.x, g - BODY - me.air);
  } else if (spell.kind === "blink") {
    cast(duel, me, spell.fx, me.x, g - me.air);
    me.x = clamp(me.act.target, duel.left + HALF_WIDTH, duel.right - HALF_WIDTH);
    cast(duel, me, spell.fx, me.x, g - me.air);
  }
}

/**
 * Does f block a blow of `damage` from fromX? A block costs qi and still lets a little damage through (chip), so
 * no fight stalls behind a guard; with too little qi left the guard breaks and the blow lands.
 */
function blocks(f, fromX, damage = 0) {
  if (f.guard === 0 || f.stun > 0 || Math.sign(fromX - f.x) !== f.face) return false;
  if (f.qi < GUARD_COST) {
    f.guard = 0;
    return false;
  }
  f.qi -= GUARD_COST;
  f.hp = Math.max(0, f.hp - damage * CHIP);
  return true;
}

/**
 * f takes a blow of `power` from `spell`, pushed toward `dir`; the step reports it as a `hit` command (or a
 * `dodge`). Returns whether it landed.
 */
function hurt(duel, f, power, dir, spell) {
  if (f.hp <= 0) return false;
  const by = duel.fighters[1 - f.index];
  const s = SPELLS[spell];
  // an ultimate cannot be dodged
  if (!s?.ultimate && duel.rng() < f.stats.dodge) {
    f.dodged = 0.3;
    f.vx -= dir * 40;
    emit(duel, { type: "dodge", x: f.x, y: f.air + CHEST });
    return false;
  }
  const crit = duel.rng() < by.stats.crit;
  const amount = damageOf(by, f, power, s?.type ?? "magic", crit, duel.rng());
  const share = take(duel, f, amount);
  emit(duel, { type: "hit", by: by.index, spell, damage: amount, crit, share, x: f.x, y: f.air + CHEST });
  // rage for the blow: an attuned fighter's spells were paid for when cast, so only its blade earns here
  if (!s?.ultimate && !(by.stats.attuned && s?.kind !== "melee")) gainRage(by, 1);
  if (by.stats.fury) by.fury = Math.min(FURY.max, by.fury + 1);
  f.flash = 0.18;
  f.stun = STAGGER;
  f.vx += dir * KNOCKBACK * (power >= LAUNCH.damage ? 1.6 : 1);
  f.fly = false;
  // a heavy blow throws it up, where it can be hit again before it lands
  if (power >= LAUNCH.damage) f.vy = Math.max(f.vy, LAUNCH.speed + power * LAUNCH.perDamage);
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
    emit(duel, { type: "unyielding", by: f.index, x: f.x, y: f.air + CHEST });
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
    emit(duel, { type: "tick", by: f.burn.by, damage: f.burn.per, share, x: f.x, y: f.air + CHEST });
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
  emit(duel, { type: "ko", x: f.x, y: f.air + CHEST });
  f.fly = false;
  other.fly = false;
  duel.winner = other.index;
  duel.wins[other.index]++;
  f.act = null;
  setMeditating(duel, f, false);
  other.act = null;
  setMeditating(duel, other, false);
}

function settle(duel, f, dt) {
  f.poseT += dt;
  f.vx *= Math.exp(-6 * dt);
  f.x = clamp(f.x + f.vx * dt, duel.left + HALF_WIDTH, duel.right - HALF_WIDTH);
  if (f.air > 0 || f.vy > 0) {
    f.vy -= GRAVITY * dt;
    f.air = Math.max(0, f.air + f.vy * dt);
  }
  f.pose = f.index === duel.winner ? "win" : "down";
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
    if (overlaps(foe, s.x, s.y, 3)) {
      if (blocks(foe, s.x - s.vx, spell.damage)) clash(duel, s.x, g - s.y, []);
      else {
        hurt(duel, foe, spell.damage, Math.sign(s.vx), "bolt");
        shock(duel, foe, SHOCK.chance);
      }
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
      cast(duel, duel.fighters[s.owner], spell.fx, s.x, g - spell.height - s.air);
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
      const reaches = Math.abs(foe.x - s.x) < spell.radius + HALF_WIDTH && (spell.grounded ? foe.air < 10 : Math.abs(foe.air - s.air) < 22);
      if (reaches) {
        hurt(duel, foe, spell.damage, Math.sign(foe.x - s.x) || 1, s.name);
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
        emit(duel, { type: "tick", by: by.index, damage: amount, share, x: foe.x, y: foe.air + CHEST });
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
  return f.x <= duel.left + HALF_WIDTH + 2 || f.x >= duel.right - HALF_WIDTH - 2;
}

function orbAt(s) {
  const [ox, oy] = orbOffset(s.t);
  return [s.x + ox, oy];
}

/** Does a point (x, height above ground) touch fighter f, give or take r? */
function overlaps(f, x, y, r) {
  return Math.abs(x - f.x) < HALF_WIDTH + r && y > f.air - r && y < f.air + BODY + r;
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
  if (me.think > 0 || me.act || me.stun > 0 || me.frozen > 0 || me.shocked > 0 || me.hp <= 0) return;
  me.think = THINK * (0.8 + duel.rng() * 0.4);
  const options = [...dangers(duel, me), ...blows(duel, me, foe), ...attacks(duel, me, foe), ...positions(duel, me, foe)];
  let best = null;
  for (const o of options) {
    o.score *= 0.9 + duel.rng() * 0.2;
    if (!best || o.score > best.score) best = o;
  }
  me.intent = { move: 0 };
  if (best.spell !== "meditate") setMeditating(duel, me, false);
  if (best.spell === "move") me.intent = { move: best.move };
  else if (best.spell === "jump") {
    if (me.air === 0) me.vy = JUMP_SPEED;
  } else if (best.spell === "guard") me.guard = best.hold;
  else if (best.spell === "fly") {
    me.fly = best.hover > 0;
    me.hover = best.hover;
    if (me.fly && me.air === 0) me.vy = JUMP_SPEED * 0.6;
  }
  else if (best.spell === "meditate") setMeditating(duel, me, true);
  else if (best.spell !== "wait") begin(duel, me, best.spell, best.target);
}

const ready = (me, name) =>
  me.spells.includes(name) && !(me.cool[name] > 0) && me.qi >= SPELLS[name].cost && (!SPELLS[name].ultimate || me.rage >= RAGE) &&
  !(name === "blink" && me.sealed > 0) &&
  // silenced, only the blade answers
  !(me.silence > 0 && SPELLS[name].kind !== "melee");

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
      if (!toward || tti > 0.6 || s.y < me.air - 3 || s.y > me.air + BODY + 3) continue;
      const urgency = s.spell.damage * (1.2 - tti);
      if (Math.sign(s.x - me.x) === me.face) out.push({ spell: "guard", hold: tti + 0.15, score: urgency * 1.1 });
      if (me.air === 0 && !me.fly && tti < 0.24 && tti > 0.08) out.push({ spell: "jump", score: urgency * 1.25 });
      // on the sword: rise over it, or sink under it
      if (me.qi > 12 && tti > 0.15) {
        const over = s.y + 4;
        if (over < FLY.top) out.push({ spell: "fly", hover: over, score: urgency * 1.15 });
        if (me.fly && s.y > BODY + 2) out.push({ spell: "fly", hover: 0, score: urgency * 1.1 });
      }
      if (ready(me, "bolt") && tti > 0.3) out.push({ spell: "bolt", target: null, score: urgency * 0.9 });
      const b = blink(s.x);
      if (b) b.score = urgency * 0.7;
    } else if (s.kind === "orb") {
      const [bx, by] = orbAt(s);
      const d = Math.hypot(bx - me.x, -by - (me.air + CHEST));
      if (d > 48) continue;
      const urgency = s.spell.damage * (1.4 - d / 48);
      out.push({ spell: "move", move: Math.sign(me.x - bx) || 1, score: urgency * 1.0 });
      if (ready(me, "bolt")) out.push({ spell: "bolt", target: null, score: urgency * 1.15 });
      const b = blink(bx);
      if (b) b.score = urgency * 0.9;
    } else if (s.kind === "zone" && !s.fell && s.t + (s.lead ?? 0) >= s.spell.notice) {
      const dx = me.x - s.x;
      if (Math.abs(dx) > (s.spell.reach ?? 0) + s.spell.radius + HALF_WIDTH + 4) continue;
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
  if (d > spell.range + HALF_WIDTH + 6) return [];
  const left = spell.windup - act.t;
  const urgency = spell.damage * 1.2;
  const out = [];
  // backing off works if I can walk out of reach before it lands
  const out_ = spell.range + HALF_WIDTH + 2 - d;
  if (out_ / WALK < left + 0.05) out.push({ spell: "move", move: Math.sign(me.x - foe.x) || 1, score: urgency * 1.2 });
  if (Math.sign(foe.x - me.x) === me.face) out.push({ spell: "guard", hold: left + 0.2, score: urgency * 0.9 });
  if (ready(me, "slash") && SPELLS.slash.windup < left && d < SPELLS.slash.range + HALF_WIDTH) {
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
  const busy = foe.stun + (foe.act ? SPELLS[foe.act.spell][foe.act.phase] - foe.act.t + (foe.act.phase === "windup" ? SPELLS[foe.act.spell].recover : 0) : 0);
  const out = [];
  const value = (name, chance) => {
    const s = SPELLS[name];
    return ((s.damage ?? 0) * chance * school.aggression) / (0.5 + s.windup + s.cost / 40);
  };
  if (ready(me, "dive") && me.air - foe.air > 14 && d < 44) {
    out.push({ spell: "dive", score: value("dive", busy > SPELLS.dive.windup ? 0.9 : foe.guard > 0 ? 0.25 : 0.6) });
  }
  if (ready(me, "slash") && d < SPELLS.slash.range + 8 && Math.abs(foe.air - me.air) < CHEST) {
    out.push({ spell: "slash", score: value("slash", busy > SPELLS.slash.windup ? 0.95 : foe.guard > 0 ? 0.15 : 0.55) });
  }
  // a bolt flies level from my chest: it must cross the foe's body
  const level = me.air + CHEST > foe.air - 2 && me.air + CHEST < foe.air + BODY;
  if (ready(me, "bolt") && d > 24 && level) {
    const travel = d / SPELLS.bolt.speed + SPELLS.bolt.windup;
    // a bolt into a bolt the foe is gathering only cancels it
    const trade = foe.act?.spell === "bolt" && foe.act.phase === "windup";
    const chance = trade ? 0.1 : busy > travel ? 0.9 : foe.guard > 0 ? 0.2 : 0.45;
    out.push({ spell: "bolt", score: value("bolt", chance) });
  }
  if (ready(me, "orb") && d > 40 && !duel.shots.some((s) => s.kind === "orb" && s.owner === me.index)) {
    out.push({ spell: "orb", score: value("orb", 0.5) });
  }
  for (const name of ["tribulation", "prison"]) {
    if (!ready(me, name)) continue;
    const lead = SPELLS[name].windup + SPELLS[name].impact;
    const pace = foe.act || foe.stun > 0 || foe.medit ? 0 : foe.intent.move * WALK;
    out.push({ spell: name, target: foe.x + pace * lead * 0.8, score: value(name, 0.8) * (SPELLS[name].strikes ?? 1) + 3 });
  }
  for (const name of me.spells) {
    if (SPELLS[name].kind !== "zone" || SPELLS[name].ultimate || !ready(me, name)) continue;
    // aim where the foe will be when it lands
    const lead = SPELLS[name].windup + (SPELLS[name].sigil ?? 0) + SPELLS[name].impact;
    const pace = foe.act || foe.stun > 0 || foe.medit ? 0 : foe.intent.move * WALK;
    const target = foe.x + pace * lead * 0.8;
    // an icicle only finds a foe standing on the floor
    const grounded = SPELLS[name].grounded ? (foe.air < 10 && !foe.fly ? 1 : 0.1) : 1;
    out.push({ spell: name, target, score: value(name, (busy > lead * 0.6 ? 0.85 : SPELLS[name].pull ? 0.5 : 0.4) * grounded) });
  }
  for (const name of me.spells) {
    const s = SPELLS[name];
    if (s.kind === "blast" && ready(me, name) && d < s.range + 4) {
      out.push({ spell: name, score: value(name, busy > s.windup ? 0.9 : foe.guard > 0 ? 0.2 : 0.55) });
    }
    // a fire or a vortex under the foe: worth it on a foe that cannot leave it soon
    if (s.kind === "void" && ready(me, name) && (busy > 0.4 || foe.frozen > 0 || foe.hp < 75)) {
      out.push({ spell: name, score: value(name, 0.7) + 4 });
    }
    // a lightning step: the foe within a stride, on my level
    if (s.kind === "dash" && ready(me, name) && d > 12 && d < s.distance - 10 && Math.abs(foe.air - me.air) < CHEST) {
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

/** Moving to the distance I fight best at, meditating when it is quiet, or waiting. */
function positions(duel, me, foe) {
  const d = foe.x - me.x;
  // the distance a fighter keeps comes from what it can use: its school's range with spells that fly or fall,
  // arm's length with a palm blast, sword's length with nothing else
  const has = (...kinds) => me.spells.some((name) => kinds.includes(SPELLS[name].kind));
  // and with too little qi to cast, it closes in to use the sword
  const casting = me.qi >= 25;
  const range = casting && has("bolt", "orb", "zone", "void") ? SCHOOLS[me.school].range : casting && has("blast", "dash") ? 26 : SPELLS.slash.range - 4;
  const off = Math.abs(d) - range;
  const out = [{ spell: "wait", score: 0.4 }];
  // inside the foe's sword reach with no blade of my own ready: get out of it
  if (Math.abs(d) < SPELLS.slash.range + 4 && me.air - foe.air < CHEST && !ready(me, "slash")) {
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
  if (!casting || Math.abs(d) < 40) want = foe.air > 16 && me.qi > 20 ? Math.min(FLY.top, foe.air) : want;
  const flying = me.fly ? me.hover : 0;
  if (Math.abs(flying - want) > 8) out.push({ spell: "fly", hover: want, score: 0.9 });
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
  emit(duel, { type: "streak", by: me.index, x0: me.x + me.face * 6, y0: me.air + CHEST, x1: x, y1: y });
}

function emit(duel, command) {
  duel.out.push(command);
}

function clamp(v, lo, hi) {
  return Math.max(lo, Math.min(hi, v));
}
