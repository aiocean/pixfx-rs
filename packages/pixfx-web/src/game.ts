// What a game holds once: the engine's shared frames, the surfaces showing them, and the workers painting them.
// Every scene of the game draws from these, so a frame one scene painted shows in all of them.
import type * as Wasm from "../wasm/pixfx_wasm.js";
import type { WasmModule } from "./engine";
import { effectIds } from "./engine";
import type { GfxTextures } from "./gfx";
import type { FromWorker, Painted, Shape, ToWorker } from "./protocol";
import { fromWorkerSchema, shapeSchema } from "./protocol";
import type { Pixels } from "./surfaces";
import { OP_MAKE, OP_STRIDE, Surfaces } from "./surfaces";

/** The game's line to one painting worker. */
export interface WorkerPort {
  post(message: ToWorker, transfer: ArrayBuffer[]): void;
  listen(onMessage: (data: unknown) => void): void;
}

/** A scene the game hands painted frames to. */
export interface Delivering {
  readonly id: number;
  /** Take back the frame painted for `job`; returns the draw list (makes and refills of the painted bytes). */
  deliver(job: number, shape: Shape): Int32Array;
}

/** One worker and the effects whose state it keeps. */
interface Lane {
  port: WorkerPort;
  effects: number;
}

/** A job's effect, as a key. */
function effectKey(scene: number, handle: number): string {
  return `${scene}:${handle}`;
}

export class Game {
  readonly frames: Wasm.Frames;
  readonly effectId: (id: string) => Wasm.EffectId;
  private surfacesHeld: Surfaces | null = null;
  private readonly stages = new Map<number, Delivering>();
  private readonly lanes: Lane[];
  /** The worker painting each effect: an effect's state lives in one worker. */
  private readonly laneOf = new Map<string, Lane>();
  /** A scene drew since the frame last ended. */
  private drew = false;

  /** @param ports the painting workers; none paints on the main thread. */
  constructor(
    wasm: WasmModule,
    private readonly memory: WebAssembly.Memory,
    ports: readonly WorkerPort[],
  ) {
    this.frames = new wasm.Frames(ports.length > 0);
    this.effectId = effectIds(wasm);
    this.lanes = ports.map((port) => ({ port, effects: 0 }));
    for (const lane of this.lanes) lane.port.listen((data) => this.receive(data));
  }

  /** The textures every scene shares: made through the first scene's texture manager, which is the game's. */
  useTextures(textures: GfxTextures): void {
    this.surfacesHeld ??= new Surfaces(textures);
  }

  get surfaces(): Surfaces {
    if (!this.surfacesHeld) throw new Error("pixfx: no scene has started yet");
    return this.surfacesHeld;
  }

  /** Surfaces held and textures held, for `stats()`. */
  counts(): { surfaces: number; textures: number } {
    return { surfaces: this.surfacesHeld?.size ?? 0, textures: this.surfacesHeld?.textureCount ?? 0 };
  }

  add(stage: Delivering): void {
    this.stages.set(stage.id, stage);
  }

  remove(stage: Delivering): void {
    this.stages.delete(stage.id);
  }

  /** Carry out the makes, refills and frees of a draw list Rust made on the main thread. */
  apply(ops: Int32Array): void {
    this.surfaces.apply(ops, this.painted);
  }

  /** A scene drew this host frame: `ops` is its draw list. Carries out its makes and refills. */
  drawn(ops: Int32Array): void {
    this.drew = true;
    this.apply(ops);
  }

  /**
   * End the host frame once every scene drew: free what Rust dropped, then send the frames to paint to the workers.
   * Runs once per host frame, however many scenes call it.
   */
  endFrame(): void {
    if (!this.drew) return;
    this.drew = false;
    this.apply(this.frames.endFrame());
    if (this.lanes.length > 0) this.send();
  }

  /** Hold the frames of a baked clip; returns how many were loaded. */
  loadBaked(json: string, pages: Uint8Array[]): number {
    const ops = this.frames.loadBaked(json, pages);
    this.apply(ops);
    let n = 0;
    for (let i = 0; i < ops.length; i += OP_STRIDE) if (ops[i] === OP_MAKE) n += 1;
    return n;
  }

  /** Let go of every surface (the game shuts down). */
  clear(): void {
    if (!this.surfacesHeld) return;
    this.apply(this.frames.clear());
    this.surfacesHeld.clear();
  }

  /** The pixels of a surface Rust painted on the main thread: views on wasm memory, read before the next call. */
  private readonly painted = (id: number, w: number, h: number): Pixels => ({
    color: new Uint8ClampedArray(this.memory.buffer, this.frames.colorPtr(id), w * h * 4),
    light: new Uint8ClampedArray(this.memory.buffer, this.frames.lightPtr(id), w * h * 4),
  });

  /** Send each worker the jobs of the effects it paints (the first job of an effect goes to the least busy). */
  private send(): void {
    const { jobs, ended } = this.frames.takeJobs();
    if (jobs.length === 0 && ended.length === 0) return;
    const out = new Map<Lane, ToWorker & { type: "paint" }>();
    const message = (lane: Lane): ToWorker & { type: "paint" } => {
      let m = out.get(lane);
      if (!m) {
        m = { type: "paint", jobs: [], ended: [] };
        out.set(lane, m);
      }
      return m;
    };
    for (const job of jobs) {
      const key = effectKey(job.scene, job.handle);
      let lane = this.laneOf.get(key);
      if (!lane) {
        lane = this.lanes.reduce((best, l) => (l.effects < best.effects ? l : best));
        lane.effects += 1;
        this.laneOf.set(key, lane);
      }
      message(lane).jobs.push(job);
    }
    for (const [scene, handle] of ended) {
      const key = effectKey(scene, handle);
      const lane = this.laneOf.get(key);
      if (!lane) continue;
      this.laneOf.delete(key);
      lane.effects -= 1;
      message(lane).ended.push([scene, handle]);
    }
    for (const [lane, m] of out) lane.port.post(m, []);
  }

  private receive(data: unknown): void {
    const msg: FromWorker = fromWorkerSchema.parse(data);
    if (msg.type === "error") throw new Error(`pixfx worker: ${msg.message}`);
    if (msg.type === "ready" || !this.surfacesHeld) return;
    for (const p of msg.painted) this.take(this.surfacesHeld, p);
  }

  /** Hand a painted frame to its scene (or, for a frame scenes share, any scene: the frames are the game's). */
  private take(surfaces: Surfaces, p: Painted): void {
    const stage = this.stages.get(p.scene) ?? this.stages.values().next().value;
    if (!stage) return;
    const ops = stage.deliver(p.job, shapeSchema.parse(p.shape));
    const bytes: Pixels = { color: new Uint8ClampedArray(p.color), light: new Uint8ClampedArray(p.light) };
    surfaces.apply(ops, () => bytes);
  }
}
