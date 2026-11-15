// The duel's first mind, kept as the yardstick its successor (pixfx-mind) is measured against in
// mind-match.test.js. It is the code site/assets/duel.js thought with before the fighters got their brain, moved
// here as it was: its numbers and rules are unchanged, only its state (when it next thinks, the breath after an
// attack, what it has seen land) and its dice live on the mind instead of the fighter and the duel, and it hands
// back a Choice for the duel to carry out instead of carrying it out. The page never loads it.
import { RAGE, SCHOOLS, SPELLS, orbOffset, ready } from "../assets/duel.js";

const REACTION = 0.16;
const THINK = 0.12;
const MIND = { spread: 0.8, risk: 0.6, floor: 0.3, lead: 0.8 };
const PRIOR = 6;
const BARRIER_THREAT = 10;

/** mulberry32, as duel.js. */
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

/** The first mind for fighter `f` of `duel`, in the shape createDuel's `minds` takes. */
export function baselineMind(duel, f) {
  const st = { rng: random(duel.seed * 2 + f.index), think: 0, calm: 0, read: { tried: {}, landed: {} }, hold: 0 };
  return {
    posture: "baseline",
    think(duel, me, foe, dt) {
      st.calm = Math.max(0, st.calm - dt);
      st.hold = Math.max(0, st.hold - dt);
      if (st.hold > 0) return null;
      return mind(duel, me, foe, dt, st);
    },
    tell(outcome) {
      if (outcome.type === "landed") st.read.landed[outcome.skill] = (st.read.landed[outcome.skill] ?? 0) + 1;
    },
    reset() {
      Object.assign(st, { think: 0.4 + st.rng() * 0.3, calm: 0, read: { tried: {}, landed: {} }, hold: 0 });
    },
    hold(seconds) {
      st.hold = seconds;
    },
    explain() {},
  };
}

/** The option it chose as a Choice; an attack takes its breath and counts as tried, as `begin` did. */
function choiceOf(me, best, st) {
  switch (best.spell) {
    case "move": return { type: "move", dir: best.move };
    case "jump": return { type: "jump" };
    case "guard": return { type: "guard", hold: best.hold };
    case "duck": return { type: "duck", hold: best.hold };
    case "fly": return { type: "fly", height: best.hover };
    case "meditate": return { type: "meditate" };
    case "wait": return { type: "wait" };
    default:
      if (SPELLS[best.spell].damage) {
        st.calm = SCHOOLS[me.school].tempo * (1 - MIND.spread / 2 + MIND.spread * st.rng());
        st.read.tried[best.spell] = (st.read.tried[best.spell] ?? 0) + 1;
      }
      return { type: "cast", skill: best.spell, target: best.target ?? null };
  }
}

/**
 * A fighter thinks every THINK seconds while it is free: it sees only what has been on screen for REACTION seconds,
 * scores everything it could do (get out of the way, attack with each ready spell, move to its range, meditate),
 * and does the best, with a little noise so no two rounds play the same.
 */
function mind(duel, me, foe, dt, st) {
  st.think -= dt;
  if (st.think > 0 || me.act || me.body.reeling() || me.squat > 0 || me.duck > 0 || me.frozen > 0 || me.shocked > 0 || me.hp <= 0) return;
  st.think = THINK * (0.8 + st.rng() * 0.4);
  const threats = [...dangers(duel, me), ...blows(duel, me, foe)];
  const options = [...threats, ...attacks(duel, me, foe, st), ...positions(duel, me, foe, st)];
  // something is about to land: raise the barrier, which answers any of it
  const barrier = me.spells.find((name) => SPELLS[name].kind === "barrier");
  const threat = Math.max(0, ...threats.map((o) => o.score));
  if (barrier && me.barrier === 0 && ready(me, barrier) && threat > BARRIER_THREAT) options.push({ spell: barrier, score: threat * 1.25 });
  let best = null;
  for (const o of options) {
    o.score *= 0.9 + st.rng() * 0.2;
    if (!best || o.score > best.score) best = o;
  }
  return choiceOf(me, best, st);
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
function attacks(duel, me, foe, st) {
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
    if (st.calm > 0 && opening < lands) return 0;
    // a raised barrier turns everything but an ultimate aside: wait it out
    if (foe.barrier > 0 && !s.ultimate) chance *= 0.05;
    // what it has seen this attack do against this foe
    const tried = st.read.tried[name] ?? 0;
    chance = (chance * PRIOR + (st.read.landed[name] ?? 0)) / (PRIOR + tried);
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
function positions(duel, me, foe, st) {
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
  if ((st.calm > 0 || foe.barrier > 0) && Math.abs(d) < reach && Math.abs(foe.air - me.air) < me.spec.chest && !foe.body.reeling()) {
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
  if (me.air === 0 && !me.fly && Math.abs(d) > SPELLS.slash.range + 10 && st.rng() < 0.5) out.push({ spell: "jump", score: 1.3 });
  const threatened = duel.shots.some((s) => s.owner !== me.index);
  if (me.qi < 45 && Math.abs(d) > 70 && !threatened && !foe.act && me.air === 0 && !me.fly) out.push({ spell: "meditate", score: 2.2 - me.qi / 40 });
  return out;
}

function atWall(duel, f) {
  return f.x <= duel.left + f.spec.halfWidth + 2 || f.x >= duel.right - f.spec.halfWidth - 2;
}

function orbAt(s) {
  const [ox, oy] = orbOffset(s.t);
  return [s.x + ox, oy];
}

/** Does a point (x, height above ground) touch fighter f, give or take r? */
/** How high a zone strikes: what hangs in the air (a collapse) at the foe's chest, what falls (an icicle) at the floor. */
function clamp(v, lo, hi) {
  return Math.max(lo, Math.min(hi, v));
}
