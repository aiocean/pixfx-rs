// What makes the duel's blows land: the screen's answer to what just happened in the fight. A blow freezes the
// fight for a few frames (hit-stop) and shakes the screen; its damage floats up from where it struck; a knockout
// lingers in slow motion; an ultimate darkens the sky as it is cast. All of it is state over the duel's events and
// real time, so the hero asks it each frame how fast the fight runs and where the screen sits. Blows weigh by the
// share of full life they took (`share`); the numbers show the real amounts.

/** Hit-stop seconds for a blow that takes `share` percent of full life, up to a cap so a combo never stalls the fight. */
const hitStop = (share) => Math.min(0.12, 0.035 + share * 0.004);
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
const DUSK = { depth: 0.55, time: 0.9 };

export function createFeel() {
  return { stop: 0, slow: 0, trauma: 0, dusk: 0, numbers: [], streaks: [] };
}

/** Take in what the duel just did (its `step` commands; the effect ones are ignored). */
export function react(feel, commands) {
  for (const c of commands) {
    if (c.type === "hit") {
      feel.stop = Math.max(feel.stop, hitStop(c.share) * (c.crit ? 1.5 : 1));
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
    } else if (c.type === "clash") {
      feel.stop = Math.max(feel.stop, 0.06);
      feel.trauma = Math.min(1, feel.trauma + 0.25);
    } else if (c.type === "land" || c.type === "bump") {
      feel.trauma = Math.min(1, feel.trauma + 0.15 * c.strength);
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
  if (feel.stop > 0) {
    feel.stop = Math.max(0, feel.stop - dt);
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
