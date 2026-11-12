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

// loadPixfx hands out views on wasm memory instead of copies; they must hold exactly the frame's words.
test.skipIf(!built)("the loaded engine's RGBA views match the frame words (run `bun run build:wasm` if skipped)", async () => {
  const { loadPixfx, spawnFx } = await import("../src/engine");
  const engine = await loadPixfx();
  const { fx } = spawnFx(engine, "fire/campfire", { seed: 3 });
  for (let i = 0; i < 3; i++) {
    fx.update(0.1);
    fx.draw("front");
    const words = fx.color();
    const light = fx.light();
    const rgba = Array.from(fx.colorRgba());
    const lit = Array.from(fx.lightRgba());
    expect(rgba.length).toBe(fx.width() * fx.height() * 4);
    const want = Array.from(words).flatMap((w) => [w & 255, (w >>> 8) & 255, (w >>> 16) & 255, w >>> 24]);
    const wantLit = Array.from(light).flatMap((w) => ((w & 0xffffff) === 0 ? [0, 0, 0, 0] : [w & 255, (w >>> 8) & 255, (w >>> 16) & 255, 255]));
    expect(rgba).toEqual(want);
    expect(lit).toEqual(wantLit);
    expect(want.some((v) => v !== 0) && wantLit.some((v) => v !== 0)).toBe(true);
  }
  fx.free();
});
