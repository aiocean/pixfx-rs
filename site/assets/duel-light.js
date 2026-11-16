// Light the duel throws on its world: each spell lights the dais, the peaks and the fighters around it in its
// school's colour while it plays, a blow flashes where it lands, and the braziers glow and flicker. The lights
// themselves (how they fade, the hard-edged rings that keep the pixel-art look, adding them over the scene) are
// pixfx-light's, through the engine (`stepLights`, `putOutLight`, `Lighting`); this file is the duel's tables and
// how its commands change the list of lights the hero keeps.

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
  // a trail rides on a fighter: a faint glow, or the fighter is lit white all the while
  "projectiles/trail": 12,
};
const REACH_DEFAULT = 36;
/** A one-shot's light lasts this long; a looping effect's lasts until it stops, then goes out over `OUT`. */
const FLASH = 0.5;
const OUT = 0.2;
/** A blow's own flash. */
const HIT = { reach: 30, life: 0.16, color: "#fff4dc" };
/** The braziers' warm light, and how it flickers. */
const BRAZIER = { reach: 64, color: "#ff9a3c", strength: 0.32 };
/** How bright a light is at its heart, 0 to 1, before it fades. */
const STRENGTH = 0.42;

/**
 * The duel's lights after its commands, as the hero plays them: a `play` lights its spot in its palette's bright
 * colour (`isLoop(fx)` says whether the effect runs until stopped), a `move` carries its light, a `stop` puts its
 * light out (`putOut(light, within)`, the engine's `putOutLight`), a `hit` or a blow a barrier turns aside (`block`)
 * flashes. Commands with no palette are the scene's own (fireflies, braziers) and cast no light here.
 */
export function kindle(lights, commands, ground, isLoop, putOut) {
  let out = lights;
  for (const c of commands) {
    if (c.type === "play" && c.opts?.palette) {
      const ramp = c.opts.palette;
      const reach = REACH[c.fx] ?? REACH_DEFAULT;
      const color = ramp[Math.floor(ramp.length * 0.7)];
      out = [...out, { key: c.key, x: c.x, y: c.y, reach, color, strength: STRENGTH, life: isLoop(c.fx) ? null : FLASH }];
    } else if (c.type === "move") {
      out = out.map((l) => (l.key === c.key ? { ...l, x: c.x, y: c.y } : l));
    } else if (c.type === "stop") {
      out = out.map((l) => (l.key === c.key ? putOut(l, OUT) : l));
    } else if (c.type === "hit" || c.type === "block") {
      out = [...out, { key: null, x: c.x, y: ground - c.y, reach: HIT.reach, color: HIT.color, strength: STRENGTH, life: HIT.life }];
    }
  }
  return out;
}

/** The braziers' glow at `time`, one per lamp at `lamps` (x), for `Lighting.paint`. */
export function lampGlows(lamps, ground, time) {
  return lamps.map((x, i) => {
    // two slow waves and a fast one, out of step from lamp to lamp
    const flicker = 0.85 + 0.1 * Math.sin(time * 7 + i * 2) + 0.05 * Math.sin(time * 23 + i);
    return { x, y: ground - 18, reach: BRAZIER.reach * flicker, color: BRAZIER.color, strength: BRAZIER.strength * flicker };
  });
}
