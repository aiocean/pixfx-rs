// How the duel looks around its fighters: the qi streaks and numbers of the fight, and the fight's bars. The
// fighters themselves (their poses, the marks that hang on them, the fire, lightning and frost over them, the dust
// they raise) are drawn by the engine (pixfx-fighter, through the wasm `Sprites`); this module tells it what each
// fighter is doing, in the duel's own terms. Everything is in game pixels times `s`, the canvas scale.
import { lookColors } from "../pkg/pixfx-canvas2d.js";
import { RAGE, SPELLS, buffsOf, skillBar } from "./duel.js";

// ---------- where the HUD's parts stand (drawing and the pointer both read these) ----------

/** Where slot `i` of fighter f's skill bar stands, in game pixels: icons `n` square, standing on row `bottom`. */
export function slotRect(duel, f, i, bottom, n) {
  const size = n + 4;
  return { x: f.index === 0 ? duel.left + i * (size + 2) : duel.right - size - i * (size + 2), y: bottom - size, w: size, h: size };
}

/** Where buff tile `i` of fighter f stands, under the bars that start at row `top`. */
export function buffRect(duel, f, i, top) {
  return { x: f.index === 1 ? duel.right - 9 - i * 11 : duel.left + i * 11, y: top + BARS_TALL + 2, w: 9, h: 9 };
}

/**
 * What of the HUD lies under game pixel (px, py): `{ kind: "skill", fighter, slot }`, `{ kind: "buff", fighter, buff }`
 * (the buff from `buffsOf`), or null. The bars start at row `top`; the skill icons are `n` square on row `bottom`.
 */
export function hudAt(duel, px, py, top, bottom, n) {
  const inside = (r) => px >= r.x && px < r.x + r.w && py >= r.y && py < r.y + r.h;
  for (const f of duel.fighters) {
    for (let i = 0; i < skillBar(f).length; i++) if (inside(slotRect(duel, f, i, bottom, n))) return { kind: "skill", fighter: f.index, slot: i };
    const list = buffsOf(f);
    for (let i = 0; i < list.length; i++) if (inside(buffRect(duel, f, i, top))) return { kind: "buff", fighter: f.index, buff: list[i] };
  }
  return null;
}
import { ICON_CELLS, ICON_COLUMNS } from "./skill-icons.js";

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
    x: f.x, air: f.air, pose: f.pose, poseT: f.poseT, face: f.face, vx: f.vx, vy: f.vy, out: f.hp <= 0, flash: f.flash > 0, untouchable: f.untouchable,
    frozen: f.frozen > 0, shocked: f.shocked > 0, hitT: f.hitT, hitPower: f.hitPower, dodged: f.dodged, jolt, act,
  };
}

/** What hangs on fighter f: a ward's shell, a barrier, and fire, lightning or frost over its body. None once it is
 *  knocked out. */
function marks(f, time) {
  if (f.hp <= 0) return null;
  return {
    // the signs of what it carries (a burn, defence down, silence, chill, fury) are on the HUD under its bars
    shield: f.shield > 0 || f.charges > 0, barrier: f.barrier > 0, rang: f.rang > 0, burning: false, defDown: false,
    silence: false, chilled: false, fury: 0, poseT: f.poseT, dome: f.school === "thunder" ? "banded" : "faceted",
    burn: !!f.burn, shock: f.shocked > 0, ice: f.frozen > 0, t: time, seed: f.index + 1,
  };
}

/**
 * Each fighter on a picture of its own, feet on row `ground`, and then the dust of the fight (`feel`); `jolts` from
 * `feel.jolts()`. Returns the pictures to draw (`pixfx.drawPicture`), the fighters' by their index: an effect that
 * wraps a fighter reads its picture.
 */
export function paintFighters(sprites, duel, feel, ground, jolts) {
  const fighters = duel.fighters.map((f) => {
    sprites.begin();
    sprites.fighter(f.school, appearance(f, jolts[f.index]), ground, marks(f, duel.time));
    return sprites.picture();
  });
  sprites.begin();
  sprites.dust(feel, ground);
  return { fighters, dust: sprites.picture() };
}

/** A fighter's picture as the sprite an effect wraps (`effect.wrap`): where it shows and its colour bytes. */
export function actorOf(picture) {
  const c = picture?.color;
  if (!c) return null;
  return { at: { x: picture.x, y: picture.y, w: picture.w, h: picture.h }, rgba: new Uint8Array(c.buffer, c.byteOffset, c.byteLength) };
}

/**
 * Each qi streak (from the engine's `FightFeel.streaks()`): Thunder's a crooked line of lightning, Frost's a straight shimmer
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
  counter: { size: 11, color: "#ffd84a" },
  punish: { size: 12, color: "#ff3d6a" },
};

/** The numbers over the fighters (from the engine's `FightFeel.numbers()`): damage, burns, heals, wards, dodges and the words over counters and punishes. */
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
const BAR = { back: "#160f1c", lost: "#5a1a14", trail: "#fff0d0", qi: "#7fb4ff", qiLight: "#c8e0ff", rage: "#ff5a32", rageLight: "#ffb070" };
/** Each buff's 7 by 7 icon and colour, in the tile under the bars (see `buffsOf`). */
const BUFF = {
  fury: { color: "#ffc23d", rows: ["...#...", "..###..", "..###..", "..###..", ".#####.", "...#...", "..###.."] },
  unyielding: { color: "#ff6a5a", rows: [".##.##.", "#######", "#######", ".#####.", "..###..", "...#...", "......."] },
  attune: { color: "#c89bff", rows: ["...#...", "..#.#..", ".#.#.#.", "#.###.#", ".#.#.#.", "..#.#..", "...#..."] },
  ward: { color: "#a6ecff", rows: ["#######", "#.....#", "#.###.#", "#.###.#", ".#.#.#.", "..#.#..", "...#..."] },
  barrier: { color: "#f0cf7a", rows: ["..###..", ".#...#.", "#.....#", "#.....#", "#.....#", "#######", "......."] },
  grit: { color: "#b8893c", rows: ["..###..", ".#####.", "##.####", "#######", "####.##", ".#####.", "......."] },
  burn: { color: "#ff9a3a", rows: ["...#...", "..##...", "..###..", ".####.#", ".######", "#######", ".#####."] },
  chill: { color: "#bff2ff", rows: ["...#...", ".#.#.#.", "..###..", "#######", "..###..", ".#.#.#.", "...#..."] },
  shock: { color: "#ffe14a", rows: ["....##.", "...##..", "..##...", ".#####.", "...##..", "..##...", ".##...."] },
  frozen: { color: "#a6ecff", rows: ["#######", "#.#...#", "#..#..#", "#...#.#", "#.#...#", "#..#..#", "#######"] },
  defDown: { color: "#8f8aa8", rows: [".##.##.", "#..#..#", "#..#..#", "#.#...#", ".#.#.#.", "..#.#..", "...#..."] },
  silence: { color: "#c9c3e6", rows: ["#.....#", ".#...#.", "..#.#..", "...#...", "..#.#..", ".#...#.", "#.....#"] },
  sealed: { color: "#c9c3e6", rows: ["..###..", ".#...#.", ".#...#.", "#######", "###.###", "###.###", "#######"] },
};
/** A buff tile's rim: green for a good one, red for a bad one. */
const BUFF_RIM = { good: "#3fae5a", bad: "#c8322a" };
/** Life each bar last showed, so a blow's loss drains away behind the bar instead of vanishing. */
const shown = new WeakMap();

/** How tall the fight's bars are, in game pixels: its medallions are this wide and tall. */
export const BARS_TALL = 26;

/**
 * The fight's bars, with no words: for each fighter a bronze medallion with its face, then its life (what a blow
 * just took drains away behind it), its qi in a blue bar of its own under that, its rage in a red bar of segments
 * that blaze once full (its ultimate is ready), and its buffs in tiles under it all. Between the two, a bronze plaque
 * with the rounds each has won. `top` is the bars' top edge in game pixels; `show(picture)` draws the faces
 * `sprites` painted (`pixfx.drawPicture`).
 */
export function drawBars(ctx, sprites, show, duel, s, top) {
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
    // qi (mana): a blue bar in its own frame under the life bar
    box(left - 1, top + 12, width + 2, 4, BRONZE.deep);
    box(left, top + 13, width, 2, BAR.back);
    const qi = Math.round((width * f.qi) / 100);
    box(from(qi), top + 13, qi, 2, BAR.qi);
    box(from(qi), top + 13, qi, 1, BAR.qiLight);
    // rage: a red bar of its own under the qi, a segment per point, blazing once full
    const full = f.rage >= RAGE;
    const blaze = full && Math.sin(duel.time * 12) > 0;
    // a narrow arena's plaque hangs between the rage bars: they shrink to leave it room
    const seg = Math.max(3, Math.min(9, Math.floor((width - (narrow ? 20 : 0) + 1) / RAGE)));
    const rageW = RAGE * seg - 1;
    const rx = right ? left + width - rageW : left;
    if (full) box(rx - 2, top + 16, rageW + 4, 8, BAR.rage, blaze ? 0.35 : 0.15);
    box(rx - 1, top + 17, rageW + 2, 6, BRONZE.deep);
    for (let i = 0; i < RAGE; i++) {
      const x = right ? rx + rageW - seg + 1 - i * seg : rx + i * seg;
      const on = i < f.rage;
      box(x, top + 18, seg - 1, 4, on ? (blaze ? "#ffffff" : BAR.rage) : BAR.back);
      if (on) box(x, top + 18, seg - 1, 1, blaze ? "#ffffff" : BAR.rageLight);
    }
    // what it carries, in tiles under the medallion and the bars, from its own edge inward (after Path of Exile)
    buffs(duel, f, top, box);
  }
  // the plaque in the middle: the rounds each fighter has won, as a number in its own colour on its own half
  const mid = Math.round((duel.left + duel.right) / 2);
  top += narrow ? 13 : 0;
  box(mid - 18, top + 2, 36, 9, BRONZE.deep);
  box(mid - 17, top + 3, 34, 7, BRONZE.dark);
  box(mid - 17, top + 3, 34, 1, BRONZE.mid);
  box(mid, top + 4, 1, 5, BRONZE.mid);
  for (const f of duel.fighters) {
    const digits = String(Math.min(999, duel.wins[f.index])).split("").map(Number);
    const w = digits.length * 4 - 1;
    // centred in its half, which is 17 pixels wide
    const x0 = f.index === 0 ? mid - 17 + Math.floor((17 - w) / 2) : mid + 1 + Math.ceil((17 - w) / 2);
    const col = colors(f.school).glow;
    digits.forEach((d, i) =>
      DIGIT[d].forEach((row, r) => {
        for (let k = 0; k < 3; k++) if (row[k] === "#") box(x0 + i * 4 + k, top + 4 + r, 1, 1, col);
      }));
  }
  sprites.begin();
  for (const m of faces) {
    const look = { ...appearance(m.f, [0, 0]), pose: "idle", face: m.f.index === 0 ? 1 : -1, flash: false, vx: 0, act: null };
    sprites.portrait(m.f.school, look, m.x, m.y, m.size, m.f.hp <= 0 ? 0.35 : 1);
  }
  show(sprites.picture());
}

/** Fighter f's buffs (`buffsOf`) as 9 by 9 tiles in a row under its bars (`buffRect`; the bars start at row `top`):
 *  an icon on a dark field, a rim green for a good one and red for a bad one, and its stacks as a digit in its lower
 *  right corner. */
function buffs(duel, f, barsTop, box) {
  buffsOf(f).forEach((b, i) => {
    const look = BUFF[b.id];
    if (!look) return;
    const { x, y: top } = buffRect(duel, f, i, barsTop);
    box(x, top, 9, 9, b.good ? BUFF_RIM.good : BUFF_RIM.bad);
    box(x + 1, top + 1, 7, 7, "#120d18");
    look.rows.forEach((row, r) => {
      for (let k = 0; k < 7; k++) if (row[k] === "#") box(x + 1 + k, top + 1 + r, 1, 1, look.color);
    });
    if (b.stacks > 0) {
      // inside the tile's lower right corner, so a tile at the screen's edge keeps its digit
      const d = DIGIT[Math.min(9, b.stacks)];
      box(x + 4, top + 2, 5, 7, "#120d18");
      d.forEach((row, r) => {
        for (let k = 0; k < 3; k++) if (row[k] === "#") box(x + 5 + k, top + 3 + r, 1, 1, "#ffffff");
      });
    }
  });
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

// ---------- the skill bars ----------

/** Digits 0 to 9, 3 by 5: the seconds an empty slot still waits, a buff's stacks and the rounds won. */
const DIGIT = [
  ["###", "#.#", "#.#", "#.#", "###"],
  [".#.", "##.", ".#.", ".#.", "###"],
  ["##.", "..#", ".#.", "#..", "###"],
  ["##.", "..#", ".#.", "..#", "##."],
  ["#.#", "#.#", "###", "..#", "..#"],
  ["###", "#..", "##.", "..#", "##."],
  [".##", "#..", "###", "#.#", "###"],
  ["###", "..#", ".#.", ".#.", ".#."],
  ["###", "#.#", "###", "#.#", "###"],
  ["###", "#.#", "###", "..#", "##."],
];
/** A trait's mark in the corner of its icon, 5 by 5: swift chevrons, a heavy weight, a piercing arrow, a lingering
 * drop, a cheap gem. */
const TRAIT_MARK = {
  swift: ["#..#.", ".#..#", "..#..", ".#..#", "#..#."],
  heavy: [".###.", "..#..", "#####", "#####", "#####"],
  piercing: ["..#..", ".###.", "#.#.#", "..#..", "..#.."],
  lingering: ["..#..", ".###.", "#####", "#####", ".###."],
  cheap: ["..#..", ".#.#.", "#...#", ".#.#.", "..#.."],
};
/** A rare skill's frame, and an epic one's (pulsing to white): the same on both schools, apart from their glows. */
const RARE = "#4fa3ff";
const EPIC = "#c36bff";
/** A legendary skill's frame, pulsing from orange to pale gold, and the word a tier-up floats per tier. */
const LEGENDARY = ["#ff8a1a", "#ffe08a"];
const TIER_WORD = { rare: "RARE", epic: "EPIC", legendary: "LEGENDARY" };
const TIER_COLOR = { rare: RARE, epic: "#e2a8ff", legendary: "#ffb040" };
const TRAIT_COLOR = { swift: "#8dffa8", heavy: "#ff9a3a", piercing: "#ffffff", lingering: "#c89bff", cheap: "#ffd84a" };
/** How stale a bar skill is (0 to 1) when its icon starts to flicker before it leaves the bar. */
const STALE_FLICKER = 2 / 3;
/** The word a lucky cast floats over its slot. */
const LUCKY_WORD = { double: "DOUBLE", free: "FREE", again: "AGAIN" };
/** How long a lucky word floats, in seconds, and a tier-up's. */
const LUCKY_TIME = 1.2;
const TIER_TIME = 1.2;
const SLOT = { field: "#1a1424", shade: "#06050c" };
/** How long a slot flashes when a spell in it is cast, and glints when its cooldown ends, in seconds. */
const GLINT = 0.25;
/** For each fighter, each slot's cooldown last frame and when it last came off cooldown. */
const glints = new WeakMap();

/**
 * Shade a share of an `n`-square from twelve o'clock, one pixel at a time: `share` > 0 is the first part of the turn
 * clockwise, `share` < 0 the last part (what a cooldown still has to run).
 */
function sweep(box, x, y, n, share, col, alpha) {
  const mid = (n - 1) / 2;
  for (let py = 0; py < n; py++) {
    for (let px = 0; px < n; px++) {
      const turn = (Math.atan2(px - mid, mid - py) / (2 * Math.PI) + 1) % 1;
      if (share >= 0 ? turn < share : turn >= 1 + share) box(x + px, y + py, 1, 1, col, alpha);
    }
  }
}

/**
 * Each fighter's skill bar on the floor of the dais, under its own side: slots 1, 2 and 3 and the ultimate, the
 * ultimate nearest the middle. A slot holds one cast: it shows its skill's icon, its traits' marks in the icon's
 * corner, and its tier in its frame (bronze, blue, purple pulsing, orange pulsing for legendary); an evolved
 * skill's icon is ringed in white. An icon is full while it can be cast and dimmer while short of qi or silenced; a
 * cooldown darkens it and sweeps back clockwise as it runs out; a cast flashes it and the end of a cooldown glints its
 * frame. An empty slot shows the seconds until its next skill while its frame lights clockwise. The ultimate's frame
 * fills with the school's glow as rage builds, and blazes once full. A lucky cast flashes its slot gold and floats
 * its word above it; a skill a duplicate draw lifts a tier flashes in its new tier's colour and floats its name. The slots stand on row `bottom`, the foot of the scene. Each icon is `n` game pixels square,
 * drawn from `icons`, a skill icon atlas: its `image` and the `size` of each icon in it, in the atlas's own pixels.
 */
export function drawSkills(ctx, duel, s, bottom, n, icons) {
  const box = (x, y, w, h, col, a = 1) => {
    ctx.globalAlpha = a;
    ctx.fillStyle = col;
    ctx.fillRect(x * s, y * s, w * s, h * s);
    ctx.globalAlpha = 1;
  };
  const glyph = (rows, x, y, col, z = 1) => {
    rows.forEach((row, r) => {
      for (let k = 0; k < row.length; k++) if (row[k] === "#") box(x + k * z, y + r * z, z, z, col);
    });
  };
  // an icon in a two-pixel frame, standing on the foot of the scene
  const size = n + 4;
  const top = bottom - size;
  ctx.imageSmoothingEnabled = false;
  for (const f of duel.fighters) {
    const c = colors(f.school);
    const seen = glints.get(f) ?? { left: [], at: [], spell: [], tier: [], up: [] };
    glints.set(f, seen);
    /** A word floating up over slot i, `since` seconds into its `time`, kept inside the arena. */
    const float = (word, color, i, since, time) => {
      const { x } = slotRect(duel, f, i, bottom, n);
      ctx.font = `700 ${8 * s}px "Pixelify Sans", monospace`;
      ctx.textAlign = "center";
      ctx.textBaseline = "alphabetic";
      const half = ctx.measureText(word).width / 2 + s;
      const tx = Math.min(Math.max((x + size / 2) * s, duel.left * s + half), duel.right * s - half);
      const ty = (top - 3 - since * 8) * s;
      ctx.globalAlpha = 1 - since / time;
      ctx.fillStyle = "#13101b";
      for (const [ox, oy] of [[-1, 0], [1, 0], [0, -1], [0, 1], [1, 1]]) ctx.fillText(word, tx + ox * s, ty + oy * s);
      ctx.fillStyle = color;
      ctx.fillText(word, tx, ty);
      ctx.globalAlpha = 1;
    };
    skillBar(f).forEach((slot, i) => {
      const { x } = slotRect(duel, f, i, bottom, n);
      if (seen.left[i] > 0 && slot.left === 0) seen.at[i] = duel.time;
      seen.left[i] = slot.left;
      // the same skill a tier higher than last frame: a duplicate draw lifted it
      if (slot.spell && slot.spell === seen.spell[i] && TIER_WORD[slot.rarity] && slot.rarity !== seen.tier[i]) seen.up[i] = duel.time;
      seen.spell[i] = slot.spell;
      seen.tier[i] = slot.rarity;
      const pulse = Math.sin(duel.time * 6) > 0;
      // the frame: bronze, or the tier's colour
      const ring = slot.rarity === "legendary" ? LEGENDARY[pulse ? 1 : 0] : slot.rarity === "epic" ? (pulse ? "#f4e0ff" : EPIC) : slot.rarity === "rare" ? RARE : BRONZE.mid;
      box(x, top, size, size, BRONZE.deep);
      frame(box, x + 1, top + 1, size - 2, ring);
      box(x + 1, top + 1, size - 2, 1, slot.rarity === "common" || slot.empty ? BRONZE.light : "#ffffff", slot.rarity === "common" || slot.empty ? 1 : 0.6);
      box(x + 2, top + 2, n, n, SLOT.field);
      const cell = ICON_CELLS[f.school]?.[slot.spell];
      if (slot.empty || cell === undefined) {
        // the wait for the next skill: its frame lights clockwise, and the seconds left stand in the middle
        if (slot.full > 0) {
          ring2(box, x, top, size, 1 - slot.left / slot.full, c.glow);
          const z = n >= 16 ? 2 : 1;
          glyph(DIGIT[Math.min(9, Math.ceil(slot.left))], x + 2 + Math.floor((n - 3 * z) / 2), top + 2 + Math.floor((n - 5 * z) / 2), BRONZE.light, z);
        }
        return;
      }
      if (slot.ultimate) {
        // rage fills the frame from the foot up, and blazes once full
        const full = slot.ready;
        const blaze = full && Math.sin(duel.time * 12) > 0;
        const lit = Math.round(size * slot.charge);
        for (let py = size - lit; py < size; py++) {
          box(x, top + py, 2, 1, blaze ? "#ffffff" : c.glow);
          box(x + size - 2, top + py, 2, 1, blaze ? "#ffffff" : c.glow);
        }
        if (lit >= size) box(x, top, size, 2, blaze ? "#ffffff" : c.glow);
        if (lit > size - 2) box(x, top + size - 2, size, 2, blaze ? "#ffffff" : c.glow);
        box(x + 2, top + 2, n, n, SLOT.field);
      }
      // an icon that cannot be cast now (short of qi, silenced) is dimmer; a cooldown shows as its sweep; a skill
      // about to leave the bar unused flickers through its last stretch
      const leaving = (slot.stale ?? 0) > STALE_FLICKER && Math.floor(duel.time * 6) % 2 === 1;
      ctx.globalAlpha = (slot.ready || slot.left > 0 ? 1 : 0.6) * (leaving ? 0.35 : 1);
      const k = icons.size;
      ctx.drawImage(icons.image, (cell % ICON_COLUMNS) * k, Math.floor(cell / ICON_COLUMNS) * k, k, k, (x + 2) * s, (top + 2) * s, n * s, n * s);
      ctx.globalAlpha = 1;
      if (slot.ultimate && !slot.ready && slot.left === 0) box(x + 2, top + 2, n, n, SLOT.shade, 0.35);
      // an evolved skill: a white ring round its icon
      if (slot.evolved) frame(box, x + 2, top + 2, n, "#ffffff", 0.8);
      // its traits' marks, in its top corner
      (n >= 16 ? slot.traits : slot.traits.slice(0, 1)).forEach((t, j) => {
        const gx = f.index === 0 ? x + 2 + j * 6 : x + 2 + n - 5 - j * 6;
        box(gx - 1, top + 1, 7, 7, SLOT.shade, 0.75);
        glyph(TRAIT_MARK[t], gx, top + 2, TRAIT_COLOR[t]);
      });
      // the cooldown: the share still to run stays dark, sweeping clockwise from twelve o'clock
      if (slot.left > 0 && slot.full > 0) {
        sweep(box, x + 2, top + 2, n, -slot.left / slot.full, SLOT.shade, 0.6);
        const since = slot.full - slot.left;
        if (since < GLINT) box(x + 2, top + 2, n, n, "#ffffff", 0.6 * (1 - since / GLINT));
      }
      const ago = duel.time - (seen.at[i] ?? -Infinity);
      if (ago < GLINT && slot.ready) frame(box, x, top, size, "#ffffff", 0.8 * (1 - ago / GLINT));
      // a tier-up: the frame flashes in the new tier's colour, and the tier's name floats over the slot
      const up = duel.time - (seen.up[i] ?? -Infinity);
      if (up < TIER_TIME) {
        if (up < 0.4) {
          frame(box, x, top, size, TIER_COLOR[slot.rarity], 1 - up / 0.4);
          box(x + 2, top + 2, n, n, "#ffffff", 0.5 * (1 - up / 0.4));
        }
        float(TIER_WORD[slot.rarity], TIER_COLOR[slot.rarity], i, up, TIER_TIME);
      }
    });
    // a lucky cast: its slot flashes gold, and its word floats up over the bar
    const luck = f.lucky;
    const since = luck ? duel.time - luck.at : Infinity;
    if (since < LUCKY_TIME) {
      const i = luck.slot ?? 0;
      const { x } = slotRect(duel, f, i, bottom, n);
      if (since < 0.3) frame(box, x, top, size, "#ffd84a", 1 - since / 0.3);
      float(LUCKY_WORD[luck.kind], "#ffd84a", i, since, LUCKY_TIME);
    }
  }
}

/** A one-pixel frame round the edge of an `n`-square at (x, y), in `col`. */
function frame(box, x, y, n, col, alpha = 1) {
  box(x, y, n, 1, col, alpha);
  box(x, y + n - 1, n, 1, col, alpha);
  box(x, y + 1, 1, n - 2, col, alpha);
  box(x + n - 1, y + 1, 1, n - 2, col, alpha);
}

/** Light the first `share` of an `n`-square's outer ring, clockwise from twelve o'clock. */
function ring2(box, x, y, n, share, col) {
  const mid = (n - 1) / 2;
  for (let py = 0; py < n; py++) {
    for (let px = 0; px < n; px++) {
      if (px > 1 && px < n - 2 && py > 1 && py < n - 2) continue;
      const turn = (Math.atan2(px - mid, mid - py) / (2 * Math.PI) + 1) % 1;
      if (turn < share) box(x + px, y + py, 1, 1, col);
    }
  }
}
