import type { Backend, FrameBuffers, FxEvents, FxInstance, Shape } from "./backend";
import { shapeOf } from "./backend";
import type { Engine } from "./engine";
import { prepare } from "./engine";
import type { PlayOptions } from "./options";
import type { FromWorker, Item, Spawn, ToWorker } from "./protocol";
import { fromWorkerSchema } from "./protocol";

/** The pool's line to one worker. */
export interface WorkerPort {
  post(message: ToWorker, transfer: ArrayBuffer[]): void;
  listen(onMessage: (data: unknown) => void): void;
}

/** One worker and what is queued for its next batch. */
interface Lane {
  port: WorkerPort;
  busy: boolean;
  /** Pixel area of the effects living in it. */
  area: number;
  spawns: Spawn[];
  frees: number[];
  items: Map<number, Item>;
}

interface Live {
  lane: Lane;
  area: number;
  events: FxEvents;
  /** The spawn has left in a batch, so freeing it needs a message. */
  sent: boolean;
}

/** Main-thread handle on an effect living in a worker. Calls queue work for the pool's next batch. */
class PooledFx implements FxInstance {
  private handle = -1;

  constructor(
    private readonly pool: WorkerPool,
    readonly shape: Shape,
    private readonly spawn: Omit<Spawn, "handle">,
  ) {}

  start(events: FxEvents): void {
    if (this.handle < 0) this.handle = this.pool.attach(this.spawn, this.shape.width * this.shape.height, events);
  }
  advance(dt: number): void {
    if (dt > 0) this.pool.queue(this.handle, (i) => (i.dt += dt));
  }
  stop(): void {
    this.pool.queue(this.handle, (i) => (i.stop = true));
  }
  free(): void {
    this.pool.free(this.handle);
  }
  recycle(frame: FrameBuffers): void {
    this.pool.queue(this.handle, (i) => i.give.push(frame));
  }
}

/**
 * Effects drawn on worker threads. Each effect lives in one worker for its whole life (the one holding the
 * least pixel area when it was played). Work is queued per effect and sent once per frame (`flush`) as one
 * batch per worker that has no batch in flight; a busy worker's effects keep accumulating scene time, so the
 * main thread never waits. Frames come back in the reply, one frame later than they were asked for.
 */
export class WorkerPool implements Backend {
  private readonly lanes: Lane[];
  private readonly live = new Map<number, Live>();
  private nextHandle = 0;

  /** @param engine the main thread's engine, used to validate plays and learn effect sizes. */
  constructor(
    private readonly engine: Engine,
    ports: readonly WorkerPort[],
  ) {
    if (ports.length === 0) throw new Error("pixfx: a worker pool needs at least one worker");
    this.lanes = ports.map((port) => ({ port, busy: false, area: 0, spawns: [], frees: [], items: new Map() }));
    for (const lane of this.lanes) lane.port.listen((data) => this.receive(lane, data));
  }

  spawn(id: string, opts: unknown): { instance: FxInstance; play: PlayOptions } {
    const p = prepare(this.engine, id, opts);
    // Built and freed here so invalid options throw now, and to learn the canvas size and origin.
    const fx = this.engine.create(p.id, p.engineOpts);
    const shape = shapeOf(fx);
    fx.free();
    return { instance: new PooledFx(this, shape, { id: p.id, opts: p.engineOpts, fps: p.fps }), play: p.play };
  }

  /** Send one batch to every idle worker that has work. Gives alone wait for the next batch with work. */
  flush(): void {
    for (const lane of this.lanes) {
      if (lane.busy || !hasWork(lane)) continue;
      const items = [...lane.items.values()];
      const transfer = items.flatMap((i) => i.give.flat());
      for (const s of lane.spawns) {
        const l = this.live.get(s.handle);
        if (l) l.sent = true;
      }
      const batch: ToWorker = { type: "batch", frees: lane.frees, spawns: lane.spawns, items };
      lane.spawns = [];
      lane.frees = [];
      lane.items = new Map();
      lane.busy = true;
      lane.port.post(batch, transfer);
    }
  }

  /** @internal Place a new effect in the worker with the least pixel area; returns its handle. */
  attach(spawn: Omit<Spawn, "handle">, area: number, events: FxEvents): number {
    const lane = this.lanes.reduce((best, l) => (l.area < best.area ? l : best));
    const handle = this.nextHandle++;
    lane.area += area;
    lane.spawns.push({ handle, ...spawn });
    lane.items.set(handle, { handle, dt: 0, stop: false, give: [] });
    this.live.set(handle, { lane, area, events, sent: false });
    return handle;
  }

  /** @internal Change what is queued for a live effect; does nothing once it is gone. */
  queue(handle: number, change: (item: Item) => void): void {
    const l = this.live.get(handle);
    if (!l) return;
    let item = l.lane.items.get(handle);
    if (!item) {
      item = { handle, dt: 0, stop: false, give: [] };
      l.lane.items.set(handle, item);
    }
    change(item);
  }

  /** @internal Forget an effect now; tell its worker to free it if the spawn was already sent. */
  free(handle: number): void {
    const l = this.live.get(handle);
    if (!l) return;
    this.forget(handle, l);
    if (l.sent) l.lane.frees.push(handle);
    else l.lane.spawns = l.lane.spawns.filter((s) => s.handle !== handle);
  }

  private forget(handle: number, l: Live): void {
    this.live.delete(handle);
    l.lane.area -= l.area;
    l.lane.items.delete(handle);
  }

  private receive(lane: Lane, data: unknown): void {
    const msg: FromWorker = fromWorkerSchema.parse(data);
    if (msg.type === "error") throw new Error(`pixfx worker: ${msg.message}`);
    if (msg.type === "ready") return;
    lane.busy = false;
    for (const r of msg.results) {
      // A reply for an effect destroyed since is dropped with its buffers.
      const l = this.live.get(r.handle);
      if (!l) continue;
      if (r.cues.length > 0) l.events.cues(r.cues);
      if (r.frame && this.live.get(r.handle) === l) l.events.frame(r.frame);
      if (r.done && this.live.get(r.handle) === l) {
        // The worker freed it already.
        this.forget(r.handle, l);
        l.events.done();
      }
    }
  }
}

function hasWork(lane: Lane): boolean {
  if (lane.spawns.length > 0 || lane.frees.length > 0) return true;
  for (const i of lane.items.values()) if (i.dt > 0 || i.stop) return true;
  return false;
}

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

/** Start `n` effect workers and wait until each has loaded the engine. Rejects if any fails to start. */
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
