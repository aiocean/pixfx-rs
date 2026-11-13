import Phaser from "phaser";
import type { Engine, FeelLike } from "./engine";
import type { TimedCue } from "./protocol";

/** A cue an effect fired, placed in the world: a ripple's `x`/`y` and a sound's `x` are world positions. */
export type WorldCue = TimedCue;

/**
 * A scene's answer to impacts (`scene.pixfx.feel`): the cues every effect fires (shake, hit-stop, ripple, flash,
 * sound) and the game's own impacts (`addTrauma`, `freeze`, `slow`) become one camera shake, one hit-stop that
 * freezes the scene's effects, slow motion and a flash. The plugin moves and tilts the main camera by the shake
 * and shows the flash over it; a game freezes its own world with `timeScale`.
 *
 * Emits `"cue"` with each cue any effect fires (`(cue: WorldCue, effect)`), whether or not the answer is on, so
 * the game plays sounds and reacts to anything else. No Phaser calls: the plugin does the drawing.
 */
export class PixfxFeel extends Phaser.Events.EventEmitter {
  private answer: FeelLike | null = null;
  private answering = true;
  private shakeK = 1;
  private flashK = 1;
  private scale = 1;

  /** @internal Created by the plugin; `engine` gives the loaded engine, or null before `loadPixfx()` resolves. */
  constructor(private readonly engine: () => Engine | null) {
    super();
  }

  /**
   * Answer cues (the default), or leave them to the game: off, effects play every second of scene time, nothing
   * shakes or flashes, and `"cue"` events still fire for the game's own answer.
   */
  setEnabled(on: boolean): this {
    this.answering = on;
    return this;
  }

  get enabled(): boolean {
    return this.answering;
  }

  /** The player's shake setting: 1 as designed (the default), 0 holds the camera still. */
  setShakeStrength(k: number): this {
    this.shakeK = Number.isFinite(k) && k > 0 ? k : 0;
    return this;
  }

  /** The player's flash setting: 1 as designed (the default), 0 never flashes (photosensitive players). */
  setFlashStrength(k: number): this {
    this.flashK = Number.isFinite(k) && k > 0 ? Math.min(1, k) : 0;
    return this;
  }

  /** The game's own impact shakes the camera: `k` (0 to 1) is added to the trauma. */
  addTrauma(k: number): this {
    this.feel()?.addTrauma(k);
    return this;
  }

  /** The game's own hit-stop: freeze the scene's effects (and, through `timeScale`, the game) for `secs`. */
  freeze(secs: number): this {
    this.feel()?.freeze(secs);
    return this;
  }

  /** Slow motion: play runs at `speed` (0 to 1) of real time for `secs` seconds, after any freeze. */
  slow(secs: number, speed: number): this {
    this.feel()?.slow(secs, speed);
    return this;
  }

  /**
   * How fast play ran in the last scene update: 0 in a hit-stop, a fraction in slow motion, 1 otherwise.
   * Multiply the game's own movement by it to freeze with the effects.
   */
  get timeScale(): number {
    return this.scale;
  }

  /** Where the camera sits this frame, `[dx, dy]` in game pixels, at the player's shake strength. */
  get shake(): [number, number] {
    const f = this.answering ? this.answer : null;
    return f ? f.shake(this.shakeK) : [0, 0];
  }

  /** How far the camera tilts this frame, in radians (positive is clockwise), at the player's shake strength. */
  get roll(): number {
    const f = this.answering ? this.answer : null;
    return f ? f.roll(this.shakeK) : 0;
  }

  /** How white the view is lifted this frame, 0 to 1, at the player's flash strength. */
  get flash(): number {
    const f = this.answering ? this.answer : null;
    return f ? f.flash() * this.flashK : 0;
  }

  /** How shaken the camera is, 0 to 1. */
  get trauma(): number {
    return this.answer?.trauma() ?? 0;
  }

  /** Play is frozen by a hit-stop now. */
  get frozen(): boolean {
    return this.answering && (this.answer?.frozen() ?? false);
  }

  /** @internal Cues an effect fired, placed in the world: emitted, and answered when on. */
  take(cues: readonly WorldCue[], effect: unknown): void {
    for (const c of cues) this.emit("cue", c, effect);
    if (this.answering) this.feel()?.take(cues);
  }

  /** @internal Let `deltaMs` real milliseconds pass; returns the milliseconds the scene's effects play. */
  advance(deltaMs: number): number {
    if (!this.answering || !(deltaMs > 0)) {
      this.scale = 1;
      return deltaMs;
    }
    const f = this.feel();
    if (!f) return deltaMs;
    const real = deltaMs / 1000;
    const run = f.advance(real);
    this.scale = run / real;
    return run * 1000;
  }

  /** @internal Free the engine's side. */
  free(): void {
    this.answer?.free();
    this.answer = null;
    this.removeAllListeners();
  }

  private feel(): FeelLike | null {
    if (!this.answer) this.answer = this.engine()?.feel() ?? null;
    return this.answer;
  }
}
