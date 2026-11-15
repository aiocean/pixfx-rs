import EventEmitter from "eventemitter3";
import type { ActorSource } from "./actor";

/** Live counts of what a scene's effects hold, read by `stats()` and the leak checks. */
export interface Counts {
  engines: number;
  textures: number;
  images: number;
}

/** A game object an effect follows: when it goes inactive the effect stops where it was. */
export interface FollowTarget {
  x: number;
  y: number;
  active: boolean;
}

/** What an effect asks of its scene. */
export interface EffectHost<P> {
  stop(handle: number): void;
  remove(effect: PixfxEffect<P>): void;
  follow(handle: number, target: FollowTarget | null, dx: number, dy: number): void;
  moveTo(handle: number, x: number, y: number): void;
  wrap(handle: number, source: ActorSource | null): void;
}

/**
 * One effect in a scene, played and drawn by the Rust scene. Emits `"complete"` when it finishes on its own and
 * `"destroy"` when destroyed; either way it is already gone when the event fires. Emits `"cue"` with each cue it
 * fires (`{t, pan, cue}`, a ripple's and a sound's `x`/`y` in the world), on the update that passes it.
 */
export class PixfxEffect<P = null> extends EventEmitter {
  private gone = false;

  /** @internal Created by a shell's `play()`; `place` is how the shell places its images beyond depth. */
  constructor(
    private readonly host: EffectHost<P>,
    readonly handle: number,
    private depths: { back: number; front: number },
    readonly place: P,
  ) {
    super();
  }

  /** The depth of its back (`layer` 0) or front (`layer` 1) images. */
  depthOf(layer: number): number {
    return layer === 0 ? this.depths.back : this.depths.front;
  }

  /** Let it end: loops fade out, one-shots are cut short. Does nothing once gone. */
  stop(): this {
    if (!this.gone) this.host.stop(this.handle);
    return this;
  }

  /** Remove at once; emits `"destroy"` (never `"complete"`). Does nothing if already gone. */
  destroy(): void {
    if (this.gone) return;
    this.gone = true;
    this.host.remove(this);
    this.emit("destroy", this);
    this.removeAllListeners();
  }

  /** Track `target` (plus offset) every update; when the target goes inactive the effect stops there. */
  follow(target: FollowTarget, dx = 0, dy = 0): this {
    if (!this.gone) this.host.follow(this.handle, target, dx, dy);
    return this;
  }

  /**
   * Play on a character: `source` is read every update and gives its sprite's frame as it shows now. An effect that
   * wraps its actor (as `feedback/power-up` does) grips that outline and follows each pose; others ignore it.
   * Shells give a source for their sprites; `null` plays on no character again.
   */
  wrap(source: ActorSource | null): this {
    if (!this.gone) this.host.wrap(this.handle, source);
    return this;
  }

  /** Set the depths of the back and front images. */
  setDepths(back: number, front: number): this {
    this.depths = { back, front };
    return this;
  }

  /** Move to a world position; stops following. */
  setPosition(x: number, y: number): this {
    if (this.gone) return this;
    this.host.follow(this.handle, null, 0, 0);
    this.host.moveTo(this.handle, x, y);
    return this;
  }

  /** @internal It finished on its own (its scene already let go of it). */
  complete(): void {
    if (this.gone) return;
    this.gone = true;
    this.emit("complete", this);
    this.removeAllListeners();
  }

  /** @internal Its scene shut down: it is destroyed without its scene being asked. */
  end(): void {
    if (this.gone) return;
    this.gone = true;
    this.emit("destroy", this);
    this.removeAllListeners();
  }
}
