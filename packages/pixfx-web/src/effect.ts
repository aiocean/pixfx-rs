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

/**
 * How an ordered volley flies toward its point: every piece curving onto the line through it and bursting on the
 * first body it meets (converge), straight at it and on through every body (pierce), or round it on a ring first
 * and then in as converge (orbit).
 */
export type Aim = "converge" | "pierce" | "orbit";

/**
 * Where one piece of an ordered volley struck, in world pixels: `index` counts the volley's pieces from 0, `body` is
 * the body it hit (`stage.setBody`), or null (the point a thrown volley lands on, or a `burst()`).
 */
export interface Strike {
  index: number;
  x: number;
  y: number;
  body: number | null;
}

/** What an effect asks of its scene. */
export interface EffectHost<P> {
  stop(handle: number): void;
  order(handle: number, x: number, y: number, aim: Aim, caster: number | undefined): boolean;
  trail(mover: PixfxEffect<P>, id: string, engine: Record<string, unknown>): PixfxEffect<P> | null;
  burst(handle: number): boolean;
  remove(effect: PixfxEffect<P>): void;
  follow(handle: number, target: FollowTarget | null, dx: number, dy: number): void;
  moveTo(handle: number, x: number, y: number): void;
  wrap(handle: number, source: ActorSource | null): void;
}

/**
 * One effect in a scene, played and drawn by the Rust scene. Emits `"complete"` when it finishes on its own and
 * `"destroy"` when destroyed; either way it is already gone when the event fires. Emits `"cue"` with each cue it
 * fires (`{t, pan, cue}`, a ripple's and a sound's `x`/`y` in the world), on the update that passes it. Emits
 * `"strike"` (`{index, x, y, body}`) as each piece of a volley it was ordered to launch (`attack`) strikes.
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

  /**
   * Order it to attack toward world (`x`, `y`), at any distance: an effect that takes orders
   * (`arcane/sword-formation`) launches its volley; each piece strikes the bodies it meets (`stage.setBody`),
   * emitting `"strike"`, and the effect ends once the last piece is done. `caster`, the attacker's own body, is
   * never struck. False when it takes no orders, was already ordered, or is gone.
   */
  attack(x: number, y: number, aim: Aim = "converge", caster?: number): boolean {
    return !this.gone && this.host.order(this.handle, x, y, aim, caster);
  }

  /**
   * Lay a trail under it: effect `id` (`projectiles/trail`) drawing the way it came, curving with it, as long as it
   * is fast, ending soon after it. Null when it is gone or `id` draws no trail; throws on an unknown id.
   */
  trail(id: string, engine: Record<string, unknown> = {}): PixfxEffect<P> | null {
    return this.gone ? null : this.host.trail(this, id, engine);
  }

  /**
   * For a game with collisions of its own: this volley piece hit something, so it strikes where its tip is and goes.
   * False unless it is a piece in flight.
   */
  burst(): boolean {
    return !this.gone && this.host.burst(this.handle);
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
