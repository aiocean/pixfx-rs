import { existsSync, readFileSync } from "node:fs";
import { expect, test } from "bun:test";
import { z } from "zod";
import { engineOptions, parsePlayOptions } from "../src/options";

const WASM_BIN = new URL("../wasm/pixfx_wasm_bg.wasm", import.meta.url);
const built = existsSync(WASM_BIN);

// The fake engine accepts any options; only the real engine proves the option types match it.
test.skipIf(!built)("the real wasm Fx accepts every documented play option (run `bun run build:wasm` if skipped)", async () => {
  const wasm = await import("../wasm/pixfx_wasm.js");
  wasm.initSync({ module: readFileSync(WASM_BIN) });
  const id = z.array(z.string()).parse(wasm.effects())[0];
  if (id === undefined) throw new Error("wasm lists no effects");
  const margin = z.object({ glowMargin: z.number() }).parse(wasm.meta(id)).glowMargin;
  const opts = parsePlayOptions({ seed: 1, flip: true, angle: 0.5, light: 0, palette: ["#000000", "#ffffff"] });
  const fx = new wasm.Fx(id, engineOptions(opts, margin));
  expect(fx.width()).toBeGreaterThan(0);
  fx.free();
});
