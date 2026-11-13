// Where the duel is fought: a stone dais high in the mountains at night. A moon over ranges of peaks that fade into
// the distance, a pagoda on a far summit with one lamp lit, pines on the near ridge, and bands of mist drifting
// between the ranges. Each layer is drawn once into a canvas at one pixel per game pixel; only the mist moves.

const SKY = ["#0b0a1c", "#110f27", "#18153a", "#221c4a", "#2e2560", "#3d3174", "#4f3f86"];
const MOON = { face: "#efe2bf", shade: "#d3c29a", crater: "#c2b08a", halo: "#cfc6ff" };
/** The ranges, far to near: how high their peaks rise, as a share of the sky over the horizon; each nearer one is
 *  lower and darker, the far ones paled by the air between. */
const RANGES = [
  { color: "#3a3170", top: 0.55, rough: 1.0 },
  { color: "#272157", top: 0.38, rough: 0.8 },
  { color: "#171434", top: 0.2, rough: 0.6 },
];
const MIST = "#a9a4dc";
const LAMP = "#ffb85a";
const STONE = { lip: "#5d5670", top: "#3b3549", slab: "#2c2838", joint: "#211e2b", face: "#1b1824", shadow: "#120f1a" };

/** The backdrop for a w x h scene whose floor is at `ground`, from a fixed seed so it is the same on every load. The
 *  moon hangs below `clear`, the band at the top the fight's bars sit in. */
export function createScene(w, h, ground, clear = 0) {
  const rand = seeded(11);
  const layer = () => {
    const c = Object.assign(document.createElement("canvas"), { width: w, height: h });
    const ctx = c.getContext("2d");
    return { c, ctx };
  };
  const horizon = ground - 4;

  const sky = layer();
  for (let y = 0; y < horizon; y++) {
    // banded, with a dithered edge between bands
    const f = (y / horizon) * (SKY.length - 1);
    const i = Math.floor(f);
    const dither = (f - i) > ((y * 7 + 3) % 4) / 4 ? 1 : 0;
    for (let x = 0; x < w; x += 2) {
      sky.ctx.fillStyle = SKY[Math.min(SKY.length - 1, i + (((x >> 1) + y) % 2 === 0 ? dither : 0))];
      sky.ctx.fillRect(x, y, 2, 1);
    }
  }
  for (let i = 0; i < Math.floor((w * horizon) / 230); i++) {
    const x = Math.floor(rand() * w);
    const y = Math.floor(rand() * horizon * 0.75);
    sky.ctx.globalAlpha = 0.25 + rand() * 0.6 * (1 - y / horizon);
    sky.ctx.fillStyle = rand() < 0.15 ? "#ffe9c0" : "#e6e2ff";
    sky.ctx.fillRect(x, y, 1, 1);
  }
  sky.ctx.globalAlpha = 1;
  const moon = Math.round(Math.max(14, Math.min(24, h * 0.1)));
  drawMoon(sky.ctx, Math.round(w * 0.74), Math.max(Math.round(Math.min(46, horizon * 0.3)), clear + moon + 4), moon);

  const ranges = RANGES.map((r, k) => {
    const l = layer();
    const peak = horizon * (1 - r.top);
    // the ridge sits between its peaks and the horizon
    const phase = [rand() * 6, rand() * 6, rand() * 6];
    const ridge = (x) =>
      Math.round(
        peak +
          (horizon - peak) * 0.55 * (1 - r.rough * Math.abs(Math.sin(x * 0.011 + phase[0]))) +
          8 * Math.sin(x * 0.043 + phase[1]) * r.rough +
          3 * Math.sin(x * 0.17 + phase[2]),
      );
    l.ctx.fillStyle = r.color;
    for (let x = 0; x < w; x++) l.ctx.fillRect(x, ridge(x), 1, ground - ridge(x));
    // a lit pagoda on the far range's highest point
    if (k === 0) {
      let best = 0;
      for (let x = Math.floor(w * 0.1); x < w * 0.6; x++) if (ridge(x) < ridge(best) || best === 0) best = x;
      drawPagoda(l.ctx, best, ridge(best), r.color);
    }
    // pines along the nearest ridge
    if (k === RANGES.length - 1) {
      for (let x = 2; x < w; x += 3 + Math.floor(rand() * 9)) drawPine(l.ctx, x, ridge(x) + 1, 5 + Math.floor(rand() * 8), r.color);
    }
    return l;
  });

  const floor = layer();
  const f = floor.ctx;
  f.fillStyle = STONE.slab;
  f.fillRect(0, ground, w, h - ground);
  f.fillStyle = STONE.lip;
  f.fillRect(0, ground, w, 1);
  f.fillStyle = STONE.top;
  f.fillRect(0, ground + 1, w, 2);
  // joints between the slabs, staggered from course to course
  f.fillStyle = STONE.joint;
  for (let y = ground + 3, row = 0; y < h; y += 6, row++) {
    f.fillRect(0, y, w, 1);
    for (let x = (row % 2) * 9; x < w; x += 18) f.fillRect(x, y, 1, 6);
  }
  f.fillStyle = STONE.shadow;
  f.globalAlpha = 0.5;
  f.fillRect(0, h - 3, w, 3);
  f.globalAlpha = 1;

  // mist bands: one between each pair of ranges and one along the foot of the near ridge
  const mist = [0.7, 0.84, 0.96].map((at, i) => ({
    y: Math.round(horizon * at),
    speed: 1.5 + i * 1.2,
    puffs: Array.from({ length: Math.ceil(w / 40) + 2 }, () => ({ x: rand() * (w + 80), len: 30 + rand() * 60, thick: 2 + Math.floor(rand() * 3) })),
    alpha: 0.07 + i * 0.03,
  }));

  return { w, h, ground, sky, ranges, floor, mist };
}

/** Draw the scene at time t (seconds) on a canvas `s` times the game size. Fighters and effects go on top. */
export function drawScene(ctx, scene, t, s) {
  const { w, h } = scene;
  ctx.imageSmoothingEnabled = false;
  const put = (l) => ctx.drawImage(l.c, 0, 0, w * s, h * s);
  put(scene.sky);
  scene.ranges.forEach((r, i) => {
    put(r);
    drawMist(ctx, scene, scene.mist[i], t, s);
  });
  put(scene.floor);
}

function drawMist(ctx, scene, band, t, s) {
  ctx.fillStyle = MIST;
  ctx.globalAlpha = band.alpha;
  const span = scene.w + 80;
  for (const p of band.puffs) {
    const x = Math.round(((((p.x + t * band.speed) % span) + span) % span) - 40);
    ctx.fillRect(x * s, band.y * s, Math.round(p.len) * s, p.thick * s);
    ctx.fillRect((x + 6) * s, (band.y - 1) * s, Math.round(p.len * 0.6) * s, s);
  }
  ctx.globalAlpha = 1;
}

function drawMoon(ctx, cx, cy, r) {
  // a soft halo, then the disc with its shaded limb and a few craters
  for (let k = 3; k >= 1; k--) {
    ctx.globalAlpha = 0.05 * k;
    ctx.fillStyle = MOON.halo;
    disc(ctx, cx, cy, r + 4 * (4 - k));
  }
  ctx.globalAlpha = 1;
  ctx.fillStyle = MOON.shade;
  disc(ctx, cx, cy, r);
  ctx.fillStyle = MOON.face;
  disc(ctx, cx - 2, cy - 1, r - 2);
  ctx.fillStyle = MOON.crater;
  for (const [dx, dy, cr] of [[-0.3, -0.25, 0.18], [0.25, 0.1, 0.13], [-0.05, 0.4, 0.1], [0.35, -0.4, 0.08]]) {
    disc(ctx, Math.round(cx + dx * r), Math.round(cy + dy * r), Math.max(1, Math.round(cr * r)));
  }
}

function disc(ctx, cx, cy, r) {
  for (let y = -r; y <= r; y++) {
    const half = Math.round(Math.sqrt(r * r - y * y));
    ctx.fillRect(cx - half, cy + y, half * 2 + 1, 1);
  }
}

/** A three-roofed pagoda standing on (x, y), in the range's own colour with one window lit. */
function drawPagoda(ctx, x, y, color) {
  ctx.fillStyle = color;
  const tier = (base, wide, tall) => {
    ctx.fillRect(x - wide, base - 1, wide * 2 + 1, 1);
    ctx.fillRect(x - wide - 1, base - 2, 2, 1);
    ctx.fillRect(x + wide, base - 2, 2, 1);
    ctx.fillRect(x - wide + 2, base - tall, (wide - 2) * 2 + 1, tall - 1);
  };
  tier(y + 2, 7, 6);
  tier(y - 4, 6, 5);
  tier(y - 9, 4, 5);
  ctx.fillRect(x, y - 17, 1, 4);
  ctx.fillStyle = LAMP;
  ctx.fillRect(x, y - 2, 1, 2);
  ctx.globalAlpha = 0.25;
  ctx.fillRect(x - 1, y - 3, 3, 4);
  ctx.globalAlpha = 1;
}

/** A pine of height `tall` standing on (x, y). */
function drawPine(ctx, x, y, tall, color) {
  ctx.fillStyle = color;
  ctx.fillRect(x, y - tall, 1, tall);
  for (let i = 0; i < tall - 1; i++) {
    // widest at the foot, tiers stepping in toward the top
    const half = Math.floor(((tall - i) * 3) / tall) + (i % 2);
    ctx.fillRect(x - half, y - i - 1, half * 2 + 1, 1);
  }
}

function seeded(seed) {
  let s = seed;
  return () => (s = (s * 1103515245 + 12345) % 2147483648) / 2147483648;
}
