// How the duel looks: the two cultivators as pixel sprites, one pose per thing they can be doing, the marks that
// hang on them, the streaks and numbers of the fight, and the fight's bars. Everything is drawn in game pixels
// times `s`, the canvas scale.
//
// A fighter is painted into a small buffer at one pixel per game pixel, outlined there (dark, or in its school's
// glow while it gathers a spell, so the caster is plain to see), then drawn scaled onto the scene.
import { RAGE, SPELLS } from "./duel.js";

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
export function drawFighter(ctx, f, ground, s, jolt = [0, 0]) {
  // a dodge leaves an afterimage where the blow was aimed
  if (f.dodged > 0 && f.hp > 0) blit(ctx, { ...f, x: f.x + f.face * 7, flash: 1 }, ground, s, f.dodged * 1.5, [0, 0]);
  blit(ctx, f, ground, s, 1, jolt);
  if (f.hp > 0) drawStatus(ctx, f, ground, s);
}

/** Where a turning pose turns about, in pixels above the feet: a tumble round its middle, a roll round its ball. */
const PIVOT = { tumble: 18, roll: 5 };

/**
 * Quarter turns a tumbling or rolling fighter has made: one every four frames (a turn 3.75 times a second), the
 * way it is moving. Whole quarter turns keep every pixel on the grid.
 */
function quarterTurns(f) {
  if (!(f.pose in PIVOT)) return 0;
  const way = Math.sign(f.vx) || -f.face;
  return (Math.floor(f.poseT * 15) % 4) * way;
}

function blit(ctx, f, ground, s, alpha, [jx, jy]) {
  const b = spriteBuffer();
  b.ctx.clearRect(0, 0, SPRITE.w, SPRITE.h);
  // a tumble is the fighter flung spread-eagled, turned about its middle
  paint(b.ctx, f.pose === "tumble" ? { ...f, pose: "spread" } : f);
  const p = gathered(f);
  const gathering = f.pose === "channel" || f.pose === "cast";
  const glow = gathering ? LOOKS[f.school].glow : null;
  outline(b.ctx, glow && Math.sin(f.poseT * 18) > -0.4 ? glow : LOOKS[f.school].outline);
  ctx.imageSmoothingEnabled = false;
  // the last fifth of a long wind-up trembles with the power held in
  const tremble = p !== null && p > 0.8 && SPELLS[f.act.spell].windup >= 0.3 && Math.floor(f.poseT * 30) % 2 ? 1 : 0;
  const x = Math.round(f.x) - SPRITE.cx + jx + tremble;
  const y = Math.round(ground - f.air) - SPRITE.feet + jy;
  // a caster's glow spills round it while it gathers, brighter as the spell builds
  if (glow) {
    // an aura in two hard-edged rings, the inner one brighter, swelling as the spell builds
    const k = p ?? 1;
    const cx = x + SPRITE.cx;
    const cy = y + SPRITE.feet - 20;
    ctx.fillStyle = glow;
    for (const [rx, ry, a] of [[10 + 4 * k, 22 + 3 * k, 0.07], [7 + 2 * k, 17 + 2 * k, 0.09]]) {
      ctx.globalAlpha = alpha * (a + 0.12 * k) * (0.85 + 0.15 * Math.sin(f.poseT * 9));
      for (let row = -Math.round(ry); row <= Math.round(ry); row++) {
        const half = Math.round(rx * Math.sqrt(1 - (row * row) / (ry * ry)));
        ctx.fillRect((cx - half) * s, (cy + row) * s, (half * 2 + 1) * s, s);
      }
    }
  }
  ctx.globalAlpha = alpha;
  const turn = quarterTurns(f);
  if (turn) {
    const pivot = SPRITE.feet - PIVOT[f.pose];
    ctx.save();
    ctx.translate((x + SPRITE.cx) * s, (y + pivot) * s);
    ctx.rotate((turn * Math.PI) / 2);
    ctx.drawImage(b.c, -SPRITE.cx * s, -pivot * s, SPRITE.w * s, SPRITE.h * s);
    ctx.restore();
  } else ctx.drawImage(b.c, x * s, y * s, SPRITE.w * s, SPRITE.h * s);
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

/** How each kind of spell is gathered: a summoning raises both arms, a palm draws back, a prayer joins the hands. */
const GATHER = { zone: "summon", void: "summon", orb: "summon", bolt: "palm", blast: "palm", dash: "lunge", heal: "prayer", ward: "prayer", barrier: "cross", blink: "cross" };

/** How far fighter f has gathered what it is winding up, 0 to 1; null when it is not winding anything up. */
function gathered(f) {
  if (!f.act || f.act.phase !== "windup") return null;
  return Math.min(1, f.act.t / SPELLS[f.act.spell].windup);
}

/** Paint fighter f's pose into the sprite buffer. */
function paint(bctx, f) {
  const c = LOOKS[f.school];
  const pose = f.pose;
  // the colour a part is painted in: white in a hit's flash, ice when frozen, flickering when shocked
  const tint = (col) => {
    if (f.flash > 0) return "#ffffff";
    if (f.frozen > 0) return lum(col) > 0.7 ? ICE.light : lum(col) > 0.4 ? ICE.mid : ICE.dark;
    if (f.shocked > 0 && Math.sin(f.poseT * 70) > 0) return SHOCK;
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
  const still = f.frozen > 0;
  const bob = still ? 0 : Math.sin(t * 3) > 0.6 ? 1 : 0;
  // cloth streams back from the way it moves, and sways a little when it does not
  const wind = still ? 0 : Math.max(-3, Math.min(3, -(f.vx ?? 0) * f.face * 0.03)) - 1 + Math.round(Math.sin(t * 4));
  const out = f.hp <= 0;
  // limbs thrown about while it flies: they swing every few frames
  const flail = still ? 0 : Math.floor(t * 12) % 2;

  if (pose === "down") {
    // flat on its back; a knocked-out fighter's eyes are crossed out and its sword lies dropped beside it
    const breath = !out && !still && Math.floor(t * 2.5) % 2 ? 1 : 0;
    lying(c, f, px, 0, 0, 0, breath, out ? "x" : "shut");
    if (out) {
      line(-6, -1, 10, -1, c.blade[2]);
      px(11, -1, 1, 1, c.trim[0]);
    }
    return;
  }
  if (pose === "launched") {
    // thrown up and back: feet higher than its head, arms flung up past it
    lying(c, f, px, 6, 3, 5 + flail, 0, "shut");
    return;
  }
  if (pose === "fall") {
    // falling flat on its back, arms and legs trailing up as it drops
    lying(c, f, px, 6, 1, 7 - flail, 0, "shut");
    return;
  }
  if (pose === "roll") {
    // tucked into a ball for a quick rise; blit turns it a quarter at a time
    px(-6, 1, 12, 9, c.robe[1]);
    px(-5, 0, 10, 11, c.robe[1]);
    px(-5, 0, 10, 1, c.robe[2]);
    px(-6, 4, 12, 1, c.trim[0]);
    px(1, 5, 5, 5, c.skin[0]);
    px(1, 9, 5, 2, c.hair[1]);
    px(-4, 1, 3, 2, c.boot);
    return;
  }
  if (pose === "sit") {
    // sitting up from the floor: legs out in front, leaning back on one hand, still dazed
    px(10, 0, 3, 4, c.boot);
    px(-2, 0, 12, 4, c.robe[1]);
    px(-2, 3, 12, 1, c.robe[0]);
    px(8, 0, 2, 4, c.trim[0]);
    px(-7, 3, 9, 9, c.robe[1]);
    px(-7, 3, 2, 9, c.robe[2]);
    px(-7, 3, 9, 1, c.trim[0]);
    line(-3, 5, 0, 11, c.inner[0]);
    // the arm behind, propping it up from the floor
    for (let i = 0; i <= 7; i++) px(-6 - Math.round(i * 0.75), 9 - i, 2, 1, c.robe[2]);
    px(-6, 9, 2, 1, c.trim[0]);
    px(-13, 0, 3, 2, c.skin[0]);
    head(px, c, f, 12, -3, wind, "shut");
    return;
  }
  if (pose === "duck") {
    // dropped low under the blow, body bent level, an arm over the head
    px(-5, 0, 4, 2, c.boot);
    px(3, 0, 4, 2, c.boot);
    px(-7, 2, 15, 5, c.robe[1]);
    px(-7, 2, 15, 1, c.trim[0]);
    px(-7, 2, 2, 5, c.robe[2]);
    px(-5, 7, 11, 5, c.robe[1]);
    px(-5, 11, 11, 1, c.robe[2]);
    px(2, 7, 3, 4, c.inner[0]);
    px(6, 6, 6, 6, c.skin[0]);
    px(6, 6, 2, 6, c.skin[1]);
    px(10, 8, 1, 1, c.outline);
    px(5, 11, 8, 3, c.school === "thunder" ? c.hair[1] : c.hair[0]);
    px(4, 12, 2, 2, c.school === "thunder" ? c.hair[1] : c.hair[0]);
    px(3, 13, 3, 1, c.trim[0]);
    px(6, 4, 5, 3, c.robe[1]);
    px(11, 4, 2, 2, c.skin[0]);
    line(-10, 6, -3, 3, c.blade[2]);
    return;
  }

  if (pose === "meditate") {
    // seated cross-legged on the floor, seen from the side: thighs out in front with a knee at the end and a foot
    // turned up on top, the robe spread round the seat, back straight, hands resting on the knees; it breathes
    const breath = still ? 0 : Math.floor(t * 1.5) % 2;
    px(-8, 0, 20, 1, c.robe[2]);
    px(-7, 1, 6, 3, c.robe[1]);
    px(-7, 1, 2, 3, c.robe[2]);
    px(-2, 1, 13, 3, c.robe[1]);
    px(-2, 3, 12, 1, c.robe[0]);
    px(11, 1, 1, 2, c.robe[1]);
    px(1, 1, 9, 1, c.robe[2]);
    px(5, 4, 3, 1, c.boot);
    px(-8, 1, 1, 1, c.trim[0]);
    px(-8, 0, 20, 1, c.trim[0]);
    // the body, upright on the seat
    const by = 4 + breath;
    px(-5, by, 9, 9, c.robe[1]);
    px(-5, by, 2, 9, c.robe[2]);
    px(3, by + 1, 1, 8, c.robe[0]);
    line(0, by + 2, 3, by + 8, c.inner[0]);
    line(-1, by + 2, 2, by + 8, c.trim[0]);
    px(-5, by, 9, 1, c.trim[0]);
    // forearms along the thighs, hands on the knees, thumb and finger touching
    px(1, by - 1, 7, 2, c.robe[1]);
    px(1, by, 7, 1, c.trim[0]);
    px(8, 4, 2, 2, c.skin[0]);
    px(9, 6, 1, 1, c.skin[1]);
    head(px, c, f, by + 9, 0, wind, "shut");
    return;
  }

  const p = gathered(f);
  const how = f.act ? GATHER[SPELLS[f.act.spell].kind] ?? "palm" : null;
  // a fresh blow snaps it back; then it reels, bent over the hurt, lower the harder it was struck
  const recoil = (pose === "hurt" && f.hitT < 0.1) || pose === "spread";
  const reel = pose === "hurt" && !recoil;
  // how far the body sinks: a crouch, a landing, a kneel, a reel, a stance braced to gather or to strike
  const braced = (pose === "channel" && (how === "summon" || how === "palm")) || (pose === "draw" && f.air === 0);
  const drop =
    pose === "crouch" ? 4 : pose === "land" ? 3 : pose === "kneel" ? 7 : reel ? (f.hitPower >= 9 ? 5 : 3) :
    pose === "channel" && how === "lunge" ? 4 : braced ? 2 : 0;
  const lean =
    recoil ? -3 : reel ? 2 : pose === "shocked" ? -2 : pose === "slash" ? 2 : pose === "cast" ? 1 :
    pose === "channel" && how === "lunge" ? 2 : pose === "channel" && how === "palm" ? -1 : pose === "draw" ? -1 : 0;
  // knees drawn up in a jump
  const tuck = pose === "jump" ? 4 : 0;

  if (pose === "fly") {
    // riding its sword: the blade flat under its feet, a trail of light behind
    line(-11, -2, 11, -2, c.blade[1]);
    line(-11, -1, 9, -1, c.blade[0]);
    px(12, -2, 1, 1, "#ffffff");
    px(-13, -3, 2, 3, c.trim[0]);
    px(-15, -2, 2, 1, c.trim[1]);
    if (Math.sin(t * 20) > -0.3) line(-16 - (Math.round(t * 30) % 3), -2, -14, -2, c.glow);
  }

  // feet: apart in a stance, staggering back from a blow, one knee on the floor in a kneel
  const stride = pose === "walk" ? (Math.sin(t * 12) > 0 ? 2 : -2) : 0;
  const spread = pose === "spread" ? 3 : drop > 0 && pose !== "kneel" ? 2 : 0;
  if (pose === "kneel") {
    px(4, 0, 4, 2, c.boot);
    px(-7, 0, 5, 2, c.robe[2]);
    px(-10, 0, 3, 1, c.boot);
    px(3, 2, 6, 3, c.robe[1]);
    px(3, 4, 6, 1, c.robe[0]);
  } else {
    px(-4 + stride - spread - (reel ? 2 : 0), 0, 4, 2, c.boot);
    px(1 - stride + spread, 0, 4, 2, c.boot);
  }
  // the lower robe, flaring to the hem, open at the front over the inner robe; shorter as the knees bend
  const top = 15 - drop;
  for (let dy = 2 + tuck; dy <= top; dy++) {
    const flare = Math.max(0, Math.round((8 - dy) / 2)) + (drop > 0 ? 1 : 0);
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
  const by = 16 - bob - drop;
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
  // the head snaps back further than the body from a fresh blow, and hangs forward as it reels
  const eye = recoil || reel || pose === "shocked" ? "shut" : pose === "channel" && how === "prayer" ? "shut" : "open";
  head(px, c, f, by + 9, lean + (recoil ? -2 : reel ? 1 : 0), wind, eye);

  // arms and sword
  const sh = by + 7;
  // a sleeve stands out from the robe behind it: the near arm a tone lighter, each edged dark along its back and top
  const sleeve = (dx, dy, w, h, shade = 0) => {
    const edge = shade === 2 ? c.outline : c.robe[2];
    px(dx + lean, dy, w, h, c.robe[shade]);
    if (w > 2) px(dx + lean, dy, 1, h, edge);
    if (h > 2) px(dx + lean, dy + h - 1, w, 1, edge);
    px(dx + lean, dy, w, 1, c.trim[0]);
  };
  const hand = (dx, dy) => px(dx + lean, dy, 2, 2, c.skin[0]);
  // qi drawn in from round about to a point, closer as the spell gathers
  const motes = (cx, cy, k) => {
    const r = 2 + Math.round((1 - k) * 11);
    for (let i = 0; i < 4; i++) {
      const a = t * 7 + (i * Math.PI) / 2;
      dot(Math.round(cx + lean + Math.cos(a) * r), Math.round(cy + Math.sin(a) * r * 0.7), i % 2 ? c.glow : "#ffffff");
    }
  };
  const orb = (cx, cy, k) => {
    const r = 1 + Math.round(2 * k);
    px(cx + lean - Math.floor(r / 2), cy - Math.floor(r / 2), r, r, c.glow);
    if (k > 0.5) dot(cx + lean, cy, "#ffffff");
  };
  if (pose === "channel") {
    if (how === "summon") {
      // both arms rise out to the sides and up as it gathers, a ball of qi swelling over the head between the hands
      const up = Math.round(2 + 9 * p);
      const out = Math.round(6 - 2 * p);
      for (const [x0, side, shade] of [[-4, -1, 2], [3, 1, 1]]) {
        const x1 = x0 + side * out;
        line(x0 + lean, sh - 1, x1 + lean, sh - 1 + up, c.robe[shade]);
        line(x0 + lean + side, sh - 1, x1 + lean + side, sh - 1 + up, c.robe[shade]);
        dot(x0 + lean, sh - 1, c.trim[0]);
        hand(x1 - (side < 0 ? 1 : 0), sh + up);
      }
      orb(0, sh + up + 4, p);
      motes(0, sh + up + 4, p);
      sword(px, line, c, "back", lean, by);
    } else if (how === "palm" || how === "lunge") {
      // the near palm drawn back to the hip, gathering; the far hand out before it, two fingers pointing
      sleeve(-7, sh - 4, 5, 3, 2);
      hand(-9, sh - 4);
      orb(-9, sh - 3, p);
      motes(-9, sh - 3, p);
      sleeve(3, sh - 1, 4, 2);
      hand(7, sh - 1);
      px(9 + lean, sh, 2, 1, c.skin[0]);
      sword(px, line, c, how === "lunge" ? "hip" : "back", lean, by);
    } else if (how === "prayer") {
      // the hands joined before the chest, eyes shut, qi rising round it
      sleeve(-2, sh - 4, 4, 3, 2);
      sleeve(1, sh - 4, 4, 3);
      hand(4, sh - 3);
      px(5 + lean, sh - 1, 1, 3, c.skin[0]);
      for (let i = 0; i < 3; i++) dot(-6 + i * 6, ((Math.floor(t * 20) + i * 5) % 14) + by, c.glow);
      sword(px, line, c, "back", lean, by);
    } else {
      // arms crossed before the chest, braced
      line(-3 + lean, sh - 4, 6 + lean, sh, c.robe[2]);
      line(-3 + lean, sh, 6 + lean, sh - 4, c.robe[1]);
      hand(6, sh - 1);
      hand(6, sh - 5);
      sword(px, line, c, "hip", lean, by);
    }
  } else if (pose === "cast") {
    if (how === "prayer" || how === "cross") {
      // arms flung wide as the spell goes out
      sleeve(-9, sh - 1, 4, 3, 2);
      sleeve(5, sh - 1, 4, 3);
      hand(-10, sh);
      hand(9, sh);
    } else {
      // the near arm thrust out, palm open, the spell leaving it
      sleeve(3, sh - 3, 8, 4);
      px(3 + lean, sh - 3, 2, 4, c.robe[2]);
      hand(11, sh - 2);
      px(13 + lean, sh - 2, 3, 3, c.glow);
      dot(15 + lean, sh + 1, "#ffffff");
      px(-1 + lean, sh - 2, 2, 2, c.skin[1]);
    }
    sword(px, line, c, "back", lean, by);
  } else if (pose === "draw") {
    // the blade drawn back over the shoulder, higher the nearer the blow; from the air, held point down to plunge
    if (f.act && SPELLS[f.act.spell].dive) {
      sleeve(1, sh - 6, 3, 6);
      hand(2, sh - 7);
      line(3 + lean, sh - 8, 3 + lean, sh - 24, c.blade[1]);
      line(4 + lean, sh - 8, 4 + lean, sh - 23, c.blade[0]);
      px(1 + lean, sh - 7, 5, 1, c.trim[0]);
    } else {
      sleeve(-7, sh - 1, 4, 4, 2);
      hand(-8, sh + 2);
      sword(px, line, c, "raised", lean, by + Math.round(2 * (p ?? 1)));
    }
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
  } else if (recoil) {
    // struck: arms thrown out behind and up
    sleeve(-9, sh - 1, 4, 3, 2);
    hand(-10, sh + 1);
    sleeve(3, sh, 3, 4);
    hand(4, sh + 4);
    sword(px, line, c, "hip", lean, by);
  } else if (reel) {
    // clutching the chest where it was struck, the other arm hanging
    sleeve(-7, sh - 8, 3, 9, 2);
    sleeve(1, sh - 3, 4, 3);
    hand(3, sh - 2);
    sword(px, line, c, "hip", lean, by);
  } else if (pose === "shocked") {
    // locked rigid, arms jerking out
    const j = Math.sin(t * 60) > 0 ? 1 : 0;
    sleeve(-9, sh - 1 + j, 4, 2, 2);
    sleeve(5, sh - j, 4, 2);
    hand(-10, sh - 1 + j);
    hand(9, sh - j);
    sword(px, line, c, "hip", lean, by);
  } else if (pose === "kneel") {
    // a hand on the raised knee, pushing up
    sleeve(-7, sh - 8, 3, 9, 2);
    sleeve(3, sh - 5, 3, 5);
    hand(4, sh - 6);
    sword(px, line, c, "hip", lean, by);
  } else {
    // idle, walk, jump, land, crouch, frozen: wide sleeves hanging, the sword at the hip
    sleeve(-7, sh - 8, 3, 9, 2);
    sleeve(4, sh - 8, 3, 9);
    px(4 + lean, sh - 8, 3, 1, c.trim[1]);
    hand(5, sh - 9);
    sword(px, line, c, "hip", lean, by);
  }
}

/**
 * A fighter on its back, `up` pixels off the floor: feet toward the foe, head away, its legs raised `legs` pixels
 * and its arms flung up `arms` (0: along its sides), its chest lifting `breath`, its eyes open, shut or crossed out.
 */
function lying(c, f, px, up, legs, arms, breath, eye) {
  const l = up + legs;
  px(12, l, 3, 4, c.boot);
  px(-1, up + Math.min(legs, 2), 13, 4, c.robe[1]);
  px(-1, up + Math.min(legs, 2) + 3, 13, 1, c.robe[0]);
  px(-1, up + Math.min(legs, 2), 13, 1, c.robe[2]);
  px(10, up + Math.min(legs, 2), 2, 4, c.trim[0]);
  px(-11, up, 11, 5 + breath, c.robe[1]);
  px(-11, up, 11, 1, c.robe[2]);
  px(-9, up + 4 + breath, 5, 1, c.inner[0]);
  px(-8, up + 3 + breath, 3, 1, c.inner[1]);
  px(-2, up, 1, 5 + breath, c.trim[0]);
  if (arms > 0) {
    // flung up and back past the head from the shoulders
    for (const [x0, len, shade] of [[-10, arms, 1], [-8, arms - 2, 2]]) {
      const x1 = x0 - Math.round(len * 0.6);
      for (let i = 0; i <= len; i++) {
        const x = Math.round(x0 + ((x1 - x0) * i) / len);
        px(x, up + 5 + i, 2, 1, c.robe[shade]);
      }
      px(x1 - 1, up + 6 + len, 2, 2, c.skin[0]);
    }
  } else {
    px(-9, up + 5 + breath, 7, 1, c.robe[2]);
    px(-2, up + 5 + breath, 2, 1, c.skin[0]);
  }
  // the head, face up
  px(-18, up, 7, 6, c.skin[0]);
  px(-18, up, 7, 1, c.skin[1]);
  if (eye === "x") {
    px(-15, up + 3, 1, 1, c.outline);
    px(-14, up + 4, 1, 1, c.outline);
    px(-13, up + 3, 1, 1, c.outline);
    px(-15, up + 5, 1, 1, c.outline);
    px(-13, up + 5, 1, 1, c.outline);
  } else px(-15, up + 4, 3, 1, c.outline);
  if (f.school === "thunder") {
    px(-21, up, 3, 6, c.hair[1]);
    px(-23, up + 1, 2, 3, c.hair[1]);
    px(-22, up + 4, 1, 1, c.trim[0]);
    px(-25, up, 2, 1, c.ribbon[0]);
  } else {
    px(-21, up, 3, 6, c.hair[0]);
    px(-29, up, 9, 2, c.hair[0]);
    px(-29, up, 9, 1, c.hair[1]);
    px(-20, up + 5, 2, 1, c.jade);
  }
}

function head(px, c, f, y, lean, wind, eye = "open") {
  const t = f.poseT;
  // neck, face, an eye (open, or screwed shut) and a brow
  px(-1 + lean, y, 2, 1, c.skin[1]);
  px(-3 + lean, y + 1, 7, 7, c.skin[0]);
  px(-3 + lean, y + 1, 2, 7, c.skin[1]);
  if (eye === "shut") {
    px(1 + lean, y + 4, 3, 1, c.outline);
    px(2 + lean, y + 2, 2, 1, c.skin[1]);
  } else px(2 + lean, y + 4, 1, 2, c.outline);
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
/**
 * A barrier: a dome of the school's qi round the body, faint while it waits and flaring white where a blow has just
 * rung on it. Thunder's golden bell is banded like a bell; Frost's ice mirror is faceted.
 */
function dome(box, f, fx, fy) {
  const c = LOOKS[f.school];
  const flare = f.rang > 0;
  const edge = flare ? "#ffffff" : c.glow;
  const a = flare ? 0.95 : 0.55 + 0.2 * Math.sin(f.poseT * 10);
  const cy = fy - 20;
  const rx = 15;
  const ry = 24;
  for (let y = -ry; y <= ry; y++) {
    const half = Math.round(rx * Math.sqrt(1 - (y * y) / (ry * ry)));
    // the shell's rim, and a faint fill inside
    box(fx - half, cy + y, 1, 1, edge, a);
    box(fx + half, cy + y, 1, 1, edge, a);
    if (half > 1) box(fx - half + 1, cy + y, half * 2 - 1, 1, c.glow, flare ? 0.22 : 0.07);
    if (f.school === "thunder" ? y % 8 === 0 : (y + Math.round(f.poseT * 12)) % 9 === 0) {
      // bands across the bell; facets drifting up the mirror
      const w = f.school === "thunder" ? half * 2 + 1 : Math.max(1, Math.round(half * 0.8));
      box(fx - (f.school === "thunder" ? half : Math.round(half * 0.4)), cy + y, w, 1, edge, a * 0.45);
    }
  }
  box(fx - 3, cy - ry, 7, 1, edge, a);
  box(fx - 3, cy + ry, 7, 1, edge, a);
}

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
  if (f.barrier > 0) dome(box, f, fx, fy);
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
/**
 * Dust a body kicked up where it hit the floor: specks thrown out to both sides along the ground, rising a little and
 * settling as they fade. `list` is the feel's dust: x, strength 0 to 1, and k, how far through its life.
 */
export function drawDust(ctx, list, ground, s) {
  for (const d of list) {
    const n = 2 + Math.round(3 * d.strength);
    ctx.globalAlpha = 0.8 * (1 - d.k);
    for (let i = 0; i < n; i++) {
      const out = Math.round((3 + i * 3) * Math.sqrt(d.k) * (0.6 + d.strength));
      const rise = Math.round(Math.sin(Math.PI * d.k) * (1 + ((i * 7) % 3)) * d.strength * 2);
      ctx.fillStyle = i % 2 ? DUST_COLOR.light : DUST_COLOR.dark;
      for (const side of [-1, 1]) ctx.fillRect(Math.round(d.x + side * out) * s, (ground - 1 - rise) * s, s * (i < 2 ? 2 : 1), s);
    }
  }
  ctx.globalAlpha = 1;
}
const DUST_COLOR = { light: "#a89a88", dark: "#6e6458" };

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

/** How tall the fight's bars are, in game pixels: its medallions are this wide and tall. */
export const BARS_TALL = 26;

/**
 * The fight's bars, with no words: for each fighter a bronze medallion with its face, then its life (what a blow
 * just took drains away behind it), its qi under that, and its rage as beads that blaze once full (its ultimate
 * is ready). Between the two, a bronze plaque with a diamond for each round won. `top` is the bars' top edge in
 * game pixels.
 */
export function drawBars(ctx, duel, s, top) {
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
  top += narrow ? 13 : 0;
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
