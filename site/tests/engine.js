// The engine's wasm package for the tests: pixfx-web's, which the site's pixfx-canvas2d bundle carries
// (`site/build.sh` builds it; `cd packages/pixfx-web && bun run build:wasm` alone is enough here).
import { readFileSync } from "node:fs";
import { useEngine } from "../assets/duel.js";

const dir = new URL("../../packages/pixfx-web/wasm/", import.meta.url);
const pkg = await import(new URL("pixfx_wasm.js", dir).href);
pkg.initSync({ module: readFileSync(new URL("pixfx_wasm_bg.wasm", dir)) });

useEngine(pkg);
export { pkg };
export const { Body, Fx, Mind, effects } = pkg;
