// How the duel looks: the two cultivators as pixel sprites, one pose per thing they can be doing, and the bars
// over the arena. Everything is drawn in game pixels times `s`, the canvas scale.
import { RAGE } from "./duel.js";


const LOOKS = {
  thunder: { robe: "#e8e2d4", shade: "#b8ad98", trim: "#e8b030", sash: "#8a1f17", hair: "#1a1420", skin: "#d8b896", blade: "#fff6c4", glow: "#ffe066" },
  frost: { robe: "#2c58aa", shade: "#1e3f80", trim: "#a6ecff", sash: "#151a3d", hair: "#e8f0ff", skin: "#e8cdb0", blade: "#dcfbff", glow: "#a6ecff" },
};

/** A frozen fighter is drawn in ice; a shocked one flickers with lightning. */
const ICE = { light: "#dff8ff", mid: "#a6ecff", dark: "#5fa8d8" };
const SHOCK = "#fff6a0";

const BAR = { hp: "#f2561d", hpLost: "#4a1712", qi: "#7fb4ff", back: "#13101b", edge: "#2f2a38", rage: "#ffc23d", rageOff: "#3a2c1a" };

/**
 * One fighter, feet at (f.x, ground - f.air), facing f.face: 34 pixels tall, the size pixelfx's effects are drawn
 * for (parry's blades meet 30 pixels up). Robe skirt 0-13, body 14-24, head 25-31, hair above.
 */
export function drawFighter(ctx, f, ground, s) {
  // a dodge leaves an afterimage where the blow was aimed
  if (f.dodged > 0 && f.hp > 0) {
    ctx.globalAlpha = f.dodged * 1.5;
    drawBody(ctx, { ...f, x: f.x + f.face * 6, flash: 1 }, ground, s);
    ctx.globalAlpha = 1;
  }
  drawBody(ctx, f, ground, s);
  if (f.hp > 0) drawStatus(ctx, f, ground, s);
}

function drawBody(ctx, f, ground, s) {
  const c = LOOKS[f.school];
  const fx = Math.round(f.x);
  const fy = Math.round(ground - f.air);
  const pose = f.pose;
  // the colour a part is drawn in: white in a hit's flash, ice when frozen, flickering when shocked
  const tint = (col) => {
    if (f.flash > 0) return "#ffffff";
    if (pose === "frozen") return col === c.skin || col === c.blade ? ICE.light : lum(col) > 0.45 ? ICE.mid : ICE.dark;
    if (pose === "shocked" && Math.sin(f.poseT * 70) > 0) return SHOCK;
    return col;
  };
  // a rect in the fighter's own frame: dx forward (toward where it faces), dy up from its feet
  const px = (dx, dy, w, h, col) => {
    const x = f.face > 0 ? fx + dx : fx - dx - w + 1;
    ctx.fillStyle = tint(col);
    ctx.fillRect(x * s, (fy - dy - h + 1) * s, w * s, h * s);
  };
  const bob = pose === "frozen" ? 0 : Math.sin(f.poseT * 3) > 0.6 ? 1 : 0;

  if (pose === "down" || pose === "tumble") {
    // lying on its back, robe spread, sword dropped beside it; thrown through the air the same way, a little
    // above where its feet would be
    if (pose === "tumble") return drawBody(ctx, { ...f, pose: "down", air: f.air + 4 }, ground, s);
    px(-14, 0, 22, 4, c.robe);
    px(-14, 0, 22, 1, c.shade);
    px(-4, 3, 6, 1, c.sash);
    px(8, 0, 6, 5, c.skin);
    px(12, 0, 4, 5, c.hair);
    px(-12, -1, 12, 1, c.shade);
    return;
  }

  if (pose === "meditate") {
    // cross-legged, floating, hands together before the chest
    const up = 6 + bob;
    px(-8, up, 17, 4, c.robe);
    px(-8, up, 17, 1, c.shade);
    px(-8, up + 3, 17, 1, c.trim);
    px(-4, up + 4, 9, 11, c.robe);
    px(-4, up + 8, 9, 2, c.sash);
    px(-1, up + 10, 3, 3, c.skin);
    head(px, c, f, up + 15, 0);
    return;
  }

  const lean = pose === "hurt" || pose === "shocked" ? -2 : pose === "slash" ? 2 : 0;
  // knees drawn up in a jump; squashed for a moment on landing
  const tuck = pose === "jump" ? 4 : pose === "land" ? 2 : 0;
  if (pose === "fly") {
    // riding its sword: the blade flat under its feet, glowing
    px(-9, -2, 19, 1, c.blade);
    px(10, -2, 1, 1, "#ffffff");
    px(-11, -2, 2, 1, c.trim);
    if (Math.sin(f.poseT * 20) > -0.3) px(-7, -3, 15, 1, c.glow);
  }
  // feet and robe skirt, flaring at the hem
  const step = pose === "walk" ? (Math.sin(f.poseT * 12) > 0 ? 2 : -2) : 0;
  px(-4 + step, 0, 3, 1, "#24130f");
  px(2 - step, 0, 3, 1, "#24130f");
  px(-6, 1 + tuck, 13, 4 - Math.min(tuck, 3), c.robe);
  px(-5, 5, 11, 9 - tuck, c.robe);
  px(-6, 1 + tuck, 1, 4 - Math.min(tuck, 3), c.shade);
  px(-5, 5, 1, 9 - tuck, c.shade);
  px(-6, 1 + tuck, 13, 1, c.trim);
  px(0, 2 + tuck, 1, 11 - tuck, c.trim);
  // body: robe crossed at the chest, sash with a hanging tail
  const by = 14 - bob;
  px(-4 + lean, by, 9, 11, c.robe);
  px(-4 + lean, by, 1, 11, c.shade);
  px(-4 + lean, by + 2, 9, 2, c.sash);
  px(-5 + lean, by - 2, 1, 4, c.sash);
  px(1 + lean, by + 4, 1, 7, c.trim);
  px(2 + lean, by + 8, 2, 1, c.trim);
  head(px, c, f, by + 11, lean);

  // arms and sword
  const shoulder = by + 8;
  const sleeve = (dx, dy, w, h) => px(dx + lean, dy, w, h, c.robe);
  const hand = (dx, dy) => px(dx + lean, dy, 2, 2, c.skin);
  if (pose === "channel" || pose === "cast") {
    const reach = pose === "cast" ? 8 : 5;
    sleeve(3, shoulder - 1, reach, 3);
    hand(3 + reach, shoulder - 1);
    // qi gathering in the palm
    if (pose === "cast" || Math.sin(f.poseT * 30) > 0) {
      px(5 + reach + lean, shoulder - 1, 2, 2, c.glow);
      px(6 + reach + lean, shoulder + 1, 1, 1, c.glow);
    }
    sword(px, c, -4 + lean, by + 1, "back");
  } else if (pose === "draw") {
    sleeve(-6, shoulder - 1, 4, 3);
    hand(-8, shoulder);
    sword(px, c, -8 + lean, shoulder + 2, "raised");
  } else if (pose === "slash") {
    sleeve(3, shoulder - 2, 5, 3);
    hand(8, shoulder - 2);
    sword(px, c, 10 + lean, shoulder - 1, "thrust");
  } else if (pose === "guard") {
    sleeve(3, shoulder - 2, 4, 3);
    hand(7, shoulder - 3);
    sword(px, c, 8 + lean, shoulder - 9, "upright");
  } else if (pose === "win") {
    sleeve(2, shoulder, 3, 6);
    hand(2, shoulder + 6);
    sword(px, c, 2 + lean, shoulder + 8, "upright");
  } else if (pose === "fly") {
    // arms out for balance, the sword under its feet
    sleeve(-8, shoulder - 1, 4, 2);
    sleeve(5, shoulder - 1, 4, 2);
    hand(-9, shoulder - 1);
    hand(8, shoulder - 1);
  } else {
    // idle, walk, jump, land, hurt, frozen, shocked: sleeves at the sides, sword at the hip
    sleeve(-6, by + 2, 2, 8);
    sleeve(5, by + 2, 2, 8);
    hand(-6, by + 1);
    sword(px, c, -7 + lean, by - 1, "hip");
  }
}

/** Relative lightness of a "#rrggbb" colour, 0 to 1. */
function lum(hex) {
  const n = parseInt(hex.slice(1), 16);
  return (0.3 * (n >> 16) + 0.59 * ((n >> 8) & 0xff) + 0.11 * (n & 0xff)) / 255;
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

/**
 * What hangs on a fighter besides its pose: a ward's shell round it, and over its head a mark for each effect on
 * it (a flame for a burn, a broken shield for defence down, a seal for silence) and its fury stacks.
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
    box(fx - 10, fy - 37, 21, 1, "#a6ecff", a);
    box(fx - 10, fy, 21, 1, "#a6ecff", a);
    box(fx - 11, fy - 36, 1, 36, "#a6ecff", a);
    box(fx + 11, fy - 36, 1, 36, "#a6ecff", a);
    box(fx - 10, fy - 36, 21, 36, "#a6ecff", 0.08);
  }
  const marks = [];
  if (f.burn) marks.push("burn");
  if (f.defDown > 0) marks.push("defDown");
  if (f.silence > 0) marks.push("silence");
  let x = fx - Math.round((marks.length * 6 - 1) / 2);
  const y = fy - 44;
  for (const m of marks) {
    if (m === "burn") {
      const lick = Math.sin(f.poseT * 14) > 0 ? 1 : 0;
      box(x + 1, y + 1, 3, 3, "#ff9a3a");
      box(x + 2 - lick, y - 1, 1, 2, "#ffd84a");
      box(x + 2, y + 2, 1, 2, "#fff0b0");
    } else if (m === "defDown") {
      // a shield with a crack and an arrow down
      box(x, y - 1, 5, 4, "#8f8aa8");
      box(x + 1, y + 3, 3, 1, "#8f8aa8");
      box(x + 2, y + 4, 1, 1, "#8f8aa8");
      box(x + 2, y - 1, 1, 3, "#ff4a3a");
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
  // fury: a gold pip per stack over the shoulder
  for (let i = 0; i < f.fury; i++) box(fx - 7 + i * 3, fy - 39, 2, 1, "#ffc23d", 0.9);
  ctx.globalAlpha = 1;
}

function head(px, c, f, y, lean) {
  px(-3 + lean, y, 6, 6, c.skin);
  px(2 + lean, y + 3, 1, 1, "#13101b");
  px(0 + lean, y, 3, 1, c.shade);
  if (f.school === "thunder") {
    // black hair in a topknot held by a gold crown pin
    px(-4 + lean, y + 4, 8, 3, c.hair);
    px(-4 + lean, y + 1, 2, 5, c.hair);
    px(-1 + lean, y + 7, 4, 2, c.hair);
    px(-2 + lean, y + 9, 6, 1, c.trim);
    px(0 + lean, y + 10, 2, 1, c.trim);
  } else {
    // long white hair flowing down the back, a blue band across the brow
    px(-4 + lean, y + 4, 8, 3, c.hair);
    px(-6 + lean, y - 10, 3, 16, c.hair);
    px(-5 + lean, y - 12, 2, 3, c.hair);
    px(-3 + lean, y + 4, 6, 1, c.trim);
  }
}

function sword(px, c, x, y, how) {
  if (how === "thrust") {
    px(x, y, 13, 1, c.blade);
    px(x + 13, y, 1, 1, "#ffffff");
    px(x - 1, y - 2, 1, 5, c.trim);
  } else if (how === "upright" || how === "raised") {
    px(x, y, 1, 13, c.blade);
    px(x, y + 13, 1, 1, "#ffffff");
    px(x - 2, y - 1, 5, 1, c.trim);
  } else if (how === "back") {
    px(x, y, 1, 13, c.shade);
    px(x - 2, y + 12, 5, 1, c.trim);
  } else {
    px(x, y - 9, 1, 10, c.shade);
    px(x - 2, y, 5, 1, c.trim);
  }
}

/**
 * The bars over the arena, with no words: each fighter's life (the part just lost fades out behind it), qi and its
 * rage pips, which flash once full (its ultimate is ready); between them, a diamond for each round won.
 */
export function drawBars(ctx, duel, s) {
  const width = Math.max(40, Math.min(110, Math.floor((duel.right - duel.left) / 2) - 18));
  // below the page's top bar
  const top = 22;
  const box = (x, y, w, h, col) => {
    ctx.fillStyle = col;
    ctx.fillRect(x * s, y * s, w * s, h * s);
  };
  for (const f of duel.fighters) {
    const left = f.index === 0 ? duel.left + 4 : duel.right - 4 - width;
    const from = (w) => (f.index === 0 ? left : left + width - w);
    box(left - 1, top - 1, width + 2, 8, BAR.edge);
    box(left, top, width, 5, BAR.hpLost);
    const hp = Math.round((width * f.hp) / 100);
    box(from(hp), top, hp, 5, BAR.hp);
    box(from(hp), top, hp, 1, "#ff9a6a");
    // a ward's shell shown on top of life
    if (f.shield > 0) {
      const w = Math.min(width, Math.round((width * f.shield) / f.stats.hp));
      box(from(w), top + 4, w, 1, "#a6ecff");
    }
    box(left, top + 5, width, 1, BAR.back);
    const qi = Math.round((width * f.qi) / 100);
    box(from(qi), top + 5, qi, 1, BAR.qi);
    const pip = 6;
    const full = f.rage >= RAGE && Math.sin(duel.time * 12) > 0;
    for (let i = 0; i < RAGE; i++) {
      const x = f.index === 0 ? left + i * (pip + 2) : left + width - pip - i * (pip + 2);
      box(x - 1, top + 8, pip + 2, 4, BAR.edge);
      box(x, top + 9, pip, 2, full ? "#ffffff" : i < f.rage ? BAR.rage : BAR.rageOff);
    }
  }
  // rounds won: diamonds either side of the middle, in each fighter's colour
  const mid = Math.round((duel.left + duel.right) / 2);
  for (const f of duel.fighters) {
    const shown = Math.min(5, duel.wins[f.index]);
    for (let i = 0; i < 5; i++) {
      const x = f.index === 0 ? mid - 6 - i * 6 : mid + 3 + i * 6;
      const col = i < shown ? LOOKS[f.school].glow : BAR.edge;
      box(x + 1, top, 1, 1, col);
      box(x, top + 1, 3, 1, col);
      box(x + 1, top + 2, 1, 1, col);
    }
  }
}
