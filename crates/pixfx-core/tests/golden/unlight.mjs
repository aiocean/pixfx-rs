// Prints the FNV-1a 64 frame hash (colour words then light words, 4 little-endian bytes each) of pixelfx shadow
// `unlight` over a lit frame, the goldens in tests/painter.rs `dim_light_matches_pixelfx_unlight`.
// Usage: node unlight.mjs <path to a pixelfx checkout>   (goldens taken at pixelfx 044e12a)
import { pathToFileURL } from "node:url";
import { join, resolve } from "node:path";

const root = process.argv[2];
if (!root) throw new Error("usage: node unlight.mjs <pixelfx dir>");
const load = (f) => import(pathToFileURL(join(resolve(root), f)).href);
const { createFrame } = await load("src/core/fx.js");
const { Painter } = await load("src/core/painter.js");
const { hex } = await load("src/core/color.js");
const { unlight } = await load("src/fx/shadow/_lib.js");

const hash = (f) => {
  let h = 0xcbf29ce484222325n;
  for (const buf of [f.color, f.light])
    for (const w of buf)
      for (let s = 0; s < 32; s += 8) h = ((h ^ BigInt((w >>> s) & 255)) * 0x100000001b3n) & 0xffffffffffffffffn;
  return h;
};
const run = (name, keep, flip = false, angle = 0) => {
  const f = createFrame(64, 48, 0, 0);
  const p = new Painter(f).place(32, 40, flip, angle);
  p.glow(0, -16, 18, hex("#9d5cec"), 0.9);
  p.glow(6, -12, 9, hex("#ffe26b"), 0.6);
  unlight(p, -10, -26, 10, -6, (x, y) => x * x + (y + 16) ** 2 < 49, keep);
  console.log(`${name} = 0x${hash(f).toString(16).padStart(16, "0")}`);
};
run("unlight_all", 0);
run("unlight_keep_third", 0.35);
run("unlight_mirrored_rotated", 0, true, 0.6);
