import Phaser from "phaser";
import type * as Wasm from "../wasm/pixfx_wasm.js";
import type { WorldCue } from "./stage";

export type { WorldCue };

/**
 * A scene's answer to impacts (`scene.pixfx.feel`), worked out by the Rust scene: the cues every effect fires
 * (shake, hit-stop, ripple, flash, sound) and the game's own impacts (`addTrauma`, `freeze`, `slow`) become one
 * camera shake, one hit-stop that freezes the scene's effects, slow motion and a flash. The plugin moves and tilts
 * the main camera by the shake and shows the flash over it; a game freezes its own world with `timeScale`.
 *
 * Emits `"cue"` with each cue any effect fires (`(cue: WorldCue, effect)`), whether or not the answer is on, so
 * the game plays sounds and reacts to anything else.
 */
export class PixfxFeel extends Phaser.Events.EventEmitter {
  private answering = true;
  private shakeK = 1;
  private flashK = 1;
  private scale = 1;

  /** @internal Created by the plugin; `scene` gives the scene's Rust scene, or null before `loadPixfx()` resolves. */
  constructor(private readonly scene: () => Wasm.Scene | null) {
    super();
  }

  /**
   * Answer cues (the default), or leave them to the game: off, effects play every second of scene time, nothing
   * shakes or flashes, and `"cue"` events still fire for the game's own answer.
   */
  setEnabled(on: boolean): this {
    this.answering = on;
    this.scene()?.setFeel(on);
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
    this.scene()?.addTrauma(k);
    return this;
  }

  /** The game's own hit-stop: freeze the scene's effects (and, through `timeScale`, the game) for `secs`. */
  freeze(secs: number): this {
    this.scene()?.freeze(secs);
    return this;
  }

  /** Slow motion: play runs at `speed` (0 to 1) of real time for `secs` seconds, after any freeze. */
  slow(secs: number, speed: number): this {
    this.scene()?.slow(secs, speed);
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
    const s = this.answering ? this.scene() : null;
    if (!s) return [0, 0];
    const [dx = 0, dy = 0] = s.shake(this.shakeK);
    return [dx, dy];
  }

  /** How far the camera tilts this frame, in radians (positive is clockwise), at the player's shake strength. */
  get roll(): number {
    const s = this.answering ? this.scene() : null;
    return s ? s.roll(this.shakeK) : 0;
  }

  /** How white the view is lifted this frame, 0 to 1, at the player's flash strength. */
  get flash(): number {
    const s = this.answering ? this.scene() : null;
    return s ? s.flash() * this.flashK : 0;
  }

  /** How shaken the camera is, 0 to 1. */
  get trauma(): number {
    return this.scene()?.trauma() ?? 0;
  }

  /** Play is frozen by a hit-stop now. */
  get frozen(): boolean {
    return this.answering && (this.scene()?.frozen() ?? false);
  }

  /** @internal The scene's update: `dt` seconds passed and `played` of them played. */
  played(dt: number, played: number): void {
    this.scale = dt > 0 ? played / dt : 1;
  }

  /** @internal A cue an effect fired, placed in the world. */
  take(cue: WorldCue, effect: unknown): void {
    this.emit("cue", cue, effect);
  }

  /** @internal */
  free(): void {
    this.removeAllListeners();
  }
}
