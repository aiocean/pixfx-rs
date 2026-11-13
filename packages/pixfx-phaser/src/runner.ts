import type { FxLike } from "./engine";
import type { Box, Frame, Side, TimedCue } from "./protocol";

/**
 * What one step did, and the cues the effect fired on the way: finished; moved to shared frame `id` (drawn,
 * cropped to each side's lit box, when the holder lacked it; else `frame` is null); drew its own fade frame (the
 * whole canvas); or nothing to show.
 */
export type Step = (
  | { kind: "done" }
  | { kind: "shared"; id: string; frame: Frame | null }
  | { kind: "own"; frame: Frame }
  | { kind: "same" }
) & { cues: TimedCue[] };

/**
 * Drives one engine instance: advances it and reports the frame it shows. A running effect shows shared frames
 * and draws one only when whoever holds the shared frames lacks it; a fading effect draws its own frame on each
 * new clock frame.
 */
export class FxRunner {
  /** The shared frame last reported, or null while fading (or before the first step). */
  private id: string | null = null;
  /** The clock frame of the last own frame drawn. */
  private clock = -1;
  private freed = false;

  constructor(private readonly fx: FxLike) {}

  /** Let it end; takes effect from the next step. */
  stop(): void {
    this.fx.stop();
  }

  /**
   * Advance by `dt` seconds (0: none) and report what it shows, if that changed (or, with `again`, even if not).
   * `held(id)` says whether the holder of the shared frames has `id`. The first step always reports.
   */
  step(dt: number, held: (id: string) => boolean, again = false): Step {
    if (dt > 0) this.fx.update(dt);
    const cues = this.fx.takeCues();
    if (this.fx.done()) return { kind: "done", cues };
    const id = this.fx.shown();
    if (id === undefined) {
      this.id = null;
      const clock = this.fx.clockFrame();
      if (clock === this.clock && !again) return { kind: "same", cues };
      this.clock = clock;
      return { kind: "own", frame: this.draw(false), cues };
    }
    this.clock = -1;
    if (id === this.id && !again) return { kind: "same", cues };
    this.id = id;
    return { kind: "shared", id, frame: held(id) ? null : this.draw(true), cues };
  }

  free(): void {
    if (this.freed) return;
    this.freed = true;
    this.fx.free();
  }

  /** Draw back and front, each cropped to its lit box or the whole canvas, into new buffers. */
  private draw(crop: boolean): Frame {
    return { back: this.side("back", crop), front: this.side("front", crop) };
  }

  private side(layer: "back" | "front", crop: boolean): Side {
    this.fx.draw(layer);
    const box: Box = crop ? this.fx.litBox() : { x: 0, y: 0, w: this.fx.width(), h: this.fx.height() };
    const color = this.fx.colorRgba(box).slice().buffer;
    const light = this.fx.lightRgba(box).slice().buffer;
    return { box, color, light };
  }
}
