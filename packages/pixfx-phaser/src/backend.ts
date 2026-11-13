// Where effects' engine instances live: on the main thread (`InlineBackend`) or in workers (`WorkerPool`).
import type { FxLike } from "./engine";
import type { Frame, SharedFrame } from "./frames";
import type { PlayOptions } from "./options";
import type { TimedCue } from "./protocol";

export type { TimedCue };

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

/** What an effect shows: a frame it shares with every effect on the same one, or its own fade frame. */
export type View = { kind: "shared"; frame: SharedFrame } | { kind: "own"; frame: Frame };

/** What an instance tells its effect. */
export interface FxEvents {
  /** The effect shows something new. */
  show(view: View): void;
  /** The effect fired cues (before any frame or end of the same step), oldest first. */
  cues(cues: TimedCue[]): void;
  /** The effect finished; the instance is already freed. */
  done(): void;
}

/** The main thread's handle on one engine instance, wherever it runs. */
export interface FxInstance {
  readonly shape: Shape;
  /** Begin: the first frame (time 0) arrives through `events.show`. */
  start(events: FxEvents): void;
  /** Advance by `dt` seconds of scene time. */
  advance(dt: number): void;
  /** Let it end: loops fade out, one-shots are cut short. */
  stop(): void;
  /** Free it; no event follows. Does nothing once freed or done. */
  free(): void;
}

export interface Backend {
  /** Validate and create effect `id`; throws on an unknown id or invalid options. */
  spawn(id: string, opts: unknown): { instance: FxInstance; play: PlayOptions };
  /** Once per game step: drop shared frames over the budget, then send the work queued (where work runs elsewhere). */
  flush(): void;
}
