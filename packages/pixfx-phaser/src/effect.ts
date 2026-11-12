import Phaser from "phaser";
import type { FxLike } from "./engine";
import type { GfxImage, GfxTexture } from "./gfx";
import type { FollowTarget } from "./lifecycle";
import { EffectCore } from "./lifecycle";
import { colorToRgba, lightToRgba } from "./pixels";

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
  private readonly all: readonly Layer[];

  /** @internal Created by the plugin's `play()`. */
  constructor(
    private readonly fx: FxLike,
    x: number,
    y: number,
    private readonly layers: Layers,
    depths: { back: number; front: number },
    private readonly counts: Counts,
    private readonly onGone: (effect: PixfxEffect) => void,
  ) {
    super();
    this.core = new EffectCore(fx, x, y);
    this.all = [layers.backColor, layers.backLight, layers.frontColor, layers.frontLight];
    this.setDepths(depths.back, depths.front);
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

  setDepths(back: number, front: number): this {
    this.layers.backColor.image.setDepth(back);
    this.layers.backLight.image.setDepth(back);
    this.layers.frontColor.image.setDepth(front);
    this.layers.frontLight.image.setDepth(front);
    return this;
  }

  /** Move to a world position; stops following. */
  setPosition(x: number, y: number): this {
    if (this.core.state === "destroyed") return this;
    this.core.moveTo(x, y);
    this.place();
    return this;
  }

  /** @internal Advance by `dt` seconds of scene time; draws and uploads only when it stepped. */
  step(dt: number): void {
    if (dt <= 0 || this.core.state === "destroyed") return;
    if (this.core.tick(dt) === "completed") {
      this.release();
      this.emit("complete", this);
      this.removeAllListeners();
      return;
    }
    this.render();
  }

  private render(): void {
    this.drawLayer("back", this.layers.backColor, this.layers.backLight);
    this.drawLayer("front", this.layers.frontColor, this.layers.frontLight);
    this.place();
  }

  private drawLayer(which: "back" | "front", color: Layer, light: Layer): void {
    this.fx.draw(which);
    colorToRgba(this.fx.color(), color.texture.pixels);
    color.texture.upload();
    lightToRgba(this.fx.light(), light.texture.pixels);
    light.texture.upload();
  }

  private place(): void {
    const p = this.core.topLeft();
    for (const l of this.all) l.image.setPosition(p.x, p.y);
  }

  /** Free everything once; false if it was already gone. */
  private release(): boolean {
    if (this.core.state === "destroyed") return false;
    this.core.destroy();
    this.counts.engines -= 1;
    for (const l of this.all) {
      l.image.destroy();
      this.counts.images -= 1;
      l.texture.destroy();
      this.counts.textures -= 1;
    }
    this.onGone(this);
    return true;
  }
}
