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

function realEngine() {
  return import("../wasm/pixfx_wasm.js").then(async (wasm) => {
    const { wasmEngine } = await import("../src/engine");
    const { memory } = wasm.initSync({ module: readFileSync(WASM_BIN) });
    return { wasm, engine: wasmEngine(wasm, memory) };
  });
}

// The engine adapter hands out views on wasm memory instead of copies; they must hold exactly the frame's words.
test.skipIf(!built)("the engine's RGBA views match the frame words (run `bun run build:wasm` if skipped)", async () => {
  const { wasm, engine } = await realEngine();
  const { prepare } = await import("../src/engine");
  const p = prepare(engine, "fire/campfire", { seed: 3 });
  const fx = engine.create(p.id, p.engineOpts);
  const raw = new wasm.Fx(p.id, p.engineOpts);
  for (let i = 0; i < 3; i++) {
    fx.update(0.1);
    raw.update(0.1);
    fx.draw("front");
    raw.draw("front");
    const rgba = Array.from(fx.colorRgba());
    const lit = Array.from(fx.lightRgba());
    expect(rgba.length).toBe(fx.width() * fx.height() * 4);
    const want = Array.from(raw.color()).flatMap((w) => [w & 255, (w >>> 8) & 255, (w >>> 16) & 255, w >>> 24]);
    const wantLit = Array.from(raw.light()).flatMap((w) => ((w & 0xffffff) === 0 ? [0, 0, 0, 0] : [w & 255, (w >>> 8) & 255, (w >>> 16) & 255, 255]));
    expect(rgba).toEqual(want);
    expect(lit).toEqual(wantLit);
    expect(want.some((v) => v !== 0) && wantLit.some((v) => v !== 0)).toBe(true);
  }
  fx.free();
  raw.free();
});

// A real module worker running worker.ts with its own wasm: frames equal the main thread's, byte for byte.
test.skipIf(!built)("a real worker draws the same pixels as the main thread (run `bun run build:wasm` if skipped)", async () => {
  const { engine } = await realEngine();
  const { InlineBackend } = await import("../src/inline");
  const { WorkerPool, openWorkers, workerPort } = await import("../src/pool");
  const workers = await openWorkers(2, 1);
  try {
    const pool = new WorkerPool(engine, workers.map(workerPort));
    const inline = new InlineBackend(engine);
    const steps = [0.25, 0.25, 0.25, 0.25, 0.25]; // each crosses a frame boundary of the effect's fps
    const collect = async (backend: typeof inline | typeof pool, wait: boolean): Promise<number[][][]> => {
      const frames: number[][][] = [];
      let arrived: (() => void) | null = null;
      const { instance } = backend.spawn("fire/campfire", { seed: 3 });
      const next = async (send: () => void): Promise<void> => {
        const got = new Promise<void>((resolve) => (arrived = resolve));
        send();
        backend.flush();
        if (wait) await got;
      };
      await next(() =>
        instance.start({
          frame: (f) => {
            frames.push(f.map((b) => Array.from(new Uint8Array(b))));
            instance.recycle(f);
            arrived?.();
          },
          done: () => arrived?.(),
        }),
      );
      for (const dt of steps) await next(() => instance.advance(dt));
      instance.free();
      return frames;
    };
    const want = await collect(inline, false);
    const got = await collect(pool, true);
    expect(want.length).toBe(steps.length + 1);
    expect(got.length).toBe(want.length);
    expect(got).toEqual(want);
  } finally {
    for (const w of workers) w.terminate();
  }
});
