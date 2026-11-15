#!/usr/bin/env node
// Reference render of an effect ported from Sprite Fusion's pixel art VFX (https://www.spritefusion.com/pixel-art-vfx),
// for the parity script (crates/pixfx-export/examples/parity.rs): the raw colour and light buffers of every frame, in the files tools/pixelfx-ref.mjs writes.
//
//   node tools/spritefusion-ref.mjs <id> --out <dir> --canvas JSON [--opts JSON] [--actor FILE]
//
// The source is tools/spritefusion/<id>.ts, vendored verbatim (node runs it by stripping its types). --canvas
// {"w", "h", "ox", "oy", "fps", "n", "seed"} is the port's canvas, origin, fps, frame count and default seed, from
// its Rust meta. Frames step as pixelfx renderFrames does: frame k is the effect after k ticks of 1 / fps, each
// split into pixelfx's 120 Hz sub-steps, drawn whole (back, then front) with its feet on the origin, light eased out
// at the edge. --opts {"seed": n, "options": {...}} sets the seed and the effect's own options; Sprite Fusion effects
// cannot mirror, so `flip` is refused. --actor FILE ({"every": n, "frames": [{x, y, w, h, data}]}, from the parity
// script's --actor) plays it on a character: its feet on the origin, its silhouette frame k / every (in turn) through frame
// k's steps and draw, each frame's top-left (x, y) in art pixels from the feet and data its ABGR words.

import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

const args = process.argv.slice(2), flags = {}, pos = [];
for (let i = 0; i < args.length; i++) args[i].startsWith("--") ? (flags[args[i].slice(2)] = args[++i]) : pos.push(args[i]);
const id = pos[0];
if (!id || !flags.out || !flags.canvas) {
  console.error("usage: spritefusion-ref.mjs <id> --out <dir> --canvas JSON [--opts JSON] [--actor FILE]");
  process.exit(2);
}
const c = JSON.parse(flags.canvas), opts = flags.opts ? JSON.parse(flags.opts) : {};
const own = { ...(opts.options ?? {}) };
if (own.flip) {
  console.error("spritefusion-ref: Sprite Fusion effects cannot mirror; drop flip");
  process.exit(2);
}
delete own.flip;

const src = join(dirname(fileURLToPath(import.meta.url)), "spritefusion", `${id}.ts`);
const create = (await import(pathToFileURL(src).href)).default;
// world units are 2 art pixels (the source's PX); feet at world (0, 0) put the effect's centre on art (0, 0)
const actors = flags.actor ? JSON.parse(readFileSync(flags.actor, "utf8")) : null;
const sprites = actors?.frames.map((f) => ({ ...f, data: Uint32Array.from(f.data) })) ?? [];
let sprite = null;
const actor = actors ? { feet: () => [0, 0], silhouette: () => sprite } : undefined;
const fx = create({ ...own, x: 0, y: 0, seed: opts.seed ?? c.seed, ...(actor ? { actor } : {}) });

const frame = { w: c.w, h: c.h, ox: -c.ox, oy: -c.oy, color: new Uint32Array(c.w * c.h), light: new Uint32Array(c.w * c.h) };
const color = Buffer.alloc(c.n * c.w * c.h * 4), light = Buffer.alloc(c.n * c.w * c.h * 4);
for (let k = 0; k < c.n; k++) {
  if (actors) sprite = sprites[Math.floor(k / actors.every) % sprites.length];
  if (k > 0) step(1 / c.fps);
  frame.color.fill(0);
  frame.light.fill(0);
  fx.draw(frame);
  edgeFade(frame);
  frame.color.forEach((v, i) => color.writeUInt32LE(v >>> 0, (k * c.w * c.h + i) * 4));
  frame.light.forEach((v, i) => light.writeUInt32LE(v >>> 0, (k * c.w * c.h + i) * 4));
}
const base = join(resolve(flags.out), id);
mkdirSync(dirname(base), { recursive: true });
writeFileSync(`${base}.color.bin`, color);
writeFileSync(`${base}.light.bin`, light);
writeFileSync(`${base}.json`, JSON.stringify({ w: c.w, h: c.h, n: c.n, fps: c.fps, opts }) + "\n");
console.log(`${id}: ${c.n} frames ${c.w}x${c.h} at ${c.fps} fps to ${dirname(base)}`);

// pixelfx src/core/fx.js update (commit 044e12a): simulations step at 120 Hz or finer whatever the frame rate
function step(dt) {
  const n = Math.max(1, Math.ceil(dt * 120 - 1e-9));
  for (let i = 0; i < n; i++) fx.update(dt / n);
}

// pixelfx tools/lib/fxlib.mjs edgeFade (commit 044e12a), verbatim: light eased out over the last E pixels
function edgeFade(f, E = 8) {
  const { w, h, light: L } = f;
  for (let y = 0; y < h; y++) {
    const ey = Math.min(y, h - 1 - y);
    for (let x = 0; x < w; x++) {
      const e = Math.min(ey, x, w - 1 - x);
      if (e >= E) { x = w - 1 - E; continue; }
      const i = y * w + x, l = L[i];
      if (!(l & 0xffffff)) continue;
      const u = (e + 0.5) / E, k = u * u * (3 - 2 * u);
      L[i] = (l & 0xff000000) | Math.round((l & 255) * k) | (Math.round(((l >>> 8) & 255) * k) << 8) | (Math.round(((l >>> 16) & 255) * k) << 16);
    }
  }
}
