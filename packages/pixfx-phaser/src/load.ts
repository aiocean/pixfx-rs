import { z } from "zod";
import type { Backend } from "./backend";
import type { Engine } from "./engine";
import { wasmEngine } from "./engine";
import { InlineBackend } from "./inline";
import { WorkerPool, openWorkers, workerPort } from "./pool";

const loadOptionsSchema = z.object({
  /** Worker threads that draw effects; 0 draws on the main thread. Default: one per spare core, 1 to 4. */
  workers: z.number().int().nonnegative().optional(),
  /** Perf harness only: each worker batch takes this many times its real time (phone emulation). */
  slowdown: z.number().min(1).default(1),
});

export type LoadOptions = z.input<typeof loadOptionsSchema>;

/** One worker per core the main thread leaves free, at least 1 and at most 4. */
export function defaultWorkers(hardwareConcurrency: number | undefined): number {
  return Math.min(4, Math.max(1, (hardwareConcurrency ?? 2) - 1));
}

let loading: Promise<{ engine: Engine; backend: Backend }> | null = null;
let loaded: { engine: Engine; backend: Backend } | null = null;

/**
 * Load the engine once and start the effect workers; `play()` works after this resolves. Later calls return the
 * same engine and ignore their options. Without `Worker` (or with `workers: 0`) effects draw on the main thread.
 */
export async function loadPixfx(options?: LoadOptions): Promise<Engine> {
  loading ??= start(loadOptionsSchema.parse(options ?? {}));
  loaded = await loading;
  return loaded.engine;
}

async function start(o: z.output<typeof loadOptionsSchema>): Promise<{ engine: Engine; backend: Backend }> {
  const wasm = await import("../wasm/pixfx_wasm.js");
  const { memory } = await wasm.default();
  const engine = wasmEngine(wasm, memory);
  const n = typeof Worker === "undefined" ? 0 : (o.workers ?? defaultWorkers(globalThis.navigator?.hardwareConcurrency));
  if (n === 0) return { engine, backend: new InlineBackend(engine) };
  const workers = await openWorkers(n, o.slowdown);
  return { engine, backend: new WorkerPool(engine, workers.map(workerPort)) };
}

export function getBackend(): Backend {
  if (!loaded) throw new Error("pixfx: call await loadPixfx() before play()");
  return loaded.backend;
}

/** Send the work every scene queued this frame. Does nothing before `loadPixfx()` resolves. */
export function flushPixfx(): void {
  loaded?.backend.flush();
}
