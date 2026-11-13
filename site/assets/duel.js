// The hero's duel: two cultivators fight with spells, and every spell is a pixfx effect. This module is the game
// only: rules, spells, and each fighter's mind. It draws nothing and never touches the engine. `step` returns what
// the engine should show (play, move or stop an effect); landing.js plays that and draws the fighters.
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
const ROUND_PAUSE = 3.2;

const CAST_CIRCLE = "arcane/rune-circle";
const CLASH = "impacts/parry";

/**
 * Each spell: what it costs, how long the fighter is committed before (`windup`) and after (`recover`) it lands,
 * its cooldown, and its own numbers. `fx` is the effect that shows it; a spell whose effect is not in the registry
 * is left out of the fight (slash and block need none).
 */
export const SPELLS = {
  slash: { kind: "melee", cost: 0, windup: 0.2, recover: 0.32, cooldown: 0.55, range: 20, damage: 8 },
  bolt: { kind: "bolt", fx: "projectiles/laser-bolt", cost: 12, windup: 0.28, recover: 0.22, cooldown: 1.3, speed: 230, damage: 9 },
  orb: {
    kind: "orb", fx: "lightning/ball-lightning", hitFx: "explosions/plasma-explosion",
    cost: 30, windup: 0.6, recover: 0.3, cooldown: 4.5, speed: 26, life: 5.5, damage: 12, radius: 10,
  },
  // icicle-fall drops its icicle at FREE + fall time: 0.64 + sqrt(2 * (80 - 36) / 950) s, and only starts to tremble,
  // which is when a fighter notices it overhead, at 0.3 s (pixelfx ice/icicle-fall.js TREMBLE)
  icicle: { kind: "zone", fx: "ice/icicle-fall", cost: 22, windup: 0.42, recover: 0.28, cooldown: 2.2, impact: 0.944, radius: 16, damage: 20, notice: 0.3, height: 0, grounded: true },
  // implosion sucks in until 0.8 s and pops at 1.02 s (pixelfx explosions/implosion.js SUCK, STILL)
  collapse: {
    kind: "zone", fx: "explosions/implosion", cost: 26, windup: 0.5, recover: 0.3, cooldown: 2.4,
    impact: 1.02, radius: 18, damage: 18, notice: 0.15, height: CHEST, pull: 30, reach: 46, pullUntil: 0.8,
  },
  // a thunderclap from the open palm, at arm's length
  palm: { kind: "blast", fx: "explosions/plasma-explosion", cost: 18, windup: 0.32, recover: 0.35, cooldown: 2.8, range: 30, damage: 14 },
  // void-orb hangs 62 px over its origin (pixelfx shadow/void-orb.js Y) and loops every 3.2 s
  void: {
    kind: "void", fx: "shadow/void-orb", endFx: "explosions/implosion",
    cost: 60, windup: 0.9, recover: 0.45, cooldown: 11, life: 3.2, height: 62, pull: 34, reach: 90, under: 22, dps: 8, damage: 10,
  },
  heal: { kind: "heal", fx: "holy/feather-burst", cost: 40, windup: 0.85, recover: 0.3, cooldown: 9, amount: 20 },
  blink: { kind: "blink", fx: "spawn/poof", cost: 20, windup: 0.06, recover: 0.12, cooldown: 1.8, distance: 56 },
};

/** The two schools: their spells, the distance each likes to fight at, how eager it is, and its colours. */
export const SCHOOLS = {
  thunder: {
    name: "Thunder", spells: ["slash", "palm", "bolt", "orb", "blink", "heal"], range: 52, aggression: 1.15,
    bolt: ["#2a1a08", "#6a4410", "#b07a1a", "#e8b030", "#ffd84a", "#ffe98a", "#fff6c4", "#ffffff"],
    circle: ["#2a1a08", "#6a4410", "#b07a1a", "#e8b030", "#ffd84a", "#fff6c4", "#ffffff"],
  },
  frost: {
    name: "Frost", spells: ["slash", "bolt", "icicle", "collapse", "void", "blink", "heal"], range: 84, aggression: 1.1,
    bolt: ["#151a3d", "#212f6b", "#2c58aa", "#3a8bd8", "#62c8f4", "#a6ecff", "#dcfbff", "#ffffff"],
    circle: ["#151a3d", "#212f6b", "#2c58aa", "#3a8bd8", "#62c8f4", "#dcfbff", "#ffffff"],
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
  return { index, school, spells, x: 0, air: 0, vy: 0, vx: 0, face: 1, hp: 100, qi: 100, act: null, stun: 0, guard: 0, medit: false, cool: {}, think: 0, intent: { move: 0 }, aura: null, pose: "idle", poseT: 0, flash: 0 };
}

function startRound(duel) {
  const span = duel.right - duel.left;
  duel.round++;
  duel.pause = 0;
  duel.winner = null;
  for (const s of duel.shots) stopShot(duel, s);
  duel.shots = [];
  for (const f of duel.fighters) {
    Object.assign(f, { air: 0, vy: 0, vx: 0, hp: 100, qi: 100, act: null, stun: 0, guard: 0, medit: false, cool: {}, think: 0.4 + duel.rng() * 0.3, intent: { move: 0 }, flash: 0 });
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
  for (const s of [...duel.shots]) shot(duel, s, dt);
  for (const f of duel.fighters) if (f.hp <= 0 && duel.winner === null) knockOut(duel, f);
}

// ---------- bodies: moving, casting, getting hit ----------

function body(duel, me, foe, dt) {
  me.flash = Math.max(0, me.flash - dt);
  me.poseT += dt;
  me.stun = Math.max(0, me.stun - dt);
  me.guard = Math.max(0, me.guard - dt);
  for (const k of Object.keys(me.cool)) me.cool[k] = Math.max(0, me.cool[k] - dt);
  me.qi = Math.min(100, me.qi + (me.medit ? MEDITATE_REGEN : QI_REGEN) * dt);
  if (!me.act && me.stun === 0) me.face = foe.x >= me.x ? 1 : -1;

  // feet: walking (only when free), knockback, the jump arc
  const free = !me.act && me.stun === 0 && !me.medit;
  const walk = free && me.guard === 0 ? me.intent.move * WALK : 0;
  me.vx *= Math.exp(-6 * dt);
  me.x = clamp(me.x + (walk + me.vx) * dt, duel.left + HALF_WIDTH, duel.right - HALF_WIDTH);
  if (me.air > 0 || me.vy > 0) {
    me.vy -= GRAVITY * dt;
    me.air = Math.max(0, me.air + me.vy * dt);
    if (me.air === 0) me.vy = 0;
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
  if (f.hp <= 0) return "down";
  if (f.stun > 0) return "hurt";
  if (f.act) {
    const k = SPELLS[f.act.spell].kind;
    if (k === "melee") return f.act.phase === "windup" ? "draw" : "slash";
    return f.act.phase === "windup" ? "channel" : "cast";
  }
  if (f.guard > 0) return "guard";
  if (f.medit) return "meditate";
  if (f.air > 0) return "jump";
  if (f.intent.move !== 0) return "walk";
  return "idle";
}

function begin(duel, me, name, target) {
  const spell = SPELLS[name];
  me.qi -= spell.cost;
  me.cool[name] = spell.cooldown + spell.windup;
  me.act = { spell: name, phase: "windup", t: 0, target };
  setMeditating(duel, me, false);
  // longer casts stand in a rune circle while they gather
  if (spell.windup >= 0.4 && duel.has(CAST_CIRCLE)) {
    me.act.circle = play(duel, CAST_CIRCLE, me.x, duel.ground, { palette: SCHOOLS[me.school].circle, seed: me.index + 1 });
  }
}

/** The moment a spell lands: it hits, flies, falls or heals. */
function release(duel, me, foe, spell) {
  const g = duel.ground;
  if (spell.kind === "melee") {
    me.vx += me.face * 90;
    const dx = Math.abs(foe.x - me.x);
    if (dx <= spell.range + HALF_WIDTH && Math.abs(foe.air - me.air) < CHEST && foe.hp > 0) {
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
    play(duel, spell.fx, x, g - spell.height, { seed: Math.floor(duel.rng() * 1e6) });
    duel.shots.push({ kind: "zone", name: me.act.spell, owner: me.index, x, t: 0, key: null, spell });
  } else if (spell.kind === "blast") {
    const x = me.x + me.face * spell.range * 0.7;
    play(duel, spell.fx, x, g - CHEST - me.air, { seed: Math.floor(duel.rng() * 1e6) });
    if (Math.abs(foe.x - me.x) <= spell.range + HALF_WIDTH && Math.abs(foe.air - me.air) < BODY && foe.hp > 0) {
      if (blocks(foe, me.x, spell.damage)) clash(duel, foe.x - me.face * 4, g - CHEST - foe.air, [foe]);
      else hurt(duel, foe, spell.damage, me.face, me.act.spell);
    }
  } else if (spell.kind === "void") {
    const x = clamp(foe.x, duel.left + 10, duel.right - 10);
    const key = play(duel, spell.fx, x, g, { seed: me.index + 5 });
    duel.shots.push({ kind: "void", owner: me.index, x, t: 0, key, spell });
  } else if (spell.kind === "heal") {
    me.hp = Math.min(100, me.hp + spell.amount);
    play(duel, spell.fx, me.x, g - BODY - me.air, { seed: Math.floor(duel.rng() * 1e6) });
  } else if (spell.kind === "blink") {
    play(duel, spell.fx, me.x, g - me.air, { seed: Math.floor(duel.rng() * 1e6) });
    me.x = clamp(me.act.target, duel.left + HALF_WIDTH, duel.right - HALF_WIDTH);
    me.air = 0;
    me.vy = 0;
    play(duel, spell.fx, me.x, g, { seed: Math.floor(duel.rng() * 1e6) });
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

/** f takes `damage` from `spell`, pushed toward `dir`; the step reports it as a `hit` command. */
function hurt(duel, f, damage, dir, spell) {
  if (f.hp <= 0) return;
  emit(duel, { type: "hit", by: 1 - f.index, spell, damage });
  f.hp = Math.max(0, f.hp - damage);
  f.flash = 0.18;
  f.stun = STAGGER;
  f.vx += dir * KNOCKBACK;
  // a hit breaks a cast: what was gathering is lost
  if (f.act?.circle !== undefined && f.act.phase === "windup") emit(duel, { type: "stop", key: f.act.circle });
  f.act = null;
  f.guard = 0;
  setMeditating(duel, f, false);
}

function clash(duel, x, y, fighters) {
  if (duel.has(CLASH)) play(duel, CLASH, x, y + 30, { height: 30, seed: Math.floor(duel.rng() * 1e6) });
  for (const f of fighters) f.vx += Math.sign(f.x - x || 1) * KNOCKBACK;
}

function knockOut(duel, f) {
  const other = duel.fighters[1 - f.index];
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
      else hurt(duel, foe, spell.damage, Math.sign(s.vx), "bolt");
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
    // a collapse drags the foe toward its heart while it sucks in
    if (spell.pull && s.t < spell.pullUntil && Math.abs(s.x - foe.x) < spell.reach && foe.hp > 0) {
      const dx = s.x - foe.x;
      foe.x += Math.sign(dx) * Math.min(Math.abs(dx), spell.pull * dt);
    }
    if (!s.fell && s.t >= spell.impact) {
      s.fell = true;
      const reaches = Math.abs(foe.x - s.x) < spell.radius + HALF_WIDTH && (!spell.grounded || foe.air < 10);
      if (reaches) hurt(duel, foe, spell.damage, Math.sign(foe.x - s.x) || 1, s.name);
    }
    if (s.t >= spell.impact + 0.2) duel.shots.splice(duel.shots.indexOf(s), 1);
  } else if (s.kind === "void") {
    // the vortex drags the foe in, and burns while it stands under it
    const dx = s.x - foe.x;
    if (Math.abs(dx) < spell.reach && foe.hp > 0) {
      foe.x += Math.sign(dx) * Math.min(Math.abs(dx), spell.pull * dt);
      if (Math.abs(dx) < spell.under) foe.hp = Math.max(0, foe.hp - spell.dps * dt);
    }
    if (s.t >= spell.life) {
      burst(duel, spell.endFx, s.x, g - spell.height);
      if (Math.abs(dx) < spell.under) hurt(duel, foe, spell.damage, -Math.sign(dx) || 1, "void");
      endShot(duel, s);
    }
  }
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
  if (me.think > 0 || me.act || me.stun > 0 || me.hp <= 0) return;
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
  else if (best.spell === "meditate") setMeditating(duel, me, true);
  else if (best.spell !== "wait") begin(duel, me, best.spell, best.target);
}

const ready = (me, name) => me.spells.includes(name) && !(me.cool[name] > 0) && me.qi >= SPELLS[name].cost;

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
    if (s.owner === me.index || s.t < REACTION) continue;
    if (s.kind === "bolt") {
      const toward = Math.sign(me.x - s.x) === Math.sign(s.vx);
      const tti = Math.abs(me.x - s.x) / Math.abs(s.vx);
      if (!toward || tti > 0.6 || s.y < me.air - 3 || s.y > me.air + BODY + 3) continue;
      const urgency = s.spell.damage * (1.2 - tti);
      if (Math.sign(s.x - me.x) === me.face) out.push({ spell: "guard", hold: tti + 0.15, score: urgency * 1.1 });
      if (me.air === 0 && tti < 0.24 && tti > 0.08) out.push({ spell: "jump", score: urgency * 1.25 });
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
    } else if (s.kind === "zone" && !s.fell && s.t >= s.spell.notice) {
      const dx = me.x - s.x;
      if (Math.abs(dx) > (s.spell.reach ?? 0) + s.spell.radius + HALF_WIDTH + 4) continue;
      const left = s.spell.impact - s.t;
      const urgency = s.spell.damage * (1.5 - left);
      const way = Math.abs(dx) < 1 ? (s.x - duel.left > duel.right - s.x ? -1 : 1) : Math.sign(dx);
      out.push({ spell: "move", move: way, score: urgency * 1.3 });
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
  if (ready(me, "slash") && d < SPELLS.slash.range + 8 && Math.abs(foe.air - me.air) < CHEST) {
    out.push({ spell: "slash", score: value("slash", busy > SPELLS.slash.windup ? 0.95 : foe.guard > 0 ? 0.15 : 0.55) });
  }
  if (ready(me, "bolt") && d > 24) {
    const travel = d / SPELLS.bolt.speed + SPELLS.bolt.windup;
    // a bolt into a bolt the foe is gathering only cancels it
    const trade = foe.act?.spell === "bolt" && foe.act.phase === "windup";
    const chance = trade ? 0.1 : busy > travel ? 0.9 : foe.guard > 0 ? 0.2 : 0.45;
    out.push({ spell: "bolt", score: value("bolt", chance) });
  }
  if (ready(me, "orb") && d > 40 && !duel.shots.some((s) => s.kind === "orb" && s.owner === me.index)) {
    out.push({ spell: "orb", score: value("orb", 0.5) });
  }
  for (const name of ["icicle", "collapse"]) {
    if (!ready(me, name)) continue;
    // aim where the foe will be when it lands
    const lead = SPELLS[name].windup + SPELLS[name].impact;
    const pace = foe.act || foe.stun > 0 || foe.medit ? 0 : foe.intent.move * WALK;
    const target = foe.x + pace * lead * 0.8;
    out.push({ spell: name, target, score: value(name, busy > lead * 0.6 ? 0.85 : SPELLS[name].pull ? 0.5 : 0.4) });
  }
  if (ready(me, "palm") && d < SPELLS.palm.range + 4) {
    out.push({ spell: "palm", score: value("palm", busy > SPELLS.palm.windup ? 0.9 : foe.guard > 0 ? 0.2 : 0.55) });
  }
  if (ready(me, "void") && (foe.hp < 75 || busy > 0.4)) {
    out.push({ spell: "void", score: value("void", 0.7) + 4 });
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
  const range = casting && has("bolt", "orb", "zone", "void") ? SCHOOLS[me.school].range : casting && has("blast") ? SPELLS.palm.range - 6 : SPELLS.slash.range - 4;
  const off = Math.abs(d) - range;
  const out = [{ spell: "wait", score: 0.4 }];
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
  const threatened = duel.shots.some((s) => s.owner !== me.index);
  if (me.qi < 45 && Math.abs(d) > 70 && !threatened && !foe.act) out.push({ spell: "meditate", score: 2.2 - me.qi / 40 });
  return out;
}

// ---------- effect commands ----------

function play(duel, fx, x, y, opts) {
  const key = ++duel.key;
  emit(duel, { type: "play", key, fx, x, y, opts });
  return key;
}

function emit(duel, command) {
  duel.out.push(command);
}

function clamp(v, lo, hi) {
  return Math.max(lo, Math.min(hi, v));
}
