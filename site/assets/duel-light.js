// Light the duel throws on its world: each spell lights the dais, the peaks and the fighters around it in its
// school's colour while it plays, a blow flashes where it lands, and the braziers glow and flicker. Lights are drawn
// at one pixel per game pixel in hard-edged rings, so the glow keeps the scene's pixel-art look, then added over it.

/** How far each effect's light reaches, in game pixels; effects not listed reach this far. */
const REACH = {
  "explosions/plasma-explosion": 70,
  "explosions/implosion": 60,
  "explosions/dirt-explosion": 46,
  "explosions/small-explosion": 40,
  "holy/feather-burst": 54,
  "fire/spirit-flame": 40,
  "arcane/rune-circle": 34,
  "impacts/parry": 30,
  "spawn/poof": 28,
};
const REACH_DEFAULT = 36;
/** A one-shot's light lasts this long; a looping effect's lasts until it stops. */
const FLASH = 0.5;
/** A blow's own flash. */
const HIT = { reach: 30, life: 0.16, color: "#fff4dc" };
/** The braziers' warm light, and how it flickers. */
const BRAZIER = { reach: 64, color: "#ff9a3c", strength: 0.32 };
/** How bright a light is at its heart, 0 to 1, before it fades. */
const STRENGTH = 0.42;
/** The rings a light is drawn in: out to this share of its reach, this share of its brightness. */
const RINGS = [
  [0.3, 1],
  [0.55, 0.6],
  [0.8, 0.3],
  [1, 0.12],
];

export function createLights() {
  return { list: [], buffer: null };
}

/**
 * Take in the duel's commands as the hero plays them: a `play` lights its spot in its palette's bright colour
 * (`isLoop(fx)` says whether the effect runs until stopped), a `stop` puts its light out, a `hit` or a blow a
 * barrier turns aside (`block`) flashes.
 * Commands with no palette are the scene's own (fireflies, braziers) and cast no light here.
 */
export function kindle(lights, commands, ground, isLoop) {
  for (const c of commands) {
    if (c.type === "play" && c.opts?.palette) {
      const ramp = c.opts.palette;
      lights.list.push({
        key: c.key,
        x: c.x,
        y: c.y,
        reach: REACH[c.fx] ?? REACH_DEFAULT,
        color: ramp[Math.floor(ramp.length * 0.7)],
        life: isLoop(c.fx) ? Infinity : FLASH,
        t: 0,
      });
    } else if (c.type === "move") {
      for (const l of lights.list) if (l.key === c.key) Object.assign(l, { x: c.x, y: c.y });
    } else if (c.type === "stop") {
      for (const l of lights.list) if (l.key === c.key) l.life = Math.min(l.life, l.t + 0.2);
    } else if (c.type === "hit" || c.type === "block") {
      lights.list.push({ key: null, x: c.x, y: ground - c.y, reach: HIT.reach, color: HIT.color, life: HIT.life, t: 0 });
    }
  }
}

/** Let `dt` seconds pass: lights fade and go out. */
export function fade(lights, dt) {
  for (const l of lights.list) l.t += dt;
  lights.list = lights.list.filter((l) => l.t < l.life);
}

/** Add the light over what is drawn on `ctx` (`s` canvas pixels per game pixel), with braziers at `lamps`. */
export function drawLights(ctx, lights, w, h, s, lamps, ground, time) {
  if (!lights.buffer || lights.buffer.width !== w || lights.buffer.height !== h) {
    lights.buffer = Object.assign(document.createElement("canvas"), { width: w, height: h });
  }
  const b = lights.buffer.getContext("2d");
  b.globalCompositeOperation = "source-over";
  b.clearRect(0, 0, w, h);
  b.globalCompositeOperation = "lighter";
  for (const [i, x] of lamps.entries()) {
    // two slow waves and a fast one, out of step from lamp to lamp
    const flicker = 0.85 + 0.1 * Math.sin(time * 7 + i * 2) + 0.05 * Math.sin(time * 23 + i);
    glow(b, x, ground - 18, BRAZIER.reach * flicker, BRAZIER.color, BRAZIER.strength * flicker);
  }
  for (const l of lights.list) {
    // in fast, out slow; a loop holds steady until it is stopped
    const k = l.life === Infinity ? Math.min(1, l.t * 8) : Math.min(1, l.t * 12) * (1 - l.t / l.life);
    glow(b, l.x, l.y, l.reach * (0.8 + 0.2 * k), l.color, STRENGTH * k);
  }
  ctx.save();
  ctx.globalCompositeOperation = "lighter";
  ctx.imageSmoothingEnabled = false;
  ctx.drawImage(lights.buffer, 0, 0, w * s, h * s);
  ctx.restore();
}

function glow(b, x, y, reach, color, strength) {
  if (strength <= 0.01 || reach < 1) return;
  // widest ring first; each ring inside adds its share on top
  b.fillStyle = color;
  for (let i = RINGS.length - 1; i >= 0; i--) {
    const [out, bright] = RINGS[i];
    const next = RINGS[i + 1]?.[1] ?? 0;
    b.globalAlpha = strength * (bright - next);
    disc(b, Math.round(x), Math.round(y), Math.round(reach * out));
  }
  b.globalAlpha = 1;
}

function disc(b, cx, cy, r) {
  for (let y = -r; y <= r; y++) {
    const half = Math.round(Math.sqrt(r * r - y * y));
    b.fillRect(cx - half, cy + y, half * 2 + 1, 1);
  }
}
