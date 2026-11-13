import type { Backend, FrameBuffers, FxEvents, FxInstance, Shape } from "./backend";
import { shapeOf } from "./backend";
import type { Engine } from "./engine";
import { prepare } from "./engine";
import type { PlayOptions } from "./options";
import type { Step } from "./runner";
import { FxRunner } from "./runner";

/** One engine instance on the main thread: every call runs at once and events fire before it returns. */
class InlineFx implements FxInstance {
  readonly shape: Shape;
  private events: FxEvents | null = null;
  private live = true;

  constructor(private readonly runner: FxRunner, shape: Shape) {
    this.shape = shape;
  }

  start(events: FxEvents): void {
    this.events = events;
    this.emit(this.runner.step(0));
  }

  advance(dt: number): void {
    if (this.live && dt > 0) this.emit(this.runner.step(dt));
  }

  stop(): void {
    if (this.live) this.runner.stop();
  }

  free(): void {
    if (!this.live) return;
    this.live = false;
    this.runner.free();
  }

  recycle(frame: FrameBuffers): void {
    if (this.live) this.runner.give(frame);
  }

  private emit(step: Step): void {
    if (step.cues.length > 0) this.events?.cues(step.cues);
    if (step.kind === "frame") this.events?.frame(step.frame);
    else if (step.kind === "done") {
      this.free();
      this.events?.done();
    }
  }
}

/** Effects drawn on the main thread: tests, `workers: 0`, and pages without `Worker`. */
export class InlineBackend implements Backend {
  constructor(private readonly engine: Engine) {}

  spawn(id: string, opts: unknown): { instance: FxInstance; play: PlayOptions } {
    const p = prepare(this.engine, id, opts);
    const fx = this.engine.create(p.id, p.engineOpts);
    return { instance: new InlineFx(new FxRunner(fx, p.fps), shapeOf(fx)), play: p.play };
  }

  flush(): void {}
}
