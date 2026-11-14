import Phaser from "phaser";

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
export interface EffectHost {
  stop(handle: number): void;
  remove(effect: PixfxEffect): void;
  follow(handle: number, target: FollowTarget | null, dx: number, dy: number): void;
  moveTo(handle: number, x: number, y: number): void;
}

/**
 * One effect in a scene, played and drawn by the Rust scene. Emits `"complete"` when it finishes on its own and
 * `"destroy"` when destroyed; either way it is already gone when the event fires. Emits `"cue"` with each cue it
 * fires (`{t, pan, cue}`, a ripple's and a sound's `x`/`y` in the world), on the update that passes it.
 */
export class PixfxEffect extends Phaser.Events.EventEmitter {
  private gone = false;

  /** @internal Created by the plugin's `play()`. */
  constructor(
    private readonly host: EffectHost,
    readonly handle: number,
    private depths: { back: number; front: number },
    readonly scrollFactor: number,
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
  override destroy(): void {
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
