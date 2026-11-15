#!/usr/bin/env node
// Reference render of a pixelfx effect, for the parity script (crates/pixfx-export/examples/parity.rs): the raw colour and light buffers of every frame.
//
//   node tools/pixelfx-ref.mjs <id> --out <dir> [--pixelfx DIR] [--opts JSON]
//
// Frames come from pixelfx's own renderFrames with no backdrop and no stand-in actor, at the effect's default
// fps and seed: each frame's `raw` and `light` are the colour (ABGR) and light (xBGR) Uint32 words of its bare
// render, edge fade applied. --opts {"seed": n, "options": {...}} sets the seed and the effect's options. Writes <out>/<id>.color.bin and <out>/<id>.light.bin (every frame's words,
// little-endian, frame after frame) and <out>/<id>.json {w, h, n, fps, opts}. The pixelfx dir is --pixelfx, else
// $PIXELFX, else ./pixelfx.

import { mkdirSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { pathToFileURL } from "node:url";

const args = process.argv.slice(2), flags = {}, pos = [];
for (let i = 0; i < args.length; i++) args[i].startsWith("--") ? (flags[args[i].slice(2)] = args[++i]) : pos.push(args[i]);
const id = pos[0];
if (!id || !flags.out) {
  console.error("usage: pixelfx-ref.mjs <id> --out <dir> [--pixelfx DIR] [--opts JSON]");
  process.exit(2);
}
const dir = resolve(flags.pixelfx ?? process.env.PIXELFX ?? "pixelfx");
const lib = await import(pathToFileURL(join(dir, "tools", "lib", "fxlib.mjs")).href);

const opts = flags.opts ? JSON.parse(flags.opts) : {};
const r = lib.renderFrames(await lib.loadEffect(id), { ...opts, bg: "none", actor: false });
if (!r.frames[0]?.raw || !r.frames[0]?.light) {
  console.error("pixelfx-ref: this pixelfx renderFrames gives no raw colour or light per frame");
  process.exit(1);
}

// pixelfx's typed arrays are in host byte order; write little-endian whatever the host
const words = (key) => {
  const out = Buffer.alloc(r.n * r.w * r.h * 4);
  r.frames.forEach((f, k) => f[key].forEach((v, i) => out.writeUInt32LE(v >>> 0, (k * r.w * r.h + i) * 4)));
  return out;
};
const base = join(resolve(flags.out), id);
mkdirSync(dirname(base), { recursive: true });
writeFileSync(`${base}.color.bin`, words("raw"));
writeFileSync(`${base}.light.bin`, words("light"));
writeFileSync(`${base}.json`, JSON.stringify({ w: r.w, h: r.h, n: r.n, fps: r.fps, opts }) + "\n");
console.log(`${id}: ${r.n} frames ${r.w}x${r.h} at ${r.fps} fps to ${dirname(base)}`);
