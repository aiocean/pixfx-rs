#!/usr/bin/env node
// An effect's game data as pixelfx has it, for the Rust goldens: its cues, its act timing and its state.
//
//   node tools/pixelfx-data.mjs <id> --pixelfx DIR [--opts JSON]
//
// Prints one JSON object {cues, act, state} on stdout:
//   cues   cuesOf(mod, options), [] when DIR has no src/core/cues.js or the effect has no cues; a sound has
//          its defaults filled (gain 1, pitch 1, len null)
//   act    actTiming(mod) without its `poses`, plus `dust`; null when DIR has no src/core/act.js or the effect has
//          no act. Each hit is {t, freeze, shake, amp, push, skin?, at?}; dust is [{t, x, n, dir}] with
//          pixelfx's defaults filled (n 6, dir null = random)
//   state  mod.meta.state, null for an effect that is no creature state
// --opts is the options JSON given to the effect (`{"from": -1, "hits": 2}`), the same words as
// the parity script's `--opt` (crates/pixfx-export/examples/parity.rs). The pixelfx dir is --pixelfx, else $PIXELFX, else ./pixelfx.

import { existsSync } from "node:fs";
import { join, resolve } from "node:path";
import { pathToFileURL } from "node:url";

const args = process.argv.slice(2), flags = {}, pos = [];
for (let i = 0; i < args.length; i++) args[i].startsWith("--") ? (flags[args[i].slice(2)] = args[++i]) : pos.push(args[i]);
const id = pos[0];
if (!id) {
  console.error("usage: pixelfx-data.mjs <id> --pixelfx DIR [--opts JSON]");
  process.exit(2);
}
const dir = resolve(flags.pixelfx ?? process.env.PIXELFX ?? "pixelfx");
const options = flags.opts ? JSON.parse(flags.opts) : {};
const load = async (rel) => (existsSync(join(dir, rel)) ? import(pathToFileURL(join(dir, rel)).href) : null);

const lib = await import(pathToFileURL(join(dir, "tools", "lib", "fxlib.mjs")).href);
const { mod } = await lib.loadEffect(id);
const cues = await load("src/core/cues.js");
const act = await load("src/core/act.js");

const filled = (c) => (c.type === "sound" ? { ...c, gain: c.gain ?? 1, pitch: c.pitch ?? 1, len: c.len ?? null } : c);
const { poses, ...timing } = act?.actTiming(mod) ?? { poses: null };
const actOut = act?.actTiming(mod) ? { ...timing, dust: (mod.act.dust ?? []).map((d) => ({ t: d.t, x: d.x, n: d.n ?? 6, dir: d.dir ?? null })) } : null;
console.log(JSON.stringify({ cues: (cues?.cuesOf(mod, options) ?? []).map(filled), act: actOut, state: mod.meta.state ?? null }, null, 2));
