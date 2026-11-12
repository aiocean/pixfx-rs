import Phaser from "phaser";
import type { FrameBuffers, FxInstance } from "./backend";
import type { GfxImage, GfxTexture } from "./gfx";
import type { FollowTarget } from "./lifecycle";
import { EffectCore } from "./lifecycle";
import type { Surface } from "./surface";

/** Live counts of what effects hold, read by `stats()` and the leak checks. */
export interface Counts {
  engines: number;
  textures: number;
  images: number;
}

/** One drawable: a texture and the image showing it. */
export interface Layer {
  texture: GfxTexture;
  image: GfxImage;
}

/** The four drawables of one effect, in draw order at equal depth (colour, then its light). */
export interface Layers {
  backColor: Layer;
  backLight: Layer;
  frontColor: Layer;
  frontLight: Layer;
}

/**
 * One effect in a scene: four images (back colour, back light, front colour, front light).
 * Emits `"complete"` when it finishes on its own and `"destroy"` when destroyed; either way it is
 * already gone (textures, images and engine instance freed) when the event fires.
 */
export class PixfxEffect extends Phaser.Events.EventEmitter {
  private readonly core: EffectCore;

  /** @internal Created by the plugin's `play()`. */
  constructor(
    private readonly instance: FxInstance,
    x: number,
    y: number,
    private readonly surface: Surface,
    depths: { back: number; front: number },
    private readonly counts: Counts,
    private readonly onGone: (effect: PixfxEffect) => void,
  ) {
    super();
    this.core = new EffectCore(instance.shape, x, y);
    this.setDepths(depths.back, depths.front);
    this.place();
    instance.start({ frame: (f) => this.show(f), done: () => this.complete() });
  }

  /** Let it end: loops fade out, one-shots are cut short. Does nothing once finished. */
  stop(): this {
    if (this.core.stop()) this.instance.stop();
    return this;
  }

  /** Remove at once; emits `"destroy"` (never `"complete"`). Does nothing if already gone. */
  override destroy(): void {
    if (!this.release()) return;
    this.emit("destroy", this);
    this.removeAllListeners();
  }

  /** Track `target` (plus offset) every frame; when the target goes inactive the effect stops there. */
  follow(target: FollowTarget, dx = 0, dy = 0): this {
    this.core.follow(target, dx, dy);
    return this;
  }

  /** Set the depths of the back and front images. Changes nothing for an effect played in a layer: the layer owns the depths. */
  setDepths(back: number, front: number): this {
    this.surface.setDepths(back, front);
    return this;
  }

  /** Move to a world position; stops following. */
  setPosition(x: number, y: number): this {
    if (this.gone()) return this;
    this.core.moveTo(x, y);
    this.place();
    return this;
  }

  /**
   * @internal Advance by `dt` seconds of scene time and move the images to the effect's place, so a follow
   * target is tracked every display frame. New frames of the effect arrive from its instance (at once on the
   * main thread, in the reply from a worker).
   */
  step(dt: number): void {
    if (dt <= 0 || this.gone()) return;
    if (!this.core.track()) this.stop();
    this.instance.advance(dt); // may complete the effect
    if (!this.gone()) this.place();
  }

  private gone(): boolean {
    return this.core.state === "destroyed";
  }

  private show(frame: FrameBuffers): void {
    if (this.gone()) return;
    const old = this.surface.show(frame);
    if (old) this.instance.recycle(old);
  }

  private place(): void {
    this.surface.moveTo(this.core.topLeft());
  }

  private complete(): void {
    if (!this.release()) return;
    this.emit("complete", this);
    this.removeAllListeners();
  }

  /** Free everything once; false if it was already gone. */
  private release(): boolean {
    if (!this.core.destroy()) return false;
    this.instance.free();
    this.counts.engines -= 1;
    this.surface.release();
    this.onGone(this);
    return true;
  }
}
