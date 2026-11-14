// The real engine for tests: pixfx-web's wasm module, loaded once, synchronously.
import { readFileSync } from "node:fs";
import * as wasm from "pixfx-web/wasm";

const { memory } = wasm.initSync({ module: readFileSync(new URL("pixfx_wasm_bg.wasm", import.meta.resolve("pixfx-web/wasm"))) });

export { memory, wasm };
