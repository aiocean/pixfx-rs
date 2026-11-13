import type { Backend, FxEvents, FxInstance, HeldFrame, Shape } from "./backend";
import { shapeOf } from "./backend";
import type { Engine } from "./engine";
import { prepare } from "./engine";
import { FrameCache, clipOf } from "./frames";
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
  /** The shared frames its worker believes the main thread holds: the ones it sent, less the drops sent back. */
  held: Set<string>;
  /** Frames the main thread holds that its worker did not send (baked), not yet told. */
  holds: string[];
  drops: string[];
  spawns: Spawn[];
  frees: number[];
  items: Map<number, Item>;
}

interface Live {
  lane: Lane;
  area: number;
  clip: string;
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
    private readonly clip: string,
    private readonly spawn: Omit<Spawn, "handle">,
  ) {}

  start(events: FxEvents): void {
    if (this.handle < 0) this.handle = this.pool.attach(this.spawn, this.clip, this.shape.width * this.shape.height, events);
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
}

/**
 * Effects drawn on worker threads. Effects of one clip (equal effect, options and margin) live in one worker,
 * so each of their frames is drawn once and sent once, cropped, into the game's frame cache; the first effect of
 * a clip goes to the worker holding the least pixel area. Work is queued per effect and sent once per frame
 * (`flush`) as one batch per worker that has no batch in flight; a busy worker's effects keep accumulating scene
 * time, so the main thread never waits. Frames come back in the reply, one frame later than they were asked for.
 */
export class WorkerPool implements Backend {
  private readonly lanes: Lane[];
  private readonly live = new Map<number, Live>();
  /** The lane of each clip with live effects, and how many it has. */
  private readonly clips = new Map<string, { lane: Lane; effects: number }>();
  private nextHandle = 0;

  /**
   * @param engine the main thread's engine, used to validate plays and learn effect sizes and clips.
   * @param cache where the shared frames the workers draw are held.
   */
  constructor(
    private readonly engine: Engine,
    ports: readonly WorkerPort[],
    private readonly cache = new FrameCache(),
  ) {
    if (ports.length === 0) throw new Error("pixfx: a worker pool needs at least one worker");
    this.lanes = ports.map((port) => ({ port, busy: false, area: 0, held: new Set(), holds: [], drops: [], spawns: [], frees: [], items: new Map() }));
    for (const lane of this.lanes) lane.port.listen((data) => this.receive(lane, data));
  }

  spawn(id: string, opts: unknown): { instance: FxInstance; play: PlayOptions } {
    const p = prepare(this.engine, id, opts);
    // Built and freed here so invalid options throw now, and to learn the canvas size, origin and clip.
    const fx = this.engine.create(p.id, p.engineOpts);
    const shape = shapeOf(fx);
    const shown = fx.shown();
    fx.free();
    const clip = shown === undefined ? `${p.id}#${this.nextHandle}` : clipOf(shown);
    return { instance: new PooledFx(this, shape, clip, { id: p.id, opts: p.engineOpts }), play: p.play };
  }

  /**
   * Drop the shared frames over the budget (each worker that sent one hears of it in its next batch), then send
   * one batch to every idle worker that has work. Drops alone wait for the next batch with work.
   */
  flush(): void {
    for (const id of this.cache.trim()) {
      for (const lane of this.lanes) if (lane.held.delete(id)) lane.drops.push(id);
    }
    for (const lane of this.lanes) {
      if (lane.busy || !hasWork(lane)) continue;
      for (const s of lane.spawns) {
        const l = this.live.get(s.handle);
        if (l) l.sent = true;
      }
      const batch: ToWorker = { type: "batch", holds: lane.holds, drops: lane.drops, frees: lane.frees, spawns: lane.spawns, items: [...lane.items.values()] };
      lane.holds = [];
      lane.drops = [];
      lane.spawns = [];
      lane.frees = [];
      lane.items = new Map();
      lane.busy = true;
      lane.port.post(batch, []);
    }
  }

  /**
   * Hold `frames` (baked) in the cache and tell every worker, in its next batch, that the main thread has them, so
   * no worker draws them. A frame already held keeps what it has.
   */
  hold(frames: readonly HeldFrame[]): void {
    for (const f of frames) {
      this.cache.put(f.id, f.frame);
      for (const lane of this.lanes) {
        if (lane.held.has(f.id)) continue;
        lane.held.add(f.id);
        lane.holds.push(f.id);
      }
    }
  }

  /** @internal Place a new effect in the worker of its clip (or the one with the least pixel area); returns its handle. */
  attach(spawn: Omit<Spawn, "handle">, clip: string, area: number, events: FxEvents): number {
    const c = this.clips.get(clip);
    const lane = c?.lane ?? this.lanes.reduce((best, l) => (l.area < best.area ? l : best));
    this.clips.set(clip, { lane, effects: (c?.effects ?? 0) + 1 });
    const handle = this.nextHandle++;
    lane.area += area;
    lane.spawns.push({ handle, ...spawn });
    lane.items.set(handle, newItem(handle));
    this.live.set(handle, { lane, area, clip, events, sent: false });
    return handle;
  }

  /** @internal Change what is queued for a live effect; does nothing once it is gone. */
  queue(handle: number, change: (item: Item) => void): void {
    const l = this.live.get(handle);
    if (!l) return;
    let item = l.lane.items.get(handle);
    if (!item) {
      item = newItem(handle);
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
    const c = this.clips.get(l.clip);
    if (c && c.effects > 1) c.effects -= 1;
    else this.clips.delete(l.clip);
  }

  private receive(lane: Lane, data: unknown): void {
    const msg: FromWorker = fromWorkerSchema.parse(data);
    if (msg.type === "error") throw new Error(`pixfx worker: ${msg.message}`);
    if (msg.type === "ready") return;
    lane.busy = false;
    for (const s of msg.frames) {
      this.cache.put(s.id, s.frame);
      lane.held.add(s.id);
    }
    for (const r of msg.results) {
      // A reply for an effect destroyed since is dropped.
      const l = this.live.get(r.handle);
      if (!l) continue;
      if (r.cues.length > 0) l.events.cues(r.cues);
      if (r.shown !== undefined && this.live.get(r.handle) === l) {
        const frame = this.cache.get(r.shown);
        // dropped while the reply was on its way: the worker hears of the drop and draws it again
        if (frame) l.events.show({ kind: "shared", frame });
        else this.queue(r.handle, (i) => (i.want = true));
      }
      if (r.own && this.live.get(r.handle) === l) l.events.show({ kind: "own", frame: r.own });
      if (r.done && this.live.get(r.handle) === l) {
        // The worker freed it already.
        this.forget(r.handle, l);
        l.events.done();
      }
    }
  }
}

function newItem(handle: number): Item {
  return { handle, dt: 0, stop: false, want: false };
}

function hasWork(lane: Lane): boolean {
  if (lane.spawns.length > 0 || lane.frees.length > 0) return true;
  for (const i of lane.items.values()) if (i.dt > 0 || i.stop || i.want) return true;
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
