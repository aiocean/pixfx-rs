// A bout: the hero duel's two fighters staging one effect on its own page, the way a real fight would use it. Each
// take walks the two to where the effect's role wants them, has the caster gather with the matching pose, plays the
// effect where it would appear in a fight, makes the one it reaches react when it lands, then lets both settle back
// to rest. The fighters' bodies are pixfx-fighter's `Body`, driven the way duel.js drives them; this module draws
// nothing. `step` returns what duel.js's `step` returns: effect commands (`play`, `move`, `stop`) and the events
// the screen feels (`hit`, `tick`, `block`, `clash`, `land`, `bump`, `streak`), so the page plays and feels them as
// the hero does.
//
// Units are game pixels and seconds. x grows to the right; heights (`air`, event y) are measured up from the ground,
// effect y in screen rows like duel.js's play commands.
import { SPELLS, STATS, damageOf, orbOffset } from "./duel.js";

/** How high a fighter may fly. */
const TOP = 96;
/** A blow whose effect sends no cue lands this long after the effect's own cue time. */
const GRACE = 0.15;
/** How long both fighters must stand still before a take is over. */
const SETTLE = 0.35;
/** The longest a take may run, in seconds: past it, the take ends where it is. */
export const TAKE_MAX = 10;
/** How long the fighters may take to walk to their places before they are put there. */
const APPROACH_MAX = 3;
/** The share of a jet's length the caster stands from the foe, so the foe stands in the stream. */
const JET_REACH = 0.75;
/** How far apart the two stand for each role, before the arena's width caps it. */
const DISTANCE = {
  strike: 72, projectile: 112, stream: 88, ward: 100, buff: 60, "ground-self": 40, "ground-foe": 70,
  clash: 26, flight: 64, ambient: 30, blink: 70, death: 60,
};
/** Knockback two blades that meet give each other (duel.js KNOCKBACK). */
const KNOCKBACK = 130;

/** A small seeded generator (mulberry32, as duel.js), so a seed replays the same bout. */
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

/** The body's fields, read and written on the fighter as if they were its own (duel.js BODY_FIELDS). */
const BODY_FIELDS = ["x", "air", "vx", "vy", "face", "stun", "wake", "land", "squat", "leap", "duck", "flash", "hitT", "hitPower", "frozen", "shocked", "poseT"];

/** Fighter `index` of `school`, standing on `body`, with every field duel-art.js reads off a duel's fighter. */
function fighter(index, school, body) {
  const f = {
    index, school, stats: STATS[school], hp: 100, qi: 100, rage: 0, act: null, guard: 0, fly: false, hover: 0, intent: { move: 0 },
    shield: 0, barrier: 0, rang: 0, burn: null, defDown: 0, silence: 0, chilled: 0, fury: 0, dodged: 0, cool: {}, coolFull: {},
    body, spec: body.spec,
  };
  for (const k of BODY_FIELDS) {
    Object.defineProperty(f, k, { enumerable: true, get: () => body[k], set: (v) => { body[k] = v; } });
  }
  for (const k of ["pose", "down"]) Object.defineProperty(f, k, { enumerable: true, get: () => body[k] });
  return f;
}

/**
 * A new bout in the arena from `left` to `right` (game pixels), standing on `ground`. `Body` is pixfx-fighter's body
 * (from the engine's wasm package). Thunder (fighter 0) stands on the left, Frost (fighter 1) on the right. `done`
 * is true until the first take.
 */
export function createBout({ left, right, ground, Body, seed = 1 }) {
  const span = right - left;
  const bout = {
    seed, rng: random(seed), left, right, ground, arena: { left, right, top: TOP }, Body, time: 0, key: 0, takes: 0, caster: null, take: null, done: true, caption: "", out: [], queued: [],
  };
  bout.fighters = [fighter(0, "thunder", new Body(left + span * 0.3)), fighter(1, "frost", new Body(left + span * 0.7))];
  bout.fighters[1].body.face = -1;
  return bout;
}

/**
 * Start a take that stages effect `fx` in `role` (a role name, or `roleFor`'s `{ role, ...tuning }`), playing it with
 * the engine options `opts`. Call it again on the same bout for the next take (after `done`, or any time: a take
 * still running is cut short and its effects stop). The fighters take turns casting: Thunder the first take, then
 * whoever did not cast the last one. `caster` (0 for Thunder, 1 for Frost) picks who casts this take instead, and the
 * turns go on from there. `bout.caster` is who cast the last take (null before the first), `bout.caption` says in
 * words what the take shows ("Thunder throws it at Frost").
 */
export function take(bout, fx, role, opts = {}, { caster: pick } = {}) {
  if (pick !== undefined && pick !== 0 && pick !== 1) throw new Error(`take: caster must be 0 or 1, got ${pick}`);
  if (bout.take) finish(bout, bout.take, bout.queued, true);
  const tune = typeof role === "string" ? { role } : role;
  const caster = bout.fighters[pick ?? (bout.caster === null ? 0 : 1 - bout.caster)];
  bout.caster = caster.index;
  const target = bout.fighters[1 - caster.index];
  bout.takes++;
  for (const f of bout.fighters) {
    f.act = null;
    f.guard = 0;
    f.fly = false;
    f.intent.move = 0;
    f.chilled = 0;
  }
  const span = bout.right - bout.left;
  let distance = tune.distance ?? DISTANCE[tune.role] ?? DISTANCE.strike;
  if (tune.role === "stream") distance = (opts.length ?? tune.length ?? 118) * JET_REACH;
  distance = clamp(distance, 24, span - 40);
  const mid = (bout.left + bout.right) / 2 + (tune.role === "ambient" && tune.at === "side" ? span * 0.12 : 0);
  bout.take = {
    fx, tune, opts, caster, target, phase: "approach", t: 0, age: 0, keys: [], cueKeys: new Set(), blows: [], playT: 0,
    spots: [mid - distance / 2, mid + distance / 2], hits: 0, firstPlay: null, still: 0, shot: null, ticks: 0, step: 0,
  };
  bout.done = false;
  bout.caption = caption(tune, caster, target);
  // a backdrop is there from the start: the two walk into it (a wipe waits for the blow, and ends the scene)
  if (tune.role === "ambient" && tune.at !== "wipe") ambientProp(bout, bout.take);
}

/** What each role shows, in words: `{c}` is the caster's school's name, `{t}` the other's. */
const CAPTIONS = {
  strike: "{c} calls it down on {t}",
  projectile: "{c} throws it at {t}",
  stream: "{c} pours it onto {t}",
  ward: "{c} shields behind it while {t} strikes",
  buff: "{c} powers up with it, then strikes {t}",
  "ground-self": "{c} gathers in it, then strikes {t}",
  "ground-foe": "{c} traps {t} in it",
  clash: "{c} and {t} cross blades in it",
  flight: "{c} flies on it and dives at {t}",
  ambient: "{c} and {t} spar beside it",
  blink: "{c} vanishes in it and strikes {t} from behind",
  death: "{c} strikes {t} down, and {t}'s body bursts apart",
};

function caption(tune, caster, target) {
  const name = (f) => f.school[0].toUpperCase() + f.school.slice(1);
  const text = tune.role === "ambient" && tune.at === "wipe" ? "{c} and {t} spar, and it ends the scene" : CAPTIONS[tune.role] ?? CAPTIONS.strike;
  return text.replaceAll("{c}", name(caster)).replaceAll("{t}", name(target));
}

/**
 * The effect played under `key` sent a cue (its impact). The blow it marks lands now: its commands come out of the
 * next `step`. Cues of effects the take is not waiting on are let be.
 */
export function impact(bout, key) {
  const T = bout.take;
  if (!T || !T.cueKeys.has(key) || T.blows.length === 0) return;
  // an effect's cue may come as several (a shake, a hit-stop, a flash): the blow nearest in time is the one it marks
  const at = bout.time - T.playT;
  let best = T.blows[0];
  for (const b of T.blows) if (Math.abs(b.at - at) < Math.abs(best.at - at)) best = b;
  if (best.done) return;
  best.done = true;
  best.land(bout.queued);
}

/** Advance the bout by `dt` seconds; returns the commands and events of this step. */
export function step(bout, dt) {
  bout.out = bout.queued;
  bout.queued = [];
  // fixed sub-steps, so the bout does not depend on the frame rate
  let left = Math.min(dt, 0.25);
  while (left > 1e-9) {
    const h = Math.min(left, 1 / 60);
    tick(bout, h);
    left -= h;
  }
  return bout.out;
}

function tick(bout, dt) {
  bout.time += dt;
  const [a, b] = bout.fighters;
  if (bout.take) direct(bout, bout.take, dt, bout.out);
  for (const [me, foe] of [[a, b], [b, a]]) move(bout, me, foe, dt, bout.out);
  bout.Body.separate(a.body, b.body, bout.arena);
}

/** One step of fighter me's body, driven as duel.js drives it, and its act's clock. */
function move(bout, me, foe, dt, out) {
  me.body.tick(dt, true);
  me.rang = Math.max(0, me.rang - dt);
  me.dodged = Math.max(0, me.dodged - dt);
  me.guard = Math.max(0, me.guard - dt);
  me.chilled = Math.max(0, me.chilled - dt);
  me.defDown = Math.max(0, me.defDown - dt);
  me.silence = Math.max(0, me.silence - dt);
  if (me.burn) {
    me.burn.t -= dt;
    if (me.burn.t <= 0) me.burn = null;
  }
  const diving = me.act?.phase === "windup" && SPELLS[me.act.spell].dive;
  const motion = diving ? { dive: SPELLS.dive.speed, floor: foe.air } : me.fly ? { fly: me.hover } : "gravity";
  const drive = { alive: true, acting: me.act !== null, meditating: false, guarding: me.guard > 0, chilled: me.chilled > 0, intent: me.intent.move, motion, foeX: foe.x };
  for (const e of me.body.step(dt, drive, bout.arena, bout.rng)) out.push(e);
  if (me.act) {
    me.act.t += dt;
    const spell = SPELLS[me.act.spell];
    if (me.act.phase === "windup" && me.act.t >= spell.windup) me.act = { ...me.act, phase: "recover", t: 0, released: true };
    else if (me.act.phase === "recover" && me.act.t >= spell.recover && !me.act.hold) me.act = null;
  }
  const doing = me.act ? me.act.phase : me.guard > 0 ? "guard" : me.fly ? "fly" : me.intent.move !== 0 ? "walk" : "idle";
  me.body.updatePose(doing, me.act !== null && SPELLS[me.act.spell].kind === "melee", true);
}

// ---------- the take ----------

/** Move the take on by one step: walk to places, then the role's script, then settle. */
function direct(bout, T, dt, out) {
  T.t += dt;
  T.age += dt;
  if (T.age > TAKE_MAX) return finish(bout, T, out, true);
  if (T.phase === "approach") return approach(bout, T);
  if (T.phase === "rest") return rest(bout, T, dt, out);
  // a blow no cue has marked (its effect sends none, or that cue never came) lands at its own time
  for (const b of T.blows) {
    if (!b.done && bout.time - T.playT >= b.at + GRACE) {
      b.done = true;
      b.land(out);
    }
  }
  (SCRIPTS[T.tune.role] ?? SCRIPTS.strike)(bout, T, dt, out);
}

function go(T, phase) {
  T.phase = phase;
  T.t = 0;
}

/** Each fighter walks to its place, Thunder on the left; once both stand there, facing each other, the role begins. */
function approach(bout, T) {
  let there = true;
  for (const f of bout.fighters) {
    const dx = T.spots[f.index] - f.x;
    const busy = f.act !== null || f.body.reeling() || f.down !== null || f.air > 0.5;
    f.intent.move = Math.abs(dx) > 1.5 && !busy ? Math.sign(dx) : 0;
    if (Math.abs(dx) > 1.5 || busy) there = false;
  }
  // a fighter that cannot get there (pinned by the other) is put there
  if (!there && T.t > APPROACH_MAX) {
    for (const f of bout.fighters) if (f.down === null && f.air < 0.5) f.x = T.spots[f.index];
    there = true;
  }
  if (!there) return;
  for (const f of bout.fighters) f.intent.move = 0;
  go(T, "begin");
}

/**
 * Both stand still for a moment and an effect that plays once has played out; then the take is over and what loops
 * stops.
 */
function rest(bout, T, dt, out) {
  for (const f of bout.fighters) f.intent.move = 0;
  const still = bout.fighters.every((f) => f.act === null && !f.body.reeling() && f.down === null && f.air < 0.5 && f.frozen === 0 && !f.fly);
  T.still = still ? T.still + dt : 0;
  const playing = !T.tune.loop && T.firstPlay !== null && bout.time - T.firstPlay < (T.tune.duration ?? 0);
  if (T.still >= SETTLE && !playing) finish(bout, T, out, false);
}

/**
 * The take ends: what it played that loops stops (`cut`: a take cut short stops everything it played), and the
 * fighters let go of what they were doing.
 */
function finish(bout, T, out, cut) {
  if (cut || T.tune.loop) for (const key of T.keys) out.push({ type: "stop", key });
  T.keys = [];
  for (const f of bout.fighters) {
    f.intent.move = 0;
    f.fly = false;
    if (f.act?.hold) f.act = null;
  }
  bout.take = null;
  bout.done = true;
}

/** The fighter starts gathering (or swinging) `spell`, in the pose duel-art.js gives its kind. */
function gather(f, spell) {
  f.act = { spell, phase: "windup", t: 0 };
}

/** Did f's act just reach its release? (Each release is read once.) */
function released(f) {
  if (!f.act?.released) return false;
  f.act.released = false;
  return true;
}

/** Play the take's effect at x, `height` up from the ground; a directional effect faces the way `by` faces. The
 *  command names `by` (its index): an effect that wraps a sprite plays on that fighter. */
function playFx(bout, T, x, height, by, out, cue = true) {
  const key = ++bout.key;
  const opts = T.tune.directional && by.face < 0 ? { ...T.opts, flip: true } : { ...T.opts };
  out.push({ type: "play", key, fx: T.fx, x: Math.round(x), y: Math.round(bout.ground - height), opts, by: by.index });
  T.keys.push(key);
  T.firstPlay ??= bout.time;
  if (cue) T.cueKeys.add(key);
  return key;
}

function moveFx(bout, key, x, height, out) {
  out.push({ type: "move", key, x: Math.round(x), y: Math.round(bout.ground - height) });
}

function stopFx(T, key, out) {
  const i = T.keys.indexOf(key);
  if (i < 0) return;
  T.keys.splice(i, 1);
  out.push({ type: "stop", key });
}

/** The take's blows land at these seconds after the effect starts, unless its cues say when first. */
function expect(bout, T, times, land) {
  T.playT = bout.time;
  T.blows = times.map((at, i) => ({ at, done: false, land: (out) => land(out, i) }));
}

/**
 * f takes a blow of `power` from `by`, pushed toward `dir`: it reels or is thrown, and the screen feels a `hit`.
 * Life is not lost: a bout shows a fight, it does not end one.
 */
function hit(bout, T, by, f, power, dir, out, { freeze = 0, ranged = false, spell = T.tune.spell ?? "seal" } = {}) {
  if (f.body.untouchable()) return false;
  const type = SPELLS[spell]?.type ?? "magic";
  const damage = damageOf(by, f, power, type, false, bout.rng());
  const launch = bout.Body.heavy(power, freeze > 0);
  out.push({ type: "hit", by: by.index, spell, power, launch, ranged, damage, crit: false, share: (damage / f.stats.hp) * 100, x: f.x, y: f.air + f.spec.chest });
  f.body.hit(power, dir, freeze > 0, f.fly);
  if (freeze > 0) f.frozen = freeze;
  f.fly = false;
  f.act = null;
  f.guard = 0;
  T.hits++;
  return true;
}

/** Which way from `a` to `b`, never 0. */
const toward = (a, b) => (b.x >= a.x ? 1 : -1);
/** The height of f's open palm. */
const palm = (f) => f.air + f.spec.chest;

// ---------- the roles ----------

const SCRIPTS = {
  /** The caster strikes the foe down: the effect plays on the foe's own body, in the pose the blow caught it in. */
  death(bout, T, dt, out) {
    const { caster: me, target: foe, tune } = T;
    if (T.phase === "begin") {
      gather(me, tune.spell ?? "seal");
      return go(T, "gather");
    }
    if (T.phase === "gather" && released(me)) {
      playFx(bout, T, foe.x, 0, foe, out, false);
      hit(bout, T, me, foe, tune.power ?? 30, toward(me, foe), out);
      go(T, "rest");
    }
  },

  /** The caster summons it on the foe; the foe sees it late, backs off, and is thrown when it lands. */
  strike(bout, T, dt, out) {
    const { caster: me, target: foe, tune } = T;
    if (T.phase === "begin") {
      gather(me, tune.spell ?? "seal");
      return go(T, "gather");
    }
    if (T.phase === "gather" && released(me)) {
      const at = tune.at ?? "ground";
      const x = at === "palm" ? me.x + me.face * (tune.distance ?? 34) * 0.7 : at === "chest" ? foe.x - me.face * (tune.pull ? 8 : 0) : foe.x;
      const height = at === "ground" ? 0 : at === "palm" ? palm(me) : palm(foe);
      T.shot = { x, height };
      playFx(bout, T, x, height, me, out);
      expect(bout, T, [tune.impact ?? 0], (o) => {
        hit(bout, T, me, foe, tune.power ?? 18, toward(me, foe), o, { freeze: tune.freeze ?? 0 });
        go(T, "after");
      });
      return go(T, "live");
    }
    if (T.phase === "live") {
      const impactAt = tune.impact ?? 0;
      // it notices what gathers over it a moment before it lands and starts to back away, too late to get clear;
      // a collapse drags it back in
      foe.intent.move = impactAt > 0.3 && T.t >= impactAt - 0.2 && T.t < impactAt ? toward(me, foe) : 0;
      if (tune.pull && T.t < (tune.pullUntil ?? impactAt)) {
        const dx = T.shot.x - foe.x;
        foe.x += Math.sign(dx) * Math.min(Math.abs(dx), tune.pull * dt);
      }
      return;
    }
    if (T.phase === "after") {
      foe.intent.move = 0;
      go(T, "rest");
    }
  },

  /** It leaves the caster's palm and flies at the foe, which reels when it is struck. */
  projectile(bout, T, dt, out) {
    const { caster: me, target: foe, tune } = T;
    if (T.phase === "begin") {
      gather(me, tune.spell ?? "bolt");
      return go(T, "gather");
    }
    if (T.phase === "gather" && released(me)) {
      if (tune.orb) {
        const x = me.x + me.face * 10;
        T.shot = { x, key: playFx(bout, T, x, 0, me, out, false), dir: me.face };
      } else {
        const x = me.x + me.face * 8;
        T.shot = { x, height: palm(me), key: playFx(bout, T, x, palm(me), me, out, false), dir: me.face };
      }
      return go(T, "live");
    }
    if (T.phase !== "live") return;
    const s = T.shot;
    let struck;
    if (tune.orb) {
      // the origin drifts at the foe; the ball wanders round it on the effect's own loop
      s.x += Math.sign(foe.x - s.x) * (tune.speed ?? 30) * dt;
      moveFx(bout, s.key, s.x, 0, out);
      const [ox, oy] = orbOffset(T.t);
      struck = overlaps(foe, s.x + ox, -oy, 10) || Math.abs(foe.x - s.x) < 4;
    } else {
      s.x += s.dir * (tune.speed ?? 230) * dt;
      moveFx(bout, s.key, s.x, s.height, out);
      struck = overlaps(foe, s.x, s.height, 3);
    }
    if (struck) {
      stopFx(T, s.key, out);
      // an orb of lightning leaves the foe burning (duel.js SPELLS.orb)
      if (hit(bout, T, me, foe, tune.power ?? 13, s.dir, out, { ranged: true }) && tune.mark === "burn") foe.burn = { t: 2 };
      return go(T, "rest");
    }
    const gone = s.x < bout.left - 40 || s.x > bout.right + 40 || T.t > (tune.life ?? 3);
    if (gone) {
      stopFx(T, s.key, out);
      go(T, "rest");
    }
  },

  /** It pours from the caster's held-out palm; the foe is battered back while it stands in it. */
  stream(bout, T, dt, out) {
    const { caster: me, target: foe, tune } = T;
    if (T.phase === "begin") {
      gather(me, tune.spell ?? "bolt");
      return go(T, "gather");
    }
    if (T.phase === "gather" && released(me)) {
      // the palm stays out while the stream pours
      me.act.hold = true;
      T.shot = { x: me.x + me.face * 8, height: palm(me), key: playFx(bout, T, me.x + me.face * 8, palm(me), me, out, false), next: 0.15 };
      return go(T, "live");
    }
    if (T.phase !== "live") return;
    const s = T.shot;
    const reach = T.opts.length ?? tune.length ?? 118;
    if (T.t >= s.next) {
      s.next += tune.every ?? 0.45;
      if (Math.abs(foe.x - s.x) <= reach + foe.spec.halfWidth) hit(bout, T, me, foe, tune.power ?? 4, me.face, out, { ranged: true });
    }
    if (T.t >= (tune.life ?? 1.8)) {
      stopFx(T, s.key, out);
      me.act = { spell: me.act?.spell ?? "bolt", phase: "recover", t: 0 };
      go(T, "rest");
    }
  },

  /** A shell forms over the caster; the foe casts at it, and each blow breaks against it. */
  ward(bout, T, dt, out) {
    const { caster: me, target: foe, tune } = T;
    const blows = tune.blows ?? [0.6];
    if (T.phase === "begin") {
      gather(me, tune.spell ?? "ward");
      return go(T, "gather");
    }
    if (T.phase === "gather" && released(me)) {
      playFx(bout, T, me.x, me.air, me, out);
      T.step = 0;
      expect(bout, T, blows, (o) => {
        const dir = toward(foe, me);
        me.rang = 0.22;
        me.vx += dir * 25;
        o.push({ type: "streak", by: foe.index, x0: foe.x + foe.face * 6, y0: palm(foe), x1: me.x - dir * 14, y1: palm(me) + 8 });
        o.push({ type: "block", by: me.index, x: me.x, y: palm(me) });
      });
      return go(T, "live");
    }
    if (T.phase !== "live") return;
    // the foe raises its palm to cast each blow so that it leaves the hand as it strikes the shell
    const windup = SPELLS.bolt.windup;
    while (T.step < blows.length && T.t >= blows[T.step] - windup) {
      gather(foe, "bolt");
      T.step++;
    }
    if (T.t >= blows[blows.length - 1] + GRACE + 0.3 && T.blows.every((b) => b.done)) go(T, "rest");
  },

  /** It lights the caster up; then the caster steps in and lands a stronger blow, the light still on it. */
  buff(bout, T, dt, out) {
    const { caster: me, target: foe, tune } = T;
    if (T.phase === "begin") {
      gather(me, tune.spell ?? "ward");
      return go(T, "gather");
    }
    if (T.phase === "gather" && released(me)) {
      T.shot = { key: playFx(bout, T, me.x, me.air, me, out) };
      expect(bout, T, [tune.impact ?? 0.3], () => {
        me.flash = me.spec.flash;
      });
      return go(T, "live");
    }
    if (T.shot && T.phase !== "rest") moveFx(bout, T.shot.key, me.x, me.air, out);
    if (T.phase === "live" && T.t >= (tune.impact ?? 0.3) + 0.45) go(T, "close");
    if (T.phase === "close") {
      const room = SPELLS.slash.range - 4;
      if (Math.abs(foe.x - me.x) > room && T.t < 2) {
        me.intent.move = toward(me, foe);
      } else {
        me.intent.move = 0;
        gather(me, "slash");
        go(T, "swing");
      }
    }
    if (T.phase === "swing" && released(me)) {
      hit(bout, T, me, foe, tune.power ?? 16, toward(me, foe), out, { spell: "slash" });
      go(T, "rest");
    }
  },

  /** It lies under the caster while it gathers; then the gathered qi strikes the foe. */
  "ground-self"(bout, T, dt, out) {
    const { caster: me, target: foe, tune } = T;
    if (T.phase === "begin") {
      T.shot = { key: playFx(bout, T, me.x, 0, me, out, false) };
      gather(me, tune.spell ?? "void");
      return go(T, "gather");
    }
    if (T.phase === "gather" && released(me)) {
      out.push({ type: "streak", by: me.index, x0: me.x + me.face * 6, y0: palm(me), x1: foe.x, y1: palm(foe) });
      hit(bout, T, me, foe, tune.power ?? 18, toward(me, foe), out);
      return go(T, "after");
    }
    if (T.phase === "after" && T.t >= 0.4) {
      if (T.tune.loop) stopFx(T, T.shot.key, out);
      go(T, "rest");
    }
  },

  /** It is lit under the foe: it burns while the foe stands in it, then flares and throws the foe out. */
  "ground-foe"(bout, T, dt, out) {
    const { caster: me, target: foe, tune } = T;
    if (T.phase === "begin") {
      gather(me, tune.spell ?? "frostfire");
      return go(T, "gather");
    }
    if (T.phase === "gather" && released(me)) {
      out.push({ type: "streak", by: me.index, x0: me.x + me.face * 6, y0: palm(me), x1: foe.x, y1: 4 });
      T.shot = { x: foe.x, key: playFx(bout, T, foe.x, 0, me, out, false), next: 0.5 };
      return go(T, "live");
    }
    if (T.phase !== "live") return;
    const s = T.shot;
    const dx = s.x - foe.x;
    // caught in it: a vortex holds it in, frost slows it to nothing, and what hurts it hangs on it as a mark
    if (tune.pull) foe.x += Math.sign(dx) * Math.min(Math.abs(dx), tune.pull * dt);
    if (tune.chill) foe.chilled = 0.1;
    foe.intent.move = 0;
    if (T.t >= s.next) {
      s.next += 0.5;
      T.ticks++;
      const damage = damageOf(me, foe, tune.power ?? 8, "magic", false, bout.rng());
      if (tune.mark === "burn") foe.burn = { t: 0.6 };
      else if (tune.mark) foe[tune.mark] = 0.6;
      out.push({ type: "tick", by: me.index, damage, share: (damage / foe.stats.hp) * 100, x: foe.x, y: palm(foe) });
    }
    if (T.t >= (tune.life ?? 2.4)) {
      hit(bout, T, me, foe, (tune.power ?? 8) * 2, toward(me, foe), out);
      stopFx(T, s.key, out);
      go(T, "rest");
    }
  },

  /** Both swing at once; the blades meet in it and throw each other back. */
  clash(bout, T, dt, out) {
    const { caster: me, target: foe } = T;
    if (T.phase === "begin") {
      gather(me, "slash");
      gather(foe, "slash");
      return go(T, "swing");
    }
    if (T.phase === "swing" && released(me)) {
      released(foe);
      const x = (me.x + foe.x) / 2;
      const chest = palm(me);
      // the sparks fly `height` over the effect's origin: they meet at chest height
      const height = T.opts.height ?? T.tune.height ?? 30;
      playFx(bout, T, x, chest - height, me, out, false);
      out.push({ type: "clash", x, y: chest });
      for (const f of [me, foe]) f.vx += Math.sign(f.x - x || (f === me ? -1 : 1)) * KNOCKBACK;
      go(T, "rest");
    }
  },

  /** It carries the caster up and over to the foe, which it dives onto, sword first. */
  flight(bout, T, dt, out) {
    const { caster: me, target: foe, tune } = T;
    const back = () => [me.x - me.face * 3, me.air + me.spec.chest - 2];
    if (T.phase === "begin") {
      me.fly = true;
      me.hover = 44;
      const [x, h] = back();
      T.shot = { key: playFx(bout, T, x, h, me, out, false) };
      return go(T, "fly");
    }
    if (T.phase === "fly") {
      const [x, h] = back();
      moveFx(bout, T.shot.key, x, h, out);
      // up, over to just short of the foe, then down onto it
      const dx = foe.x - me.face * 12 - me.x;
      const over = Math.abs(dx) <= 2;
      me.intent.move = T.t > 0.3 && !over ? Math.sign(dx) : 0;
      if (((T.t >= (tune.life ?? 1.4) && over) || T.t >= (tune.life ?? 1.4) + 2) && me.air > 20) {
        me.intent.move = 0;
        stopFx(T, T.shot.key, out);
        me.fly = false;
        gather(me, tune.spell ?? "dive");
        go(T, "dive");
      }
      return;
    }
    if (T.phase === "dive" && released(me)) {
      me.vx += me.face * 50;
      hit(bout, T, me, foe, tune.power ?? 11, toward(me, foe), out, { spell: "dive" });
      out.push({ type: "land", x: me.x, strength: 0.6 });
      go(T, "rest");
    }
  },

  /** The caster vanishes in it, comes out behind the foe and cuts it, then vanishes back to its place. */
  blink(bout, T, dt, out) {
    const { caster: me, target: foe, tune } = T;
    const home = T.spots[me.index];
    if (T.phase === "begin") {
      gather(me, tune.spell ?? "step");
      return go(T, "gather");
    }
    if (T.phase === "gather" && released(me)) {
      const side = toward(me, foe);
      playFx(bout, T, me.x, me.air, me, out, false);
      me.x = clamp(foe.x + side * 18, bout.left + me.spec.halfWidth, bout.right - me.spec.halfWidth);
      playFx(bout, T, me.x, me.air, me, out, false);
      me.act = null;
      return go(T, "behind");
    }
    if (T.phase === "behind" && T.t >= 0.12 && me.act === null && !T.cut) {
      T.cut = true;
      gather(me, "slash");
    }
    if (T.phase === "behind" && T.cut && released(me)) {
      hit(bout, T, me, foe, tune.power ?? 15, toward(me, foe), out, { spell: "slash" });
      return go(T, "back");
    }
    if (T.phase === "back" && T.t >= 0.5 && me.act === null) {
      playFx(bout, T, me.x, me.air, me, out, false);
      me.x = home;
      playFx(bout, T, me.x, me.air, me, out, false);
      go(T, "rest");
    }
  },

  /** The two spar beside it: a cut turned aside by a guard, then a cut that lands. */
  ambient(bout, T, dt, out) {
    const { caster: me, target: foe } = T;
    // a backdrop that plays once stays for its whole length; one that loops, for a few seconds of the spar
    const hold = T.tune.loop ? 3.5 : Math.min(T.tune.duration ?? 3, TAKE_MAX - 2);
    if (T.phase === "begin") {
      gather(me, "slash");
      foe.guard = 0.5;
      return go(T, "first");
    }
    if (T.phase === "first" && released(me)) {
      const x = (me.x + foe.x) / 2;
      out.push({ type: "clash", x, y: palm(me) });
      for (const f of [me, foe]) f.vx += Math.sign(f.x - x || (f === me ? -1 : 1)) * KNOCKBACK * 0.4;
      return go(T, "answer");
    }
    // the one that guarded steps back in to sword's length and answers
    const reach = SPELLS.slash.range + me.spec.halfWidth - 4;
    if (T.phase === "answer" && T.t >= 0.3 && foe.act === null && !foe.body.reeling()) {
      const close = Math.abs(foe.x - me.x) <= reach;
      foe.intent.move = close ? 0 : toward(foe, me);
      if (close || T.t > 2) {
        gather(foe, "slash");
        go(T, "second");
      }
      return;
    }
    if (T.phase === "second" && released(foe)) {
      hit(bout, T, foe, me, 5, toward(foe, me), out, { spell: "slash" });
      if (T.tune.at === "wipe") ambientProp(bout, T, out);
      return go(T, "linger");
    }
    if (T.phase === "linger" && (T.age >= hold || T.tune.at === "wipe")) go(T, "rest");
  },
};

/** Where a backdrop stands: beside the arena, over the two, across the whole scene, or over the screen. */
function ambientProp(bout, T, out = bout.queued) {
  const span = bout.right - bout.left;
  const mid = (bout.left + bout.right) / 2;
  const at = T.tune.at ?? "side";
  const by = T.caster;
  if (at === "side") playFx(bout, T, bout.left + Math.min(36, span * 0.15), 0, by, out, false);
  else if (at === "above") playFx(bout, T, mid, 44, by, out, false);
  // a weather's ground is 10 pixels over its canvas's foot (landing.js)
  else if (at === "screen") playFx(bout, T, mid, -10, by, out, false);
  // a wipe's origin is its top left corner: centre its canvas on the arena
  else playFx(bout, T, mid - 160, 170, by, out, false);
}

/** Does a point (x, height above ground) touch fighter f, give or take r? (duel.js overlaps) */
function overlaps(f, x, y, r) {
  if (f.body.untouchable()) return false;
  return Math.abs(x - f.x) < f.spec.halfWidth + r && y > f.air - r && y < f.air + (f.duck > 0 ? f.spec.duck : f.spec.body) + r;
}

function clamp(v, lo, hi) {
  return Math.max(lo, Math.min(hi, v));
}
