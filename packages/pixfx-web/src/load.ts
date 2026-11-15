import { z } from "zod";
import type { Engine } from "./engine";
import { wasmEngine } from "./engine";
import type { WorkerPort } from "./game";
import { Game } from "./game";
import type { ToWorker } from "./protocol";
import { fromWorkerSchema } from "./protocol";

const loadOptionsSchema = z.strictObject({
  /** Worker threads that paint effects; 0 paints on the main thread. Default: one per spare core, 1 to 4. */
  workers: z.number().int().nonnegative().optional(),
  /** Bytes the frames identical effects share may hold before the least recently shown go. Default 64 MiB. */
  frameBudget: z.number().int().nonnegative().optional(),
  /** Perf harness only: each worker message takes this many times its real time (phone emulation). */
  slowdown: z.number().min(1).default(1),
});

export type LoadOptions = z.input<typeof loadOptionsSchema>;

/** One worker per core the main thread leaves free, at least 1 and at most 4. */
export function defaultWorkers(hardwareConcurrency: number | undefined): number {
  return Math.min(4, Math.max(1, (hardwareConcurrency ?? 2) - 1));
}

interface Loaded {
  engine: Engine;
  game: Game;
}

let loading: Promise<Loaded> | null = null;
let loaded: Loaded | null = null;

/**
 * Load the engine once and start the painting workers; `play()` works after this resolves. Later calls return the
 * same engine and ignore their options. Without `Worker` (or with `workers: 0`) effects paint on the main thread.
 */
export async function loadPixfx(options?: LoadOptions): Promise<Engine> {
  loading ??= start(loadOptionsSchema.parse(options ?? {}));
  loaded = await loading;
  return loaded.engine;
}

async function start(o: z.output<typeof loadOptionsSchema>): Promise<Loaded> {
  const wasm = await import("../wasm/pixfx_wasm.js");
  const { memory } = await wasm.default();
  const n = typeof Worker === "undefined" ? 0 : (o.workers ?? defaultWorkers(globalThis.navigator?.hardwareConcurrency));
  const ports = n === 0 ? [] : (await openWorkers(n, o.slowdown)).map(workerPort);
  const game = new Game(wasm, memory, ports);
  if (o.frameBudget !== undefined) game.frames.setBudget(o.frameBudget);
  return { engine: wasmEngine(wasm), game };
}

/** The loaded engine, or null before `loadPixfx()` resolves. */
export function loadedEngine(): Engine | null {
  return loaded?.engine ?? null;
}

/** The loaded game, or null before `loadPixfx()` resolves. */
export function loadedGame(): Game | null {
  return loaded?.game ?? null;
}

/**
 * Hold the frames of a clip the bake script (`crates/pixfx-export/examples/bake.rs`) wrote: `<name>.bake.json` and
 * the PNG pages it names, fetched from `url`'s folder. Effects playing that effect with the same options then show
 * the baked frames instead of painting them. Baked frames are held like painted ones: past the frame budget they go, and are painted again if shown.
 * Returns how many frames were loaded. Call after `loadPixfx()` resolves and a scene of the shell has started.
 */
export async function loadBaked(url: string): Promise<number> {
  if (!loaded) throw new Error("pixfx: call await loadPixfx() before loadBaked()");
  const { game } = loaded;
  const fetchOk = async (u: URL): Promise<Response> => {
    const r = await fetch(u);
    if (!r.ok) throw new Error(`pixfx: fetching ${u.href} failed: ${r.status}`);
    return r;
  };
  const jsonUrl = new URL(url, globalThis.location?.href);
  const json = await (await fetchOk(jsonUrl)).text();
  const { pages } = bakeIndexSchema.parse(JSON.parse(json));
  const bytes = await Promise.all(
    pages.map(async (p) => new Uint8Array(await (await fetchOk(new URL(p.file, jsonUrl))).arrayBuffer())),
  );
  return game.loadBaked(json, bytes);
}

/** The part of a bake index the loader reads itself: the page files (the engine reads the rest). */
const bakeIndexSchema = z.object({ pages: z.array(z.object({ file: z.string() })) });

/** Wrap a module worker running `worker.ts`. */
export function workerPort(worker: Worker): WorkerPort {
  return {
    post: (message, transfer) => worker.postMessage(message, transfer),
    listen: (onMessage) => {
      worker.onmessage = (e: MessageEvent<unknown>) => onMessage(e.data);
      worker.onerror = (e) => {
        throw new Error(`pixfx worker: ${e.message}`);
      };
    },
  };
}

/** Start `n` painting workers and wait until each has loaded the engine. Rejects if any fails to start. */
export async function openWorkers(n: number, slowdown: number): Promise<Worker[]> {
  const workers = Array.from({ length: n }, () => new Worker(new URL("./worker.ts", import.meta.url), { type: "module" }));
  try {
    await Promise.all(workers.map((w) => ready(w, slowdown)));
  } catch (err) {
    for (const w of workers) w.terminate();
    throw err;
  }
  return workers;
}

function ready(worker: Worker, slowdown: number): Promise<void> {
  return new Promise((resolve, reject) => {
    worker.onmessage = (e: MessageEvent<unknown>) => {
      const msg = fromWorkerSchema.safeParse(e.data);
      if (msg.success && msg.data.type === "ready") resolve();
      else reject(new Error(`pixfx worker failed to start: ${msg.success && msg.data.type === "error" ? msg.data.message : "unexpected reply"}`));
    };
    worker.onerror = (e) => reject(new Error(`pixfx worker failed to start: ${e.message}`));
    const init: ToWorker = { type: "init", slowdown };
    worker.postMessage(init);
  });
}
