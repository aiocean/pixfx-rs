// The engine's wasm package for the tests, as site/build.sh builds it (into target/site-pkg): run the build first.
import { readFileSync } from "node:fs";

const dir = new URL("../../target/site-pkg/", import.meta.url);
const pkg = await import(new URL("pixfx_canvas.js", dir).href);
await pkg.default({ module_or_path: readFileSync(new URL("pixfx_canvas_bg.wasm", dir)) });

export const { Body, Fx, effects } = pkg;
