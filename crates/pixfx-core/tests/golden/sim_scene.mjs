// Prints the goldens of tests/sim.rs, each hash the FNV-1a 64 of a 160x120 frame placed at (80, 100) (colour words
// then light words, 4 little-endian bytes each):
// - `particle_scene_matches_pixelfx`: `(step, count, hash)` rows for every particle kind (a FIRE that leaves SMOKE,
//   BRANDs trailing flame whose landings start blazes), a still MOTE, a back-layer SPARK and a light, at 1/120 s;
//   then the most blazes burning at once;
// - `debris_scene_matches_pixelfx`: `(step, count, hash)` rows for a SHARD, a CHUNK and a DRIP, at 1/120 s;
// - `sim_shapes_match_pixelfx`: one hash for sparkle, flare, band and crystal.
// Usage: node sim_scene.mjs <path to a pixelfx checkout>   (goldens taken at pixelfx 044e12a)
import { pathToFileURL } from "node:url";
import { join, resolve } from "node:path";

const root = process.argv[2];
if (!root) throw new Error("usage: node sim_scene.mjs <pixelfx dir>");
const load = (f) => import(pathToFileURL(join(resolve(root), "src/core", f)).href);
const { createFrame } = await load("fx.js");
const { Painter } = await load("painter.js");
const { WHITE, hex } = await load("color.js");
const { Rng } = await load("math.js");
const { Particles, P, PF, Blazes, Lights, Bits, B, sparkle, flare, band, crystal } = await load("sim.js");

const hash = (f) => {
  let h = 0xcbf29ce484222325n;
  for (const buf of [f.color, f.light])
    for (const w of buf)
      for (let s = 0; s < 32; s += 8) h = ((h ^ BigInt((w >>> s) & 255)) * 0x100000001b3n) & 0xffffffffffffffffn;
  return h;
};

const frame = () => {
  const f = createFrame(160, 120, 0, 0);
  return [f, new Painter(f).place(80, 100, false, 0)];
};

console.log("particle scene");
const R = new Rng(5);
let blazes;
const ps = new Particles({ rng: R, ground: () => 0, haze: "#88ccff", onLand: (x, y) => blazes.add(x, y - 0.25, R.range(0.9, 1.5), R.range(0.55, 0.85)) });
blazes = new Blazes(ps);
const lights = new Lights();
for (let k = 0; k <= 11; k++)
  ps.spawn(k, (k - 6) * 8, -20, R.range(-80, 80), R.range(-160, -40), R.range(0.6, 1.4), k === P.SMOKE || k === P.FIRE ? 0 : "#ffcc66", R.range(0.8, 2.6), k * 0.02, k === P.FIRE ? PF.SMOKES : k === P.SMOKE ? PF.BIG : 0);
ps.spawn(P.MOTE, 0, -10, 0, 0, 1);
ps.spawn(P.SPARK, 4, -30, 50, -50, 1, WHITE, 1, 0, PF.BACK);
ps.spawn(P.BRAND, -10, -40, 30, -80, 2, WHITE, 1);
lights.add(0, -20, 30, "#ffaa44", 0.8, 0.5);

let mostBlazes = 0;
const dt = 1 / 120, checkpoints = new Set([0, 12, 60, 150, 300]);
for (let step = 0; step <= 300; step++) {
  if (step > 0) {
    ps.update(dt);
    blazes.update(dt);
    lights.update(dt);
    mostBlazes = Math.max(mostBlazes, blazes.count);
  }
  if (!checkpoints.has(step)) continue;
  const [f, p] = frame();
  blazes.drawBack(p);
  ps.draw(p, "back");
  ps.draw(p, "front");
  lights.draw(p);
  console.log(`(${step}, ${ps.count}, 0x${hash(f).toString(16).padStart(16, "0")}),`);
}

console.log(`most blazes ${mostBlazes}`);

console.log("debris scene");
const bits = new Bits({ ground: () => 0 });
bits.add({ kind: B.SHARD, x: -20, y: -30, vx: 60, vy: -120, va: 9, s: 5, c: "#d8f4ff", bounce: 0.4, life: 2 });
bits.add({ kind: B.CHUNK, x: 0, y: -40, vx: -40, vy: -90, va: -5, s: 3, bounce: 0.3, drag: 0.5, life: 2 });
bits.add({ kind: B.DRIP, x: 20, y: -50, c: "#58b4ec", g: 0.5, life: 2 });
const debrisAt = new Set([0, 60, 120, 240]);
for (let step = 0; step <= 240; step++) {
  if (step > 0) bits.update(dt);
  if (!debrisAt.has(step)) continue;
  const [f, p] = frame();
  bits.draw(p);
  console.log(`(${step}, ${bits.count}, 0x${hash(f).toString(16).padStart(16, "0")}),`);
}

console.log("shapes");
const R5 = ["#0c1430", "#1f4f94", "#58b4ec", "#d8f4ff", "#ffffff"];
const R7 = ["#0c1430", "#162c5c", "#1f4f94", "#2f7fcc", "#58b4ec", "#9bdcff", "#d8f4ff"];
{
  const [f, p] = frame();
  sparkle(p, -40, -40, 5, hex("#ffe066"), hex("#b07a1a"), 0.6);
  flare(p, 0, -60, 12, 0.7, R5);
  band(p, 30, -30, 18, 0.4, 3, 0.9, R5, (g) => Math.sin(g) < 0, true);
  crystal(p, -20, -5, 24, 10, 3, R7, { glint: 0.5 });
  console.log(`0x${hash(f).toString(16).padStart(16, "0")}`);
}
