// Prints the FNV-1a 64 frame hash (colour words then light words, 4 little-endian bytes each) of pixelfx
// transitions/burn-away played by createFx at placements off the whole-pixel grid and rotated, on a frame with
// room round the screen: the goldens in tests/transitions.rs `burn_away_placed_matches_pixelfx`.
// Usage: node burn_away_placed.mjs <path to a pixelfx checkout>   (goldens taken at pixelfx 044e12a)
import { pathToFileURL } from "node:url";
import { join, resolve } from "node:path";

const root = process.argv[2];
if (!root) throw new Error("usage: node burn_away_placed.mjs <pixelfx dir>");
const load = (f) => import(pathToFileURL(join(resolve(root), "src", f)).href);
const { createFx, createFrame } = await load("core/fx.js");
const fx = await load("fx/transitions/burn-away.js");

const hash = (f) => {
  let h = 0xcbf29ce484222325n;
  for (const buf of [f.color, f.light])
    for (const w of buf)
      for (let s = 0; s < 32; s += 8) h = ((h ^ BigInt((w >>> s) & 255)) * 0x100000001b3n) & 0xffffffffffffffffn;
  return h;
};
// frame i is drawn after i steps of 1/30 s, as tools renders do
const run = (name, x, y, angle, frames) => {
  const inst = createFx(fx, { x, y, angle });
  let i = 0;
  for (const n of frames) {
    for (; i < n; i++) inst.update(1 / 30);
    const f = createFrame(640, 500, -160, -160);
    inst.draw(f);
    console.log(`${name}_${n} = 0x${hash(f).toString(16).padStart(16, "0")}`);
  }
};

// burning in (30), fully covered (42), burning open (60)
run("half_pixel", 0.5, 0.5, 0, [30, 42, 60]);
run("rotated", 160, 90, 0.3, [30, 42, 60]);
