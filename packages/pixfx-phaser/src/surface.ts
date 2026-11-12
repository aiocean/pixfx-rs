import type { Counts, Layer, Layers } from "./effect";
import type { FxLike } from "./engine";
import { colorToRgba, lightToRgba } from "./pixels";

/** Where one effect draws: its own four images, or a share of a layer's. */
export interface Surface {
  /** Set the depths of the back and front image pairs (a shared surface ignores it). */
  setDepths(back: number, front: number): void;
  /** Show `fx` with its top-left at `topLeft`; `redraw` is true when the effect's pixels changed. */
  show(fx: FxLike, topLeft: { x: number; y: number }, redraw: boolean): void;
  /** Free what the surface holds for this effect. */
  release(): void;
}

/** Four textures and four images owned by one effect. */
export class OwnSurface implements Surface {
  private readonly all: readonly Layer[];

  constructor(
    private readonly layers: Layers,
    private readonly counts: Counts,
  ) {
    this.all = [layers.backColor, layers.backLight, layers.frontColor, layers.frontLight];
  }

  setDepths(back: number, front: number): void {
    this.layers.backColor.image.setDepth(back);
    this.layers.backLight.image.setDepth(back);
    this.layers.frontColor.image.setDepth(front);
    this.layers.frontLight.image.setDepth(front);
  }

  show(fx: FxLike, topLeft: { x: number; y: number }, redraw: boolean): void {
    if (redraw) {
      this.draw(fx, "back", this.layers.backColor, this.layers.backLight);
      this.draw(fx, "front", this.layers.frontColor, this.layers.frontLight);
    }
    for (const l of this.all) l.image.setPosition(topLeft.x, topLeft.y);
  }

  release(): void {
    for (const l of this.all) {
      l.image.destroy();
      this.counts.images -= 1;
      l.texture.destroy();
      this.counts.textures -= 1;
    }
  }

  private draw(fx: FxLike, which: "back" | "front", color: Layer, light: Layer): void {
    fx.draw(which);
    colorToRgba(fx.color(), color.texture.pixels);
    color.texture.upload();
    lightToRgba(fx.light(), light.texture.pixels);
    light.texture.upload();
  }
}
