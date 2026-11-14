// The real engine for tests: the wasm module `bun run build:wasm` made, loaded once, synchronously.
import { readFileSync } from "node:fs";
import * as wasm from "../wasm/pixfx_wasm.js";

const { memory } = wasm.initSync({ module: readFileSync(new URL("../wasm/pixfx_wasm_bg.wasm", import.meta.url)) });

export { memory, wasm };
