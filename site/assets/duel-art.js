// How the duel looks: the two cultivators as pixel sprites, one pose per thing they can be doing, the marks that
// hang on them, the streaks and numbers of the fight, and the fight's bars. Everything is drawn in game pixels
// times `s`, the canvas scale.
//
// A fighter is painted into a small buffer at one pixel per game pixel, outlined there (dark, or in its school's
// glow while it gathers a spell, so the caster is plain to see), then drawn scaled onto the scene.
import { RAGE } from "./duel.js";

/** Each school's colours, three tones where a part is shaded. */
const LOOKS = {
  thunder: {
    robe: ["#f4efe2", "#d6ccb6", "#a39680"], inner: ["#b3261c", "#7a1912"], trim: ["#f0bc3a", "#a8741a"],
    hair: ["#3a3048", "#1a1420"], skin: ["#ecc9a2", "#c99f7a"], boot: "#2a1a14", blade: ["#ffffff", "#fff6c4", "#c8b880"],
    jade: "#5fd08a", ribbon: ["#e0443a", "#a8241c"], glow: "#ffe066", outline: "#1a1220",
  },
  frost: {
    robe: ["#4a7ed0", "#2f5fae", "#1e3f80"], inner: ["#e6f0ff", "#a8c0e4"], trim: ["#bff2ff", "#5fa8d8"],
    hair: ["#ffffff", "#c4d2ee"], skin: ["#f2dcc2", "#d2b294"], boot: "#141a33", blade: ["#ffffff", "#dcfbff", "#8fc8e8"],
    jade: "#9ff0e0", ribbon: ["#bff2ff", "#7ab8e0"], glow: "#a6ecff", outline: "#0c1024",
  },
};

/** A frozen fighter is drawn in ice; a shocked one flickers with lightning. */
const ICE = { light: "#e8fbff", mid: "#a6ecff", dark: "#5fa8d8" };
const SHOCK = "#fff6a0";

/** The sprite buffer: feet at (32, 60). */
const SPRITE = { w: 64, h: 64, cx: 32, feet: 60 };
let buffer = null;

function spriteBuffer() {
  if (!buffer) {
    const c = Object.assign(document.createElement("canvas"), { width: SPRITE.w, height: SPRITE.h });
    buffer = { c, ctx: c.getContext("2d", { willReadFrequently: true }) };
  }
  return buffer;
}

/**
 * One fighter, feet at (f.x, ground - f.air), facing f.face: about 40 pixels tall, its body the 34 the effects are
 * drawn for (parry's blades meet 30 pixels up), its hair above that.
 */
export function drawFighter(ctx, f, ground, s) {
  // a dodge leaves an afterimage where the blow was aimed
  if (f.dodged > 0 && f.hp > 0) blit(ctx, { ...f, x: f.x + f.face * 7, flash: 1 }, ground, s, f.dodged * 1.5);
  blit(ctx, f, ground, s, 1);
  if (f.hp > 0) drawStatus(ctx, f, ground, s);
}

function blit(ctx, f, ground, s, alpha) {
  const b = spriteBuffer();
  b.ctx.clearRect(0, 0, SPRITE.w, SPRITE.h);
  paint(b.ctx, f);
  const gathering = f.pose === "channel" || f.pose === "cast";
  const glow = gathering ? LOOKS[f.school].glow : null;
  outline(b.ctx, glow && Math.sin(f.poseT * 18) > -0.4 ? glow : LOOKS[f.school].outline);
  ctx.imageSmoothingEnabled = false;
  const x = Math.round(f.x) - SPRITE.cx;
  const y = Math.round(ground - f.air) - SPRITE.feet;
  // a caster's glow spills round it while it gathers
  if (glow) {
    ctx.globalAlpha = alpha * (0.18 + 0.08 * Math.sin(f.poseT * 9));
    ctx.fillStyle = glow;
    ctx.fillRect((x + SPRITE.cx - 12) * s, (y + SPRITE.feet - 40) * s, 25 * s, 40 * s);
  }
  ctx.globalAlpha = alpha;
  ctx.drawImage(b.c, x * s, y * s, SPRITE.w * s, SPRITE.h * s);
  ctx.globalAlpha = 1;
}

/** Ring every painted pixel with `color`: each empty pixel touching a painted one takes it. */
function outline(bctx, color) {
  const img = bctx.getImageData(0, 0, SPRITE.w, SPRITE.h);
  const d = img.data;
  const n = parseInt(color.slice(1), 16);
  const [r, g, b] = [n >> 16, (n >> 8) & 255, n & 255];
  const painted = (x, y) => x >= 0 && y >= 0 && x < SPRITE.w && y < SPRITE.h && d[(y * SPRITE.w + x) * 4 + 3] > 128;
  const ring = [];
  for (let y = 0; y < SPRITE.h; y++) {
    for (let x = 0; x < SPRITE.w; x++) {
      if (d[(y * SPRITE.w + x) * 4 + 3] > 0) continue;
      if (painted(x - 1, y) || painted(x + 1, y) || painted(x, y - 1) || painted(x, y + 1)) ring.push(y * SPRITE.w + x);
    }
  }
  for (const i of ring) {
    d[i * 4] = r;
    d[i * 4 + 1] = g;
    d[i * 4 + 2] = b;
    d[i * 4 + 3] = 255;
  }
  bctx.putImageData(img, 0, 0);
}

/** Paint fighter f's pose into the sprite buffer. */
function paint(bctx, f) {
  const c = LOOKS[f.school];
  const pose = f.pose;
  // the colour a part is painted in: white in a hit's flash, ice when frozen, flickering when shocked
  const tint = (col) => {
    if (f.flash > 0) return "#ffffff";
    if (pose === "frozen") return lum(col) > 0.7 ? ICE.light : lum(col) > 0.4 ? ICE.mid : ICE.dark;
    if (pose === "shocked" && Math.sin(f.poseT * 70) > 0) return SHOCK;
    return col;
  };
  // a rect in the fighter's own frame: dx forward (toward where it faces), dy up from its feet
  const px = (dx, dy, w, h, col) => {
    const x = f.face > 0 ? SPRITE.cx + dx : SPRITE.cx - dx - w + 1;
    bctx.fillStyle = tint(col);
    bctx.fillRect(x, SPRITE.feet - dy - h + 1, w, h);
  };
  const dot = (dx, dy, col) => px(dx, dy, 1, 1, col);
  const line = (x0, y0, x1, y1, col) => {
    const n = Math.max(Math.abs(x1 - x0), Math.abs(y1 - y0), 1);
    for (let i = 0; i <= n; i++) dot(Math.round(x0 + ((x1 - x0) * i) / n), Math.round(y0 + ((y1 - y0) * i) / n), col);
  };
  const t = f.poseT;
  const still = pose === "frozen";
  const bob = still ? 0 : Math.sin(t * 3) > 0.6 ? 1 : 0;
  // cloth streams back from the way it moves, and sways a little when it does not
  const wind = still ? 0 : Math.max(-3, Math.min(3, -(f.vx ?? 0) * f.face * 0.03)) - 1 + Math.round(Math.sin(t * 4));

  if (pose === "down" || pose === "tumble") {
    // lying on its back, robe spread, the sword dropped beside it; thrown through the air the same way
    const up = pose === "tumble" ? 4 : 0;
    px(-15, up, 22, 4, c.robe[1]);
    px(-15, up + 3, 22, 1, c.robe[0]);
    px(-15, up, 22, 1, c.robe[2]);
    px(-6, up + 1, 3, 3, c.inner[0]);
    px(-15, up + 1, 2, 2, c.trim[0]);
    px(7, up, 6, 5, c.skin[0]);
    px(7, up, 6, 1, c.skin[1]);
    px(10, up + 3, 1, 1, c.outline);
    px(12, up - 1, 5, 6, c.hair[1]);
    if (pose === "down") {
      line(-12, -1, 2, -1, c.blade[2]);
      px(3, -1, 1, 1, c.trim[0]);
    }
    return;
  }

  if (pose === "meditate") {
    // cross-legged in the air, hands together before the chest, robe pooled round it
    const up = 6 + bob;
    px(-9, up, 19, 4, c.robe[1]);
    px(-9, up, 19, 1, c.robe[2]);
    px(-9, up + 3, 19, 1, c.trim[0]);
    px(-5, up + 4, 10, 10, c.robe[1]);
    px(-5, up + 4, 2, 10, c.robe[2]);
    px(-1, up + 5, 2, 9, c.inner[0]);
    px(-5, up + 7, 10, 2, c.inner[1]);
    px(-1, up + 9, 3, 3, c.skin[0]);
    head(px, c, f, up + 14, 0, wind);
    return;
  }

  const lean = pose === "hurt" || pose === "shocked" ? -2 : pose === "slash" ? 2 : pose === "cast" ? 1 : 0;
  // knees drawn up in a jump; squashed for a moment on landing
  const tuck = pose === "jump" ? 4 : pose === "land" ? 2 : 0;

  if (pose === "fly") {
    // riding its sword: the blade flat under its feet, a trail of light behind
    line(-11, -2, 11, -2, c.blade[1]);
    line(-11, -1, 9, -1, c.blade[0]);
    px(12, -2, 1, 1, "#ffffff");
    px(-13, -3, 2, 3, c.trim[0]);
    px(-15, -2, 2, 1, c.trim[1]);
    if (Math.sin(t * 20) > -0.3) line(-16 - (Math.round(t * 30) % 3), -2, -14, -2, c.glow);
  }

  // feet
  const stride = pose === "walk" ? (Math.sin(t * 12) > 0 ? 2 : -2) : 0;
  px(-4 + stride, 0, 4, 2, c.boot);
  px(1 - stride, 0, 4, 2, c.boot);
  // the lower robe, flaring to the hem, open at the front over the inner robe
  for (let dy = 2 + tuck; dy <= 15; dy++) {
    const flare = Math.max(0, Math.round((8 - dy) / 2));
    const back = -5 - flare + (dy < 6 ? Math.min(0, wind) : 0);
    const front = 4 + Math.max(0, flare - 1);
    px(back, dy, front - back + 1, 1, c.robe[1]);
    px(back, dy, 2, 1, c.robe[2]);
    px(front - 1, dy, 1, 1, c.robe[0]);
    px(1, dy, 2, 1, c.inner[dy < 8 ? 1 : 0]);
  }
  px(-7 + Math.min(0, wind), 2 + tuck, 13 - Math.min(0, wind), 1, c.trim[0]);
  px(-6, 3 + tuck, 1, 1, c.trim[1]);

  // the body: robe crossed over the inner robe, a sash with a jade pendant and a tail that streams back
  const by = 16 - bob;
  px(-5 + lean, by, 10, 9, c.robe[1]);
  px(-5 + lean, by, 2, 9, c.robe[2]);
  px(3 + lean, by + 1, 1, 8, c.robe[0]);
  line(0 + lean, by + 2, 3 + lean, by + 8, c.inner[0]);
  line(-1 + lean, by + 2, 2 + lean, by + 8, c.trim[0]);
  px(-5 + lean, by, 10, 2, c.inner[1]);
  px(-5 + lean, by + 1, 10, 1, c.inner[0]);
  px(1 + lean, by, 2, 2, c.trim[0]);
  px(2 + lean, by - 3, 1, 3, c.ribbon[1]);
  px(2 + lean, by - 4, 1, 1, c.jade);
  for (let i = 0; i < 6; i++) {
    dot(-6 + lean - i + Math.round(wind * i * 0.3), by + 1 - Math.round(i / 2) + (i > 3 ? Math.round(Math.sin(t * 6 + i)) : 0), c.inner[i < 3 ? 0 : 1]);
  }
  head(px, c, f, by + 9, lean, wind);

  // arms and sword
  const sh = by + 7;
  const sleeve = (dx, dy, w, h, shade = 1) => {
    px(dx + lean, dy, w, h, c.robe[shade]);
    px(dx + lean, dy, w, 1, c.trim[0]);
  };
  const hand = (dx, dy) => px(dx + lean, dy, 2, 2, c.skin[0]);
  if (pose === "channel" || pose === "cast") {
    // the near arm out, palm open, qi gathering in it; the far hand at the chest
    const reach = pose === "cast" ? 8 : 5;
    sleeve(3, sh - 3, reach, 4);
    px(3 + lean, sh - 3, 2, 4, c.robe[2]);
    hand(3 + reach, sh - 2);
    const r = pose === "cast" ? 2 : Math.sin(t * 30) > 0 ? 1 : 2;
    px(5 + reach + lean, sh - 2 - (r - 1), r + 1, r + 1, c.glow);
    dot(6 + reach + lean + r, sh + r, "#ffffff");
    px(-1 + lean, sh - 2, 2, 2, c.skin[1]);
    sword(px, line, c, "back", lean, by);
  } else if (pose === "draw") {
    sleeve(-7, sh - 1, 4, 4, 2);
    hand(-8, sh + 2);
    sword(px, line, c, "raised", lean, by);
  } else if (pose === "slash") {
    sleeve(3, sh - 2, 6, 3);
    hand(9, sh - 2);
    sword(px, line, c, "thrust", lean, by);
  } else if (pose === "guard") {
    sleeve(3, sh - 2, 4, 4);
    hand(7, sh - 2);
    sword(px, line, c, "upright", lean, by);
  } else if (pose === "win") {
    sleeve(2, sh - 6, 3, 6);
    hand(2, sh - 7);
    sword(px, line, c, "planted", lean, by);
  } else if (pose === "fly") {
    // arms out for balance, the sleeves streaming
    sleeve(-9, sh - 1, 4, 3, 2);
    sleeve(5, sh - 1, 4, 3);
    hand(-10, sh - 1);
    hand(9, sh - 1);
  } else {
    // idle, walk, jump, land, hurt, frozen, shocked: wide sleeves hanging, the sword at the hip
    sleeve(-7, sh - 8, 3, 9, 2);
    sleeve(4, sh - 8, 3, 9);
    px(4 + lean, sh - 8, 3, 1, c.trim[1]);
    hand(5, sh - 9);
    sword(px, line, c, "hip", lean, by);
  }
}

function head(px, c, f, y, lean, wind) {
  const t = f.poseT;
  // neck, face, an eye and a brow
  px(-1 + lean, y, 2, 1, c.skin[1]);
  px(-3 + lean, y + 1, 7, 7, c.skin[0]);
  px(-3 + lean, y + 1, 2, 7, c.skin[1]);
  px(2 + lean, y + 4, 1, 2, c.outline);
  px(2 + lean, y + 6, 2, 1, c.hair[1]);
  px(3 + lean, y + 2, 1, 1, c.skin[1]);
  if (f.school === "thunder") {
    // black hair swept up into a topknot under a gold crown, a red ribbon streaming from it
    px(-4 + lean, y + 6, 8, 3, c.hair[1]);
    px(-4 + lean, y + 2, 3, 5, c.hair[1]);
    px(-3 + lean, y + 8, 5, 1, c.hair[0]);
    px(-1 + lean, y + 9, 3, 3, c.hair[1]);
    px(-2 + lean, y + 10, 5, 1, c.trim[0]);
    px(0 + lean, y + 11, 1, 2, c.trim[0]);
    px(-2 + lean, y + 10, 1, 1, c.trim[1]);
    for (let i = 0; i < 6; i++) {
      const sway = Math.round(Math.sin(t * 5 + i * 0.9) * (i / 3));
      px(-3 + lean - i + Math.min(0, wind) * Math.round(i / 3), y + 10 - Math.round(i / 2) + sway, 1, 1, c.ribbon[i < 3 ? 0 : 1]);
    }
  } else {
    // long white hair falling down the back and lifting in the wind, a jade pin
    px(-4 + lean, y + 6, 8, 3, c.hair[0]);
    px(-5 + lean, y + 3, 3, 5, c.hair[0]);
    px(2 + lean, y + 6, 2, 2, c.hair[0]);
    px(-4 + lean, y + 8, 6, 1, c.hair[1]);
    for (let i = 0; i < 14; i++) {
      const sway = Math.round(Math.sin(t * 3 - i * 0.45) * (i / 7)) + Math.round((Math.min(0, wind) * i) / 6);
      px(-5 + lean - Math.round(i / 5) + sway, y + 3 - i, 2, 1, i % 4 === 0 ? c.hair[1] : c.hair[0]);
    }
    px(-1 + lean, y + 9, 3, 1, c.jade);
    px(-2 + lean, y + 9, 1, 1, c.trim[1]);
  }
}

/** The sword, where the pose holds it: at the hip, on the back, raised, thrust, upright or planted. */
function sword(px, line, c, how, lean, by) {
  const [edge, blade, dull] = c.blade;
  if (how === "thrust") {
    const y = by + 6;
    line(11 + lean, y, 26 + lean, y, blade);
    line(11 + lean, y + 1, 24 + lean, y + 1, edge);
    px(27 + lean, y, 1, 1, "#ffffff");
    px(10 + lean, y - 2, 1, 5, c.trim[0]);
    // the arc the blade just cut
    px(14 + lean, y + 3, 10, 1, c.glow);
    px(18 + lean, y + 4, 5, 1, c.glow);
  } else if (how === "upright") {
    line(8 + lean, by + 4, 8 + lean, by + 18, blade);
    line(9 + lean, by + 5, 9 + lean, by + 17, edge);
    px(8 + lean, by + 19, 1, 1, "#ffffff");
    px(6 + lean, by + 3, 5, 1, c.trim[0]);
  } else if (how === "raised") {
    line(-8 + lean, by + 10, -16 + lean, by + 22, blade);
    line(-7 + lean, by + 10, -15 + lean, by + 22, edge);
    px(-9 + lean, by + 8, 3, 1, c.trim[0]);
  } else if (how === "planted") {
    line(2 + lean, by - 4, 2 + lean, by + 6, blade);
    px(0 + lean, by + 7, 5, 1, c.trim[0]);
    px(2 + lean, by + 8, 1, 2, c.trim[1]);
  } else if (how === "back") {
    line(-5 + lean, by - 2, -9 + lean, by + 12, dull);
    px(-10 + lean, by + 12, 3, 1, c.trim[0]);
    px(-10 + lean, by + 13, 1, 2, c.ribbon[0]);
  } else {
    // in its scabbard at the hip, hilt forward, a tassel hanging
    line(-9 + lean, by - 9, 3 + lean, by - 1, dull);
    line(-9 + lean, by - 8, 2 + lean, by - 1, c.trim[1]);
    px(3 + lean, by - 1, 1, 2, c.trim[0]);
    line(4 + lean, by, 6 + lean, by + 1, c.robe[2]);
    px(4 + lean, by - 3, 1, 2, c.ribbon[0]);
  }
}

/** Relative lightness of a "#rrggbb" colour, 0 to 1. */
function lum(hex) {
  const n = parseInt(hex.slice(1), 16);
  return (0.3 * (n >> 16) + 0.59 * ((n >> 8) & 0xff) + 0.11 * (n & 0xff)) / 255;
}

// ---------- marks, streaks and numbers ----------

/**
 * What hangs on a fighter besides its pose: a ward's shell round it, and over its head a mark for each effect on
 * it (a flame for a burn, a broken shield for defence down, a seal for silence, a snowflake for a chill) and its
 * fury stacks.
 */
function drawStatus(ctx, f, ground, s) {
  const fx = Math.round(f.x);
  const fy = Math.round(ground - f.air);
  const box = (x, y, w, h, col, a = 1) => {
    ctx.globalAlpha = a;
    ctx.fillStyle = col;
    ctx.fillRect(x * s, y * s, w * s, h * s);
  };
  if (f.shield > 0) {
    // a shell of frost qi, flickering at the edges
    const a = 0.45 + 0.25 * Math.sin(f.poseT * 9);
    box(fx - 11, fy - 41, 23, 1, "#a6ecff", a);
    box(fx - 11, fy + 1, 23, 1, "#a6ecff", a);
    box(fx - 12, fy - 40, 1, 41, "#a6ecff", a);
    box(fx + 12, fy - 40, 1, 41, "#a6ecff", a);
    box(fx - 11, fy - 40, 23, 41, "#a6ecff", 0.08);
  }
  const marks = [];
  if (f.burn) marks.push("burn");
  if (f.defDown > 0) marks.push("defDown");
  if (f.silence > 0) marks.push("silence");
  if (f.chilled > 0) marks.push("chill");
  let x = fx - Math.round((marks.length * 6 - 1) / 2);
  const y = fy - 48;
  for (const m of marks) {
    if (m === "burn") {
      const lick = Math.sin(f.poseT * 14) > 0 ? 1 : 0;
      box(x + 1, y + 1, 3, 3, "#ff9a3a");
      box(x + 2 - lick, y - 1, 1, 2, "#ffd84a");
      box(x + 2, y + 2, 1, 2, "#fff0b0");
    } else if (m === "defDown") {
      // a shield with a crack through it
      box(x, y - 1, 5, 4, "#8f8aa8");
      box(x + 1, y + 3, 3, 1, "#8f8aa8");
      box(x + 2, y + 4, 1, 1, "#8f8aa8");
      box(x + 2, y - 1, 1, 3, "#ff4a3a");
    } else if (m === "chill") {
      // a snowflake
      box(x + 2, y - 1, 1, 5, "#bff2ff");
      box(x, y + 1, 5, 1, "#bff2ff");
      box(x + 1, y, 1, 1, "#bff2ff");
      box(x + 3, y + 2, 1, 1, "#bff2ff");
    } else {
      // a seal over the lips: a ring with a bar through it
      box(x, y, 5, 1, "#c9c3e6");
      box(x, y + 4, 5, 1, "#c9c3e6");
      box(x, y, 1, 5, "#c9c3e6");
      box(x + 4, y, 1, 5, "#c9c3e6");
      box(x + 1, y + 2, 3, 1, "#ff4a3a");
    }
    x += 6;
  }
  // fury: a gold pip per stack over the head
  for (let i = 0; i < f.fury; i++) box(fx - 7 + i * 3, fy - 43, 2, 1, "#ffc23d", 0.9);
  ctx.globalAlpha = 1;
}

/**
 * Each qi streak (from duel-feel.js `streaks`): Thunder's a crooked line of lightning, Frost's a straight shimmer
 * of ice, from the caster's hand to where the spell lands.
 */
export function drawStreaks(ctx, list, duel, s) {
  for (const k of list) {
    const school = duel.fighters[k.by].school;
    const c = LOOKS[school];
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

/** The numbers over the fighters (from duel-feel.js `numbers`): damage, burns, heals, wards and dodges. */
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
    ctx.fillStyle = look.color ?? LOOKS[duel.fighters[n.by].school].glow;
    ctx.fillText(n.text, x, y);
  }
  ctx.globalAlpha = 1;
}

// ---------- the bars ----------

const BRONZE = { light: "#f0cf7a", mid: "#b8893c", dark: "#6a4a1c", deep: "#2a1c0e" };
const BAR = { back: "#160f1c", lost: "#5a1a14", trail: "#fff0d0", qi: "#7fb4ff", qiDark: "#2c4a80", beadOff: "#3a2c1a" };
/** Life each bar last showed, so a blow's loss drains away behind the bar instead of vanishing. */
const shown = new WeakMap();

/**
 * The fight's bars, with no words: for each fighter a bronze medallion with its face, then its life (what a blow
 * just took drains away behind it), its qi under that, and its rage as beads that blaze once full (its ultimate
 * is ready). Between the two, a bronze plaque with a diamond for each round won. `top` is the bars' top edge in
 * game pixels.
 */
export function drawBars(ctx, duel, s, top) {
  const span = duel.right - duel.left;
  const medal = 26;
  const width = Math.max(30, Math.min(120, Math.floor(span / 2) - medal - 26));
  const box = (x, y, w, h, col, a = 1) => {
    ctx.globalAlpha = a;
    ctx.fillStyle = col;
    ctx.fillRect(x * s, y * s, w * s, h * s);
    ctx.globalAlpha = 1;
  };
  for (const f of duel.fighters) {
    const right = f.index === 1;
    const c = LOOKS[f.school];
    const mx = right ? duel.right - medal : duel.left;
    medallion(ctx, f, mx, top, medal, s, box, c);
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
  box(mid - 18, top + 2, 36, 9, BRONZE.deep);
  box(mid - 17, top + 3, 34, 7, BRONZE.dark);
  box(mid - 17, top + 3, 34, 1, BRONZE.mid);
  box(mid, top + 4, 1, 5, BRONZE.mid);
  for (const f of duel.fighters) {
    const won = Math.min(3, duel.wins[f.index]);
    for (let i = 0; i < 3; i++) {
      const x = f.index === 0 ? mid - 6 - i * 5 : mid + 4 + i * 5;
      const col = i < won ? LOOKS[f.school].glow : BRONZE.deep;
      box(x + 1, top + 4, 1, 1, col);
      box(x, top + 5, 3, 2, col);
      box(x + 1, top + 7, 1, 1, col);
    }
  }
}

/** A bronze medallion at (x, top), `size` across, with f's face in it, lit in its colour once its rage is full. */
function medallion(ctx, f, x, top, size, s, box, c) {
  // an octagon: a dark rim, bronze lit from above, a dark field
  const inset = 3;
  for (let y = 0; y < size; y++) {
    const cut = Math.max(0, inset - y, y - (size - 1 - inset));
    box(x + cut, top + y, size - 2 * cut, 1, BRONZE.deep);
    if (y > 0 && y < size - 1) box(x + cut + 1, top + y, size - 2 * cut - 2, 1, y < size / 2 ? BRONZE.light : BRONZE.mid);
    if (y > 1 && y < size - 2) box(x + cut + 2, top + y, size - 2 * cut - 4, 1, "#1a1424");
  }
  box(x + 4, top + 4, size - 8, size - 8, f.rage >= RAGE ? c.glow : "#241a30", f.rage >= RAGE ? 0.35 : 1);
  // the face: the sprite's head, scaled up, facing into the arena
  const b = spriteBuffer();
  b.ctx.clearRect(0, 0, SPRITE.w, SPRITE.h);
  paint(b.ctx, { ...f, pose: "idle", face: f.index === 0 ? 1 : -1, flash: 0, vx: 0 });
  outline(b.ctx, c.outline);
  ctx.save();
  ctx.beginPath();
  ctx.rect((x + 3) * s, (top + 3) * s, (size - 6) * s, (size - 6) * s);
  ctx.clip();
  ctx.imageSmoothingEnabled = false;
  // the face and the hair around it, at twice the sprite's size so it reads at a glance
  const field = size - 6;
  const head = { x: SPRITE.cx - field / 4, y: SPRITE.feet - 35, w: field / 2, h: field / 2 };
  ctx.globalAlpha = f.hp <= 0 ? 0.35 : 1;
  ctx.drawImage(b.c, head.x, head.y, head.w, head.h, (x + 3) * s, (top + 3) * s, field * s, field * s);
  ctx.restore();
  ctx.globalAlpha = 1;
}
