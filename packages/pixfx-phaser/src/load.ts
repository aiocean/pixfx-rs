import { z } from "zod";
import type { Backend } from "./backend";
import type { Engine } from "./engine";
import { wasmEngine } from "./engine";
import { DEFAULT_FRAME_BUDGET, DEFAULT_TEXTURE_BUDGET, FrameCache } from "./frames";
import { InlineBackend } from "./inline";
import { frameSchema } from "./protocol";
import { WorkerPool, openWorkers, workerPort } from "./pool";

const loadOptionsSchema = z.object({
  /** Worker threads that draw effects; 0 draws on the main thread. Default: one per spare core, 1 to 4. */
  workers: z.number().int().nonnegative().optional(),
  /** Bytes the frames identical effects share may hold before the least recently shown go. Default 64 MiB. */
  frameBudget: z.number().int().nonnegative().default(DEFAULT_FRAME_BUDGET),
  /** Bytes the textures of frames off screen may hold before the least recently shown are reused. Default 32 MiB. */
  textureBudget: z.number().int().nonnegative().default(DEFAULT_TEXTURE_BUDGET),
  /** Perf harness only: each worker batch takes this many times its real time (phone emulation). */
  slowdown: z.number().min(1).default(1),
});

export type LoadOptions = z.input<typeof loadOptionsSchema>;

/** One worker per core the main thread leaves free, at least 1 and at most 4. */
export function defaultWorkers(hardwareConcurrency: number | undefined): number {
  return Math.min(4, Math.max(1, (hardwareConcurrency ?? 2) - 1));
}

/** The engine, where effects run, and the wasm reader of baked clips. */
interface Loaded {
  engine: Engine;
  backend: Backend;
  bakedFrames: (json: string, pages: Uint8Array[]) => unknown;
}

let loading: Promise<Loaded> | null = null;
let loaded: Loaded | null = null;

/**
 * Load the engine once and start the effect workers; `play()` works after this resolves. Later calls return the
 * same engine and ignore their options. Without `Worker` (or with `workers: 0`) effects draw on the main thread.
 */
export async function loadPixfx(options?: LoadOptions): Promise<Engine> {
  loading ??= start(loadOptionsSchema.parse(options ?? {}));
  loaded = await loading;
  return loaded.engine;
}

async function start(o: z.output<typeof loadOptionsSchema>): Promise<Loaded> {
  const wasm = await import("../wasm/pixfx_wasm.js");
  const { memory } = await wasm.default();
  const engine = wasmEngine(wasm, memory);
  const cache = new FrameCache(o.frameBudget, o.textureBudget);
  const n = typeof Worker === "undefined" ? 0 : (o.workers ?? defaultWorkers(globalThis.navigator?.hardwareConcurrency));
  const bakedFrames = (json: string, pages: Uint8Array[]): unknown => wasm.bakedFrames(json, pages);
  if (n === 0) return { engine, backend: new InlineBackend(engine, cache), bakedFrames };
  const workers = await openWorkers(n, o.slowdown);
  return { engine, backend: new WorkerPool(engine, workers.map(workerPort), cache), bakedFrames };
}

/** The loaded engine, or null before `loadPixfx()` resolves. */
export function loadedEngine(): Engine | null {
  return loaded?.engine ?? null;
}

const heldFramesSchema = z.array(z.object({ id: z.string(), frame: frameSchema }));

/**
 * Hold the frames of a clip baked by `pixfx bake` (`<name>.bake.json` and the PNG pages it names, fetched from
 * `url`'s folder): effects playing that effect with the same options then show the baked frames instead of drawing
 * them. Baked frames are held like drawn ones: past the frame budget they go, and are drawn again if shown.
 * Returns how many frames were loaded. Call after `loadPixfx()` resolves.
 */
export async function loadBaked(url: string): Promise<number> {
  if (!loaded) throw new Error("pixfx: call await loadPixfx() before loadBaked()");
  const { backend, bakedFrames } = loaded;
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
  const frames = heldFramesSchema.parse(bakedFrames(json, bytes));
  backend.hold(frames);
  return frames.length;
}

/** The part of a bake index the loader reads itself: the page files (the engine reads the rest). */
const bakeIndexSchema = z.object({ pages: z.array(z.object({ file: z.string() })) });

export function getBackend(): Backend {
  if (!loaded) throw new Error("pixfx: call await loadPixfx() before play()");
  return loaded.backend;
}

/**
 * Once per game step: drop shared frames over the budget and send the work every scene queued. Does nothing
 * before `loadPixfx()` resolves.
 */
export function flushPixfx(): void {
  loaded?.backend.flush();
}
