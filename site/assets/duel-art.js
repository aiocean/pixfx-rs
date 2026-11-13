// How the duel looks around its fighters: the qi streaks and numbers of the fight, and the fight's bars. The
// fighters themselves (their poses, the marks that hang on them, the fire, lightning and frost over them, the dust
// they raise) are drawn by the engine (pixfx-fighter, through the wasm `Sprites`); this module tells it what each
// fighter is doing, in the duel's own terms. Everything is in game pixels times `s`, the canvas scale.
import { lookColors } from "../pkg/pixfx_canvas.js";
import { RAGE, SPELLS } from "./duel.js";

/** Each school's colours the HUD draws in, from the engine's looks (asked once the engine has loaded). */
const LOOKS = {};
const colors = (school) => (LOOKS[school] ??= lookColors(school));

/** How each kind of spell is gathered: a summoning raises both arms, a palm draws back, a prayer joins the hands. */
const GATHER = { zone: "summon", void: "summon", orb: "summon", bolt: "palm", blast: "palm", dash: "lunge", heal: "prayer", ward: "prayer", barrier: "cross", blink: "cross" };

/** What the engine's painter reads off fighter f, jolted `jolt` pixels by the blow that just struck it. */
function appearance(f, jolt) {
  let act = null;
  if (f.act) {
    const spell = SPELLS[f.act.spell];
    const progress = f.act.phase === "windup" ? Math.min(1, f.act.t / spell.windup) : null;
    act = { how: GATHER[spell.kind] ?? "palm", dive: !!spell.dive, progress, long: spell.windup >= 0.3 };
  }
  return {
    x: f.x, air: f.air, pose: f.pose, poseT: f.poseT, face: f.face, vx: f.vx, vy: f.vy, out: f.hp <= 0, flash: f.flash > 0, untouchable: f.body.untouchable(),
    frozen: f.frozen > 0, shocked: f.shocked > 0, hitT: f.hitT, hitPower: f.hitPower, dodged: f.dodged, jolt, act,
  };
}

/** What hangs on fighter f: a ward's shell, a barrier, a mark for each effect on it, fury, and fire, lightning or
 *  frost over its body. None once it is knocked out. */
function marks(f, time) {
  if (f.hp <= 0) return null;
  return {
    shield: f.shield > 0, barrier: f.barrier > 0, rang: f.rang > 0, burning: !!f.burn, defDown: f.defDown > 0,
    silence: f.silence > 0, chilled: f.chilled > 0, fury: f.fury, poseT: f.poseT, dome: f.school === "thunder" ? "banded" : "faceted",
    burn: !!f.burn, shock: f.shocked > 0, ice: f.frozen > 0, t: time, seed: f.index + 1,
  };
}

/** The two fighters and the dust of the fight (`feel`), feet on row `ground`; `jolts` from `feel.jolts()`. */
export function drawFighters(sprites, duel, feel, ground, jolts) {
  sprites.begin();
  for (const f of duel.fighters) sprites.fighter(f.school, appearance(f, jolts[f.index]), ground, marks(f, duel.time));
  sprites.dust(feel, ground);
  sprites.end();
}

/**
 * Each qi streak (from the engine's `Feel.streaks()`): Thunder's a crooked line of lightning, Frost's a straight shimmer
 * of ice, from the caster's hand to where the spell lands.
 */
export function drawStreaks(ctx, list, duel, s) {
  for (const k of list) {
    const school = duel.fighters[k.by].school;
    const c = colors(school);
    const x0 = k.x0;
    const y0 = duel.ground - k.y0;
    const x1 = k.x1;
    const y1 = duel.ground - k.y1;
    const n = Math.max(2, Math.round(Math.hypot(x1 - x0, y1 - y0) / 6));
    const pts = [];
    for (let i = 0; i <= n; i++) {
      const u = i / n;
      // lightning jags, ice runs true
      const jag = school === "thunder" && i > 0 && i < n ? Math.round(Math.sin(i * 12.9898 + k.x0 * 78.233) * 3) : 0;
      pts.push([Math.round(x0 + (x1 - x0) * u), Math.round(y0 + (y1 - y0) * u) + jag]);
    }
    ctx.globalAlpha = Math.min(1, k.fade * 1.6);
    for (let i = 0; i < n; i++) {
      plot(ctx, pts[i], pts[i + 1], s, c.glow);
      if (school === "frost" && i % 2 === 0) {
        ctx.fillStyle = "#ffffff";
        ctx.fillRect(pts[i][0] * s, (pts[i][1] - 1) * s, s, s);
      }
    }
    ctx.globalAlpha = Math.min(1, k.fade * 1.2);
    for (let i = 0; i < n; i++) plot(ctx, pts[i], pts[i + 1], s, "#ffffff", true);
  }
  ctx.globalAlpha = 1;
}

/** A pixel line between two points; `thin` draws every other pixel, for a bright core over the glow. */
function plot(ctx, [x0, y0], [x1, y1], s, color, thin = false) {
  ctx.fillStyle = color;
  const n = Math.max(Math.abs(x1 - x0), Math.abs(y1 - y0), 1);
  for (let i = 0; i <= n; i += thin ? 2 : 1) {
    ctx.fillRect(Math.round(x0 + ((x1 - x0) * i) / n) * s, Math.round(y0 + ((y1 - y0) * i) / n) * s, s, s);
  }
}

/** How each kind of number looks: its size in game pixels and colour (null: the striker's glow). */
const NUMBER = {
  hit: { size: 9, color: null },
  crit: { size: 13, color: "#ff6a3d" },
  tick: { size: 7, color: "#ff9a3a" },
  heal: { size: 8, color: "#8dffa8" },
  ward: { size: 8, color: "#a6ecff" },
  dodge: { size: 9, color: "#b8b4c8" },
};

/** The numbers over the fighters (from the engine's `Feel.numbers()`): damage, burns, heals, wards and dodges. */
export function drawNumbers(ctx, list, duel, s) {
  ctx.textAlign = "center";
  ctx.textBaseline = "alphabetic";
  for (const n of list) {
    const look = NUMBER[n.kind] ?? NUMBER.hit;
    ctx.font = `700 ${look.size * s}px "Pixelify Sans", monospace`;
    const x = Math.round(n.x) * s;
    const y = Math.round(duel.ground - n.y) * s;
    ctx.globalAlpha = n.alpha;
    // a dark outline keeps it readable over the brightest blast
    ctx.fillStyle = "#13101b";
    for (const [ox, oy] of [[-1, 0], [1, 0], [0, -1], [0, 1], [1, 1]]) ctx.fillText(n.text, x + ox * s, y + oy * s);
    ctx.fillStyle = look.color ?? colors(duel.fighters[n.by].school).glow;
    ctx.fillText(n.text, x, y);
  }
  ctx.globalAlpha = 1;
}

// ---------- the bars ----------

const BRONZE = { light: "#f0cf7a", mid: "#b8893c", dark: "#6a4a1c", deep: "#2a1c0e" };
const BAR = { back: "#160f1c", lost: "#5a1a14", trail: "#fff0d0", qi: "#7fb4ff", qiDark: "#2c4a80", beadOff: "#3a2c1a" };
/** Life each bar last showed, so a blow's loss drains away behind the bar instead of vanishing. */
const shown = new WeakMap();

/** How tall the fight's bars are, in game pixels: its medallions are this wide and tall. */
export const BARS_TALL = 26;

/**
 * The fight's bars, with no words: for each fighter a bronze medallion with its face, then its life (what a blow
 * just took drains away behind it), its qi under that, and its rage as beads that blaze once full (its ultimate
 * is ready). Between the two, a bronze plaque with a diamond for each round won. `top` is the bars' top edge in
 * game pixels.
 */
export function drawBars(ctx, sprites, duel, s, top) {
  const faces = [];
  const span = duel.right - duel.left;
  const medal = BARS_TALL;
  // a narrow arena hangs the plaque under the bars, between the rage beads, so the bars get its room
  const narrow = span < 260;
  const width = Math.max(30, Math.min(120, Math.floor(span / 2) - medal - (narrow ? 5 : 26)));
  const box = (x, y, w, h, col, a = 1) => {
    ctx.globalAlpha = a;
    ctx.fillStyle = col;
    ctx.fillRect(x * s, y * s, w * s, h * s);
    ctx.globalAlpha = 1;
  };
  for (const f of duel.fighters) {
    const right = f.index === 1;
    const c = colors(f.school);
    const mx = right ? duel.right - medal : duel.left;
    medallion(f, mx, top, medal, box, c, faces);
    const left = right ? mx - 3 - width : mx + medal + 3;
    // each bar grows from its medallion outward
    const from = (w) => (right ? left + width - w : left);
    // life: a bronze frame with end caps
    box(left - 2, top + 2, width + 4, 9, BRONZE.deep);
    box(left - 1, top + 3, width + 2, 7, BRONZE.mid);
    box(left - 1, top + 3, width + 2, 1, BRONZE.light);
    box(left, top + 4, width, 5, BAR.back);
    const hp = Math.round((width * f.hp) / 100);
    const last = Math.max(f.hp, Math.min(100, shown.get(f) ?? f.hp));
    shown.set(f, last - Math.max(0.4, (last - f.hp) * 0.06));
    const trail = Math.round((width * last) / 100);
    box(from(trail), top + 4, trail, 5, BAR.lost);
    box(from(trail), top + 4, trail, 5, BAR.trail, 0.25);
    box(from(hp), top + 4, hp, 5, c.ribbon[1]);
    box(from(hp), top + 4, hp, 2, c.ribbon[0]);
    box(from(hp), top + 4, hp, 1, "#ffffff", 0.35);
    if (f.shield > 0) {
      const w = Math.min(width, Math.round((width * f.shield) / f.stats.hp));
      box(from(w), top + 7, w, 2, "#bff2ff");
    }
    // a notch each tenth
    for (let i = 1; i < 10; i++) box(left + Math.round((width * i) / 10), top + 8, 1, 1, BAR.back, 0.6);
    const cap = (x) => {
      box(x, top + 4, 1, 5, BRONZE.light);
      box(x + (right ? -1 : 1), top + 5, 1, 3, BRONZE.mid);
    };
    cap(right ? left - 2 : left + width + 1);
    // qi
    box(left, top + 11, width, 2, BAR.qiDark);
    const qi = Math.round((width * f.qi) / 100);
    box(from(qi), top + 11, qi, 2, BAR.qi);
    box(from(qi), top + 11, qi, 1, "#c8e0ff");
    // rage beads
    const full = f.rage >= RAGE;
    const blaze = full && Math.sin(duel.time * 12) > 0;
    for (let i = 0; i < RAGE; i++) {
      const x = right ? left + width - 6 - i * 8 : left + i * 8;
      const on = i < f.rage;
      if (full) box(x - 1, top + 14, 8, 7, c.glow, blaze ? 0.35 : 0.15);
      box(x + 1, top + 15, 4, 1, BRONZE.dark);
      box(x, top + 16, 6, 3, BRONZE.dark);
      box(x + 1, top + 19, 4, 1, BRONZE.dark);
      box(x + 1, top + 16, 4, 3, on ? (blaze ? "#ffffff" : c.glow) : BAR.beadOff);
      if (on) box(x + 1, top + 16, 2, 1, "#ffffff", 0.7);
    }
  }
  // the plaque in the middle: a diamond per round won, each side in its fighter's colour
  const mid = Math.round((duel.left + duel.right) / 2);
  top += narrow ? 13 : 0;
  box(mid - 18, top + 2, 36, 9, BRONZE.deep);
  box(mid - 17, top + 3, 34, 7, BRONZE.dark);
  box(mid - 17, top + 3, 34, 1, BRONZE.mid);
  box(mid, top + 4, 1, 5, BRONZE.mid);
  for (const f of duel.fighters) {
    const won = Math.min(3, duel.wins[f.index]);
    for (let i = 0; i < 3; i++) {
      const x = f.index === 0 ? mid - 6 - i * 5 : mid + 4 + i * 5;
      const col = i < won ? colors(f.school).glow : BRONZE.deep;
      box(x + 1, top + 4, 1, 1, col);
      box(x, top + 5, 3, 2, col);
      box(x + 1, top + 7, 1, 1, col);
    }
  }
  sprites.begin();
  for (const m of faces) {
    const look = { ...appearance(m.f, [0, 0]), pose: "idle", face: m.f.index === 0 ? 1 : -1, flash: false, vx: 0, act: null };
    sprites.portrait(m.f.school, look, m.x, m.y, m.size, m.f.hp <= 0 ? 0.35 : 1);
  }
  sprites.end();
}

/** A bronze medallion at (x, top), `size` across, with f's face in it, lit in its colour once its rage is full. */
function medallion(f, x, top, size, box, c, faces) {
  // an octagon: a dark rim, bronze lit from above, a dark field
  const inset = 3;
  for (let y = 0; y < size; y++) {
    const cut = Math.max(0, inset - y, y - (size - 1 - inset));
    box(x + cut, top + y, size - 2 * cut, 1, BRONZE.deep);
    if (y > 0 && y < size - 1) box(x + cut + 1, top + y, size - 2 * cut - 2, 1, y < size / 2 ? BRONZE.light : BRONZE.mid);
    if (y > 1 && y < size - 2) box(x + cut + 2, top + y, size - 2 * cut - 4, 1, "#1a1424");
  }
  box(x + 4, top + 4, size - 8, size - 8, f.rage >= RAGE ? c.glow : "#241a30", f.rage >= RAGE ? 0.35 : 1);
  // the face: the sprite's head at twice its size, facing into the arena
  faces.push({ f, x: x + 3, y: top + 3, size: size - 6 });
}
