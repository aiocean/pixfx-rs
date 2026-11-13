// Where effects' engine instances live: on the main thread (`InlineBackend`) or in workers (`WorkerPool`).
import type { FxLike } from "./engine";
import type { PlayOptions } from "./options";
import type { FrameBuffers, TimedCue } from "./protocol";

export type { FrameBuffers, TimedCue };

/** Canvas size of an effect and where its origin sits on the canvas. */
export interface Shape {
  width: number;
  height: number;
  originX: number;
  originY: number;
}

export function shapeOf(fx: FxLike): Shape {
  return { width: fx.width(), height: fx.height(), originX: fx.originX(), originY: fx.originY() };
}

/** What an instance tells its effect. */
export interface FxEvents {
  /** A new frame of the effect; hand the buffers back with `recycle` once they are no longer read. */
  frame(frame: FrameBuffers): void;
  /** The effect fired cues (before any frame or end of the same step), oldest first. */
  cues(cues: TimedCue[]): void;
  /** The effect finished; the instance is already freed. */
  done(): void;
}

/** The main thread's handle on one engine instance, wherever it runs. */
export interface FxInstance {
  readonly shape: Shape;
  /** Begin: the first frame (time 0) arrives through `events.frame`. */
  start(events: FxEvents): void;
  /** Advance by `dt` seconds of scene time. */
  advance(dt: number): void;
  /** Let it end: loops fade out, one-shots are cut short. */
  stop(): void;
  /** Free it; no event follows. Does nothing once freed or done. */
  free(): void;
  /** Return a frame's buffers for reuse. */
  recycle(frame: FrameBuffers): void;
}

export interface Backend {
  /** Validate and create effect `id`; throws on an unknown id or invalid options. */
  spawn(id: string, opts: unknown): { instance: FxInstance; play: PlayOptions };
  /** Send the work queued this frame (a no-op where work runs at once). */
  flush(): void;
}
