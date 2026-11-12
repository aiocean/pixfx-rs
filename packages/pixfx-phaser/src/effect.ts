import Phaser from "phaser";
import type { FxLike } from "./engine";
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
  /** Index of the effect frame (at its authored fps) last drawn. */
  private shown: number;

  /** @internal Created by the plugin's `play()`. `fps` is the effect's authored frame rate (`meta.fps`). */
  constructor(
    private readonly fx: FxLike,
    private readonly fps: number,
    x: number,
    y: number,
    private readonly surface: Surface,
    depths: { back: number; front: number },
    private readonly counts: Counts,
    private readonly onGone: (effect: PixfxEffect) => void,
  ) {
    super();
    this.core = new EffectCore(fx, x, y);
    this.setDepths(depths.back, depths.front);
    this.shown = this.frameIndex();
    this.render();
  }

  /** Let it end: loops fade out, one-shots are cut short. Does nothing once finished. */
  stop(): this {
    this.core.stop();
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
    if (this.core.state === "destroyed") return this;
    this.core.moveTo(x, y);
    this.place();
    return this;
  }

  /**
   * @internal Advance by `dt` seconds of scene time. Draws and uploads only when the effect reached a new
   * frame of its authored fps (the frame shown is the engine's frame at the current time); otherwise only
   * moves the images, so a follow target is tracked every display frame.
   */
  step(dt: number): void {
    if (dt <= 0 || this.core.state === "destroyed") return;
    if (this.core.tick(dt) === "completed") {
      this.release();
      this.emit("complete", this);
      this.removeAllListeners();
      return;
    }
    const frame = this.frameIndex();
    if (frame === this.shown) {
      this.place();
      return;
    }
    this.shown = frame;
    this.render();
  }

  private frameIndex(): number {
    return Math.floor(this.fx.t() * this.fps);
  }

  private render(): void {
    this.surface.show(this.fx, this.core.topLeft(), true);
  }

  private place(): void {
    this.surface.show(this.fx, this.core.topLeft(), false);
  }

  /** Free everything once; false if it was already gone. */
  private release(): boolean {
    if (this.core.state === "destroyed") return false;
    this.core.destroy();
    this.counts.engines -= 1;
    this.surface.release();
    this.onGone(this);
    return true;
  }
}
