import type { FrameBuffers } from "./backend";
import type { Counts, Layer, Layers } from "./effect";

/** Where one effect is shown: its own four images, or a share of a layer's. */
export interface Surface {
  /** Set the depths of the back and front image pairs (a shared surface ignores it). */
  setDepths(back: number, front: number): void;
  /** Show a new frame. Returns a frame no longer read (this one or an older one), or null. */
  show(frame: FrameBuffers): FrameBuffers | null;
  /** Put the effect's top-left at `topLeft`. */
  moveTo(topLeft: { x: number; y: number }): void;
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

  /** Uploads at once, so the frame comes straight back. */
  show(frame: FrameBuffers): FrameBuffers {
    const [backColor, backLight, frontColor, frontLight] = frame;
    this.layers.backColor.texture.upload(new Uint8ClampedArray(backColor));
    this.layers.backLight.texture.upload(new Uint8ClampedArray(backLight));
    this.layers.frontColor.texture.upload(new Uint8ClampedArray(frontColor));
    this.layers.frontLight.texture.upload(new Uint8ClampedArray(frontLight));
    return frame;
  }

  moveTo(topLeft: { x: number; y: number }): void {
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
}
