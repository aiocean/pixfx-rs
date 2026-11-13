// Prints pixelfx's ease curves, fbm/tile noise samples, loopNoise and PAL ramps as Rust literals, the goldens in
// tests/math.rs. Colours are packed words (R | G << 8 | B << 16 | A << 24).
// Usage: node math.mjs <path to a pixelfx checkout>   (goldens taken at pixelfx 044e12a)
import { pathToFileURL } from "node:url";
import { join, resolve } from "node:path";

const root = process.argv[2];
if (!root) throw new Error("usage: node math.mjs <pixelfx dir>");
const load = (f) => import(pathToFileURL(join(resolve(root), "src/core", f)).href);
const { ease, fbm, tileNoise, tileFbm, loopNoise } = await load("math.js");
const { PAL, RAMP } = await load("color.js");

const snake = (s) => s.replace(/[A-Z]/g, (c) => "_" + c.toLowerCase());
const f = (v) => (Number.isInteger(v) ? v.toFixed(1) : String(v));
const ts = [-0.1, 0, 0.3, 0.5, 0.9, 1, 1.2];
console.log(`const EASE_T: [f64; ${ts.length}] = [${ts.map(f).join(", ")}];`);
for (const [name, fn] of Object.entries(ease)) {
  console.log(`    ("${snake(name)}", [${ts.map((t) => f(fn(t))).join(", ")}]),`);
}
console.log(`fbm(1.3, 2.7, 5, 3) = ${f(fbm(1.3, 2.7, 5, 3))}`);
console.log(`tile_noise(3.3, 7.6, 4, 8, 2) = ${f(tileNoise(3.3, 7.6, 4, 8, 2))}`);
console.log(`tile_noise(3.3, 7.6, 0, 0, 2) = ${f(tileNoise(3.3, 7.6, 0, 0, 2))}`);
console.log(`tile_fbm(3.3, 7.6, 4, 8, 2, 3) = ${f(tileFbm(3.3, 7.6, 4, 8, 2, 3))}`);
for (const p of [0, 0.37, 1]) console.log(`loop_noise(${f(p)}, 1.5, 4) = ${f(loopNoise(p, 1.5, 4))}`);
for (const [name, ramp] of Object.entries(RAMP)) {
  console.log(`    ("${name}", &[${ramp.map((c) => "0x" + (c >>> 0).toString(16).padStart(8, "0")).join(", ")}]),`);
}
