// What makes the duel's blows land: the screen's answer to what just happened in the fight. A blow freezes the
// fight for a few frames (hit-stop) while the one it struck shudders, and shakes the screen; its damage floats up
// from where it struck; a body hitting the floor kicks up dust; a knockout lingers in slow motion; an ultimate
// darkens the sky as it is cast. All of it is state over the duel's events and real time, so the hero asks it each
// frame how fast the fight runs and where the screen sits. The shake weighs a blow by the share of full life it
// took (`share`); the numbers show the real amounts.
import { SPELLS } from "./duel.js";

/**
 * Hit-stop, in frames at 60 a second, the way fighting games and Sakurai's Smash count it: both fighters freeze
 * together, longer the heavier the blow (Street Fighter 6 gives its mediums 11 frames, its heavies 13); a crit
 * holds longer, a spell that struck from afar shorter, and no blow freezes the fight past a cap.
 */
const STOP = { light: 6, medium: 8, launch: 11, crit: 3, ranged: -2, cap: 13 };
const RANGED = new Set(["bolt", "orb"]);
function hitStop(c) {
  const kind = SPELLS[c.spell]?.kind;
  let frames = c.launch ? STOP.launch : c.power >= 9 ? STOP.medium : STOP.light;
  if (c.crit) frames += STOP.crit;
  if (RANGED.has(kind)) frames += STOP.ranged;
  return Math.min(STOP.cap, frames) / 60;
}
/** The one struck shudders through the hit-stop: this many pixels at first, dying away by its end. */
const JOLT = 2;
/** Dust a body raises where it hits the floor: how long it hangs, and how far it spreads. */
export const DUST = { life: 0.4, spread: 12 };
/** Trauma, 0 to 1, drains this fast per second; the shake is trauma squared times this many game pixels. */
const TRAUMA_DRAIN = 1.6;
const SHAKE = 3;
/** A knockout: a longer stop, then the fight runs at this speed for this long. */
const KO = { stop: 0.25, speed: 0.35, slow: 1.2 };
/** Damage numbers rise this many pixels over their life. */
const NUMBER = { rise: 14, life: 0.8 };
/** A streak of qi from a caster to where its spell lands shows this long. */
const STREAK = 0.28;
/** An ultimate dims the sky this much, fading over this many seconds. */
export const DUSK = { depth: 0.55, time: 0.9 };

export function createFeel() {
  return { stop: 0, slow: 0, trauma: 0, dusk: 0, numbers: [], streaks: [], dust: [], jolt: null, tick: 0 };
}

/** Take in what the duel just did (its `step` commands; the effect ones are ignored). */
export function react(feel, commands) {
  for (const c of commands) {
    if (c.type === "hit") {
      const stop = hitStop(c);
      feel.stop = Math.max(feel.stop, stop);
      // grounded, it shudders side to side; in the air, up and down (Sakurai)
      feel.jolt = { who: 1 - c.by, left: stop, full: stop, air: c.y > 24 };
      feel.trauma = Math.min(1, feel.trauma + c.share / 25 + (c.crit ? 0.2 : 0));
      number(feel, c, c.crit ? "crit" : "hit", String(c.damage));
    } else if (c.type === "tick") {
      number(feel, c, "tick", String(c.damage));
    } else if (c.type === "heal") {
      number(feel, c, "heal", `+${c.amount}`);
    } else if (c.type === "ward") {
      number(feel, c, "ward", `+${c.amount}`);
    } else if (c.type === "dodge") {
      // a dodge shows as a dash where the blow would have struck
      number(feel, { ...c, by: null }, "dodge", "-");
    } else if (c.type === "unyielding") {
      feel.stop = Math.max(feel.stop, 0.18);
      feel.trauma = Math.min(1, feel.trauma + 0.4);
    } else if (c.type === "block") {
      // a barrier rings: a short stop, a little shake
      feel.stop = Math.max(feel.stop, 0.05);
      feel.trauma = Math.min(1, feel.trauma + 0.15);
    } else if (c.type === "clash") {
      feel.stop = Math.max(feel.stop, 0.06);
      feel.trauma = Math.min(1, feel.trauma + 0.25);
    } else if (c.type === "land" || c.type === "bump") {
      feel.trauma = Math.min(1, feel.trauma + 0.15 * c.strength);
      if (c.type === "land" && c.strength > 0.3) feel.dust.push({ x: c.x, strength: c.strength, t: 0 });
    } else if (c.type === "ko") {
      feel.stop = Math.max(feel.stop, KO.stop);
      feel.slow = KO.slow;
      feel.trauma = Math.min(1, feel.trauma + 0.8);
    } else if (c.type === "streak") {
      feel.streaks.push({ ...c, t: 0 });
    } else if (c.type === "ultimate") {
      feel.stop = Math.max(feel.stop, 0.1);
      feel.dusk = DUSK.time;
    }
  }
}

function number(feel, c, kind, text) {
  // numbers from one spot step aside so a barrage stays readable
  const near = feel.numbers.filter((n) => Math.abs(n.x - c.x) < 12 && n.t < 0.3).length;
  feel.numbers.push({ x: c.x + (near % 2 ? 7 : -7) * Math.min(near, 1), y: c.y + near * 6, text, by: c.by, kind, t: 0 });
}

/**
 * Let `dt` real seconds pass. Returns how many seconds the fight should run for it: none during a hit-stop, a
 * fraction in a knockout's slow motion.
 */
export function advance(feel, dt) {
  feel.trauma = Math.max(0, feel.trauma - TRAUMA_DRAIN * dt);
  feel.dusk = Math.max(0, feel.dusk - dt);
  for (const n of feel.numbers) n.t += dt;
  feel.numbers = feel.numbers.filter((n) => n.t < NUMBER.life);
  for (const k of feel.streaks) k.t += dt;
  feel.streaks = feel.streaks.filter((k) => k.t < STREAK);
  for (const d of feel.dust) d.t += dt;
  feel.dust = feel.dust.filter((d) => d.t < DUST.life);
  feel.tick++;
  if (feel.jolt) {
    feel.jolt.left -= dt;
    if (feel.jolt.left <= 0) feel.jolt = null;
  }
  if (feel.stop > 0) {
    // counted in whole frames: what is left after the last one is rounding, not another frame
    feel.stop = feel.stop - dt > 1e-6 ? feel.stop - dt : 0;
    return 0;
  }
  if (feel.slow > 0) {
    feel.slow = Math.max(0, feel.slow - dt);
    return dt * KO.speed;
  }
  return dt;
}

/** Where the screen sits this frame, in whole game pixels; `rand` gives numbers in [0, 1). */
export function shake(feel, rand) {
  const a = feel.trauma * feel.trauma * SHAKE;
  return [Math.round((rand() * 2 - 1) * a), Math.round((rand() * 2 - 1) * a)];
}

/** How far each fighter is jolted off its place this frame, in whole game pixels: [dx, dy] per fighter index. */
export function jolts(feel) {
  const out = [[0, 0], [0, 0]];
  const j = feel.jolt;
  if (!j) return out;
  // flips side every frame, smaller as the stop runs out
  const a = Math.round(JOLT * (j.left / j.full) + 0.4) * (feel.tick % 2 ? 1 : -1);
  out[j.who] = j.air ? [0, a] : [a, 0];
  return out;
}

/** Each puff of dust: where, how strong, and how far through its life (0 new, 1 gone). */
export function dust(feel) {
  return feel.dust.map((d) => ({ x: d.x, strength: d.strength, k: d.t / DUST.life }));
}

/** How dark the sky is, 0 to DUSK.depth. */
export function dusk(feel) {
  const k = feel.dusk / DUSK.time;
  // in fast, out slow
  return DUSK.depth * Math.min(1, k * 4) * Math.min(1, k * 1.6);
}

/** Each number: where it is (height above the ground), how faded, who caused it, and its kind (hit, crit, tick, heal,
 *  ward or dodge). */
export function numbers(feel) {
  return feel.numbers.map((n) => ({
    x: n.x,
    y: n.y + 16 + NUMBER.rise * Math.sqrt(n.t / NUMBER.life),
    alpha: Math.min(1, 2.5 * (1 - n.t / NUMBER.life)),
    text: n.text,
    by: n.by,
    kind: n.kind,
  }));
}

/** Each qi streak: from where, to where (heights up from the ground), whose, and how faded (1 new, 0 gone). */
export function streaks(feel) {
  return feel.streaks.map((k) => ({ x0: k.x0, y0: k.y0, x1: k.x1, y1: k.y1, by: k.by, fade: 1 - k.t / STREAK }));
}
