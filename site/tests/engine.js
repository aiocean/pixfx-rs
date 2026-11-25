// The engine's wasm module as the site's pages load it, for the tests: pixfx-web's, which the site's
// pixfx-canvas2d bundle carries (`cd packages/pixfx-web && bun run build:wasm`).
import { readFileSync } from "node:fs";

const dir = new URL("../../packages/pixfx-web/wasm/", import.meta.url);
const pkg = await import(new URL("pixfx_wasm.js", dir).href);
pkg.initSync({ module: readFileSync(new URL("pixfx_wasm_bg.wasm", dir)) });

export { pkg };
export const { Body, Fx, Mind, effects } = pkg;
