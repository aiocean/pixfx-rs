// Prints the FNV-1a 64 frame hash (colour words then light words, 4 little-endian bytes each) of a scene drawn
// through pixelfx weather `clip` (with its `glow`) and `pass`, at whole, fractional, mirrored and rotated
// placements: the goldens in pixfx-effects tests/weather_clip.rs.
// Usage: node weather_clip.mjs <path to a pixelfx checkout>   (goldens taken at pixelfx 044e12a)
import { pathToFileURL } from "node:url";
import { join, resolve } from "node:path";

const root = process.argv[2];
if (!root) throw new Error("usage: node weather_clip.mjs <pixelfx dir>");
const load = (f) => import(pathToFileURL(join(resolve(root), f)).href);
const { createFrame } = await load("src/core/fx.js");
const { Painter } = await load("src/core/painter.js");
const { hex, WHITE } = await load("src/core/color.js");
const { clip, pass } = await load("src/fx/weather/_lib.js");

const hash = (f) => {
  let h = 0xcbf29ce484222325n;
  for (const buf of [f.color, f.light])
    for (const w of buf)
      for (let s = 0; s < 32; s += 8) h = ((h ^ BigInt((w >>> s) & 255)) * 0x100000001b3n) & 0xffffffffffffffffn;
  return h;
};

// a 40 x 24 canvas with its origin at (20, 18), in a 64 x 48 frame
const meta = { size: [40, 24], origin: [20, 18] };
const A = hex("#93abd6"), B = hex("#24304f"), C = hex("#ffe26b");
const scene = (p) => {
  const q = clip(p, meta);
  pass(p, meta, -18, 5, (x, y, out) => {
    const fx = Math.floor(x), fy = Math.floor(y);
    out.c = (fx + fy) & 1 ? A : B;
    out.a = 0.8;
    out.b = (fx & 3) === 0;
    if ((fy & 3) === 0) ((out.l = C), (out.k = 0.15));
  });
  q.glow(-20, -18, 12, C, 0.8);
  q.glow(19.5, 5.5, 12, A, 0.8);
  q.glow(0, -6, 9, C, 0.6);
  for (let x = -22; x <= 21; x++) for (const y of [-19, -18, 5, 6]) q.px(x, y, WHITE);
  for (let y = -19; y <= 6; y++) for (const x of [-21, -20, 19, 20]) q.px(x, y, WHITE);
};
const run = (name, at, flip, angle) => {
  const f = createFrame(64, 48, 0, 0);
  scene(new Painter(f).place(at[0], at[1], flip, angle));
  console.log(`${name} = 0x${hash(f).toString(16).padStart(16, "0")}`);
};

run("whole", [32, 24], false, 0);
run("half", [32.5, 24.5], false, 0);
run("third", [32.3, 24.3], false, 0);
run("half_mirrored", [32.5, 24.5], true, 0);
run("rotated", [32, 24], false, 0.5);
