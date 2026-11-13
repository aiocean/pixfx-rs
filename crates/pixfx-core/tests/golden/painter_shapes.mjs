// Prints the FNV-1a 64 frame hash (colour words then light words, 4 little-endian bytes each) of each shape
// case, the goldens in tests/painter.rs `shapes_match_pixelfx`.
// Usage: node painter_shapes.mjs <path to a pixelfx checkout>   (goldens taken at pixelfx 044e12a)
import { pathToFileURL } from "node:url";
import { join, resolve } from "node:path";

const root = process.argv[2];
if (!root) throw new Error("usage: node painter_shapes.mjs <pixelfx dir>");
const load = (f) => import(pathToFileURL(join(resolve(root), "src/core", f)).href);
const { createFrame } = await load("fx.js");
const { Painter } = await load("painter.js");
const { hex, WHITE } = await load("color.js");
// the transitions' clipped painter (pixelfx Clipped: put, add and blendPx keep only cells in the 320 x 180 screen)
const { clipTo } = await import(pathToFileURL(join(resolve(root), "src/fx/transitions/_lib.js")).href);

const hash = (f) => {
  let h = 0xcbf29ce484222325n;
  for (const buf of [f.color, f.light])
    for (const w of buf)
      for (let s = 0; s < 32; s += 8) h = ((h ^ BigInt((w >>> s) & 255)) * 0x100000001b3n) & 0xffffffffffffffffn;
  return h;
};
const run = (name, draw, flip = false, angle = 0, clip = false, at = [32, 40]) => {
  const f = createFrame(64, 48, 0, 0);
  const p = new Painter(f).place(at[0], at[1], flip, angle);
  draw(clip ? clipTo(p) : p);
  console.log(`${name} = 0x${hash(f).toString(16).padStart(16, "0")}`);
};

// colours go through hex(): the painter's put/blendPx take packed words, not strings
run("disc_soft", (p) => p.disc(0, -16, 9, hex("#ff8a2a"), 0.6));
run("disc_tiny", (p) => p.disc(0, -16, 0.4, hex("#ff8a2a"), 1));
run("circle_fn", (p) => p.circle(0, -16, 9, (d) => (d < 0.5 ? WHITE : hex("#3f8cff")), 1));
run("circle_tiny", (p) => p.circle(0, -16, 0.5, hex("#3f8cff"), 1));
run("ring_fn", (p) => p.ring(0, -16, 12, 3, hex("#9a48ec"), (v) => v));
run("light", (p) => p.light(3, -5, hex("#ffe26b"), 0.7));
run("disc_mirrored_rotated", (p) => p.disc(0, -16, 9, hex("#ff8a2a"), 0.6), true, 0.6);
// through clipTo: a glow and a disc straddling the screen's top-left corner, mirrored and rotated; then its
// bottom-right corner (320, 180) brought onto the frame
const straddle = (p) => (p.glow(2, 3, 14, hex("#ffe26b"), 0.8), p.disc(1, 4, 9, hex("#ff8a2a"), 0.6), p.circle(-2, 6, 4, WHITE, 1));
run("clip_origin_mirrored_rotated", straddle, true, 0.6, true, [32, 24]);
run("clip_far_corner", (p) => (p.glow(318, 177, 14, hex("#ffe26b"), 0.8), p.disc(319, 176, 9, hex("#ff8a2a"), 0.6)), false, 0, true, [-288, -150]);
