// Prints `(step, count, hash)` rows for the particle scene in tests/sim.rs `particle_scene_matches_pixelfx`: every
// particle kind (a FIRE that leaves SMOKE, a BRAND trailing flame), a still MOTE and a back-layer SPARK, stepped
// at 1/120 s; the hash is the FNV-1a 64 of the frame (colour words then light words, 4 little-endian bytes each).
// Usage: node sim_scene.mjs <path to a pixelfx checkout>   (goldens taken at pixelfx 044e12a)
import { pathToFileURL } from "node:url";
import { join, resolve } from "node:path";

const root = process.argv[2];
if (!root) throw new Error("usage: node sim_scene.mjs <pixelfx dir>");
const load = (f) => import(pathToFileURL(join(resolve(root), "src/core", f)).href);
const { createFrame } = await load("fx.js");
const { Painter } = await load("painter.js");
const { WHITE } = await load("color.js");
const { Rng } = await load("math.js");
const { Particles, P, PF } = await load("sim.js");

const hash = (f) => {
  let h = 0xcbf29ce484222325n;
  for (const buf of [f.color, f.light])
    for (const w of buf)
      for (let s = 0; s < 32; s += 8) h = ((h ^ BigInt((w >>> s) & 255)) * 0x100000001b3n) & 0xffffffffffffffffn;
  return h;
};

const R = new Rng(5);
const ps = new Particles({ rng: R, ground: () => 0, haze: "#88ccff" });
for (let k = 0; k <= 11; k++)
  ps.spawn(k, (k - 6) * 8, -20, R.range(-80, 80), R.range(-160, -40), R.range(0.6, 1.4), k === P.SMOKE || k === P.FIRE ? 0 : "#ffcc66", R.range(0.8, 2.6), k * 0.02, k === P.FIRE ? PF.SMOKES : k === P.SMOKE ? PF.BIG : 0);
ps.spawn(P.MOTE, 0, -10, 0, 0, 1);
ps.spawn(P.SPARK, 4, -30, 50, -50, 1, WHITE, 1, 0, PF.BACK);
ps.spawn(P.BRAND, -10, -40, 30, -80, 2, WHITE, 1);

const dt = 1 / 120, checkpoints = new Set([0, 12, 60, 150, 300]);
for (let step = 0; step <= 300; step++) {
  if (step > 0) ps.update(dt);
  if (!checkpoints.has(step)) continue;
  const f = createFrame(160, 120, 0, 0);
  const p = new Painter(f).place(80, 100, false, 0);
  ps.draw(p, "back");
  ps.draw(p, "front");
  console.log(`(${step}, ${ps.count}, 0x${hash(f).toString(16).padStart(16, "0")}),`);
}
