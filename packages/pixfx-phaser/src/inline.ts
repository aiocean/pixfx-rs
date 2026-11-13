import type { Backend, FxEvents, FxInstance, Shape } from "./backend";
import { shapeOf } from "./backend";
import type { Engine } from "./engine";
import { prepare } from "./engine";
import { FrameCache } from "./frames";
import type { PlayOptions } from "./options";
import type { Step } from "./runner";
import { FxRunner } from "./runner";

/** One engine instance on the main thread: every call runs at once and events fire before it returns. */
class InlineFx implements FxInstance {
  readonly shape: Shape;
  private events: FxEvents | null = null;
  private live = true;
  private readonly held: (id: string) => boolean;

  constructor(
    private readonly runner: FxRunner,
    shape: Shape,
    private readonly cache: FrameCache,
  ) {
    this.shape = shape;
    this.held = (id) => cache.has(id);
  }

  start(events: FxEvents): void {
    this.events = events;
    this.emit(this.runner.step(0, this.held));
  }

  advance(dt: number): void {
    if (this.live && dt > 0) this.emit(this.runner.step(dt, this.held));
  }

  stop(): void {
    if (this.live) this.runner.stop();
  }

  free(): void {
    if (!this.live) return;
    this.live = false;
    this.runner.free();
  }

  private emit(step: Step): void {
    if (step.cues.length > 0) this.events?.cues(step.cues);
    if (step.kind === "shared") {
      const frame = step.frame ? this.cache.put(step.id, step.frame) : this.cache.get(step.id);
      if (frame) this.events?.show({ kind: "shared", frame });
    } else if (step.kind === "own") {
      this.events?.show({ kind: "own", frame: step.frame });
    } else if (step.kind === "done") {
      this.free();
      this.events?.done();
    }
  }
}

/** Effects drawn on the main thread: tests, `workers: 0`, and pages without `Worker`. */
export class InlineBackend implements Backend {
  constructor(
    private readonly engine: Engine,
    private readonly cache = new FrameCache(),
  ) {}

  spawn(id: string, opts: unknown): { instance: FxInstance; play: PlayOptions } {
    const p = prepare(this.engine, id, opts);
    const fx = this.engine.create(p.id, p.engineOpts);
    return { instance: new InlineFx(new FxRunner(fx), shapeOf(fx), this.cache), play: p.play };
  }

  flush(): void {
    this.cache.trim();
  }
}
