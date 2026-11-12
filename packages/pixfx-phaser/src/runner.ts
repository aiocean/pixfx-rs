import type { FxLike } from "./engine";
import type { FrameBuffers } from "./protocol";

export type Step = { kind: "done" } | { kind: "frame"; frame: FrameBuffers } | { kind: "same" };

/**
 * Drives one engine instance: advances it, and draws back and front into RGBA buffers only when it reached a
 * new frame of its authored fps. Buffers handed back with `give` are reused before new ones are allocated.
 */
export class FxRunner {
  /** Buffer sets allocated so far. */
  allocated = 0;
  private shown = -1;
  private freed = false;
  private readonly spare: FrameBuffers[] = [];
  private readonly bytes: number;

  constructor(
    private readonly fx: FxLike,
    private readonly fps: number,
  ) {
    this.bytes = fx.width() * fx.height() * 4;
  }

  /** Let it end; takes effect from the next step. */
  stop(): void {
    this.fx.stop();
  }

  /** Advance by `dt` seconds (0: none) and report what changed. The first step always draws. */
  step(dt: number): Step {
    if (dt > 0) this.fx.update(dt);
    if (this.fx.done()) return { kind: "done" };
    const frame = Math.floor(this.fx.t() * this.fps);
    if (frame === this.shown) return { kind: "same" };
    this.shown = frame;
    return { kind: "frame", frame: this.draw() };
  }

  /** Take back a set for reuse; a set of the wrong size (or detached by a transfer) is dropped. */
  give(set: FrameBuffers): void {
    if (set.every((b) => b.byteLength === this.bytes)) this.spare.push(set);
  }

  free(): void {
    if (this.freed) return;
    this.freed = true;
    this.fx.free();
  }

  private draw(): FrameBuffers {
    const set = this.spare.pop() ?? this.allocate();
    this.fx.draw("back");
    new Uint8ClampedArray(set[0]).set(this.fx.colorRgba());
    new Uint8ClampedArray(set[1]).set(this.fx.lightRgba());
    this.fx.draw("front");
    new Uint8ClampedArray(set[2]).set(this.fx.colorRgba());
    new Uint8ClampedArray(set[3]).set(this.fx.lightRgba());
    return set;
  }

  private allocate(): FrameBuffers {
    this.allocated += 1;
    const b = this.bytes;
    return [new ArrayBuffer(b), new ArrayBuffer(b), new ArrayBuffer(b), new ArrayBuffer(b)];
  }
}
