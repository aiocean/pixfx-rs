// The two wasm modules the site's pages load, for the tests: the engine's (pixfx-web's, which the site's
// pixfx-canvas2d bundle carries; `cd packages/pixfx-web && bun run build:wasm`) and the arena's rules, which
// `site/build.sh` embeds into site/arena (the arena's `site/embed.sh`).
import { readFileSync } from "node:fs";
import { useRules } from "../arena/duel.js";

const dir = new URL("../../packages/pixfx-web/wasm/", import.meta.url);
const pkg = await import(new URL("pixfx_wasm.js", dir).href);
pkg.initSync({ module: readFileSync(new URL("pixfx_wasm_bg.wasm", dir)) });

const rulesDir = new URL("../arena/wasm/", import.meta.url);
const rules = await import(new URL("arena_wasm.js", rulesDir).href);
rules.initSync({ module: readFileSync(new URL("arena_wasm_bg.wasm", rulesDir)) });
useRules(rules);

export { pkg };
export const { Body, Fx, Mind, effects } = pkg;
