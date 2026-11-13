import type { Shape, View } from "./backend";
import type { Counts } from "./effect";
import type { FrameTextures, SharedFrame, Side, SideTextures } from "./frames";
import type { Gfx, GfxImage, GfxTexture } from "./gfx";

/** Where one effect is shown: its own four images, or a share of a layer's. */
export interface Surface {
  /** Set the depths of the back and front image pairs (a shared surface ignores it). */
  setDepths(back: number, front: number): void;
  /** Show what the effect shows now. */
  show(view: View): void;
  /** Put the effect's top-left at `topLeft`. */
  moveTo(topLeft: { x: number; y: number }): void;
  /** Free what the surface holds for this effect. */
  release(): void;
}

/** A colour image and the light image drawn over it. */
export interface SideImages {
  color: GfxImage;
  light: GfxImage;
}

/** The four images of one effect, in draw order at equal depth (colour, then its light). */
export interface EffectImages {
  back: SideImages;
  front: SideImages;
}

/**
 * Four images owned by one effect. A shared frame is shown from its textures, each side at its lit box; a fade
 * frame is uploaded into the effect's own four textures (the whole canvas), made the first time it fades.
 */
export class ImageSurface implements Surface {
  private topLeft = { x: 0, y: 0 };
  private textures: FrameTextures = { back: null, front: null };
  private shared: SharedFrame | null = null;
  private own: { textures: GfxTexture[]; keys: FrameTextures } | null = null;

  /** @param ownKey prefix of the keys of the effect's own textures, unique in the game. */
  constructor(
    private readonly images: EffectImages,
    private readonly gfx: Gfx,
    private readonly ownKey: string,
    private readonly shape: Shape,
    private readonly counts: Counts,
  ) {}

  setDepths(back: number, front: number): void {
    this.images.back.color.setDepth(back);
    this.images.back.light.setDepth(back);
    this.images.front.color.setDepth(front);
    this.images.front.light.setDepth(front);
  }

  show(view: View): void {
    if (view.kind === "shared") {
      view.frame.retain();
      this.shared?.release();
      this.shared = view.frame;
      this.textures = view.frame.textures(this.gfx);
    } else {
      this.shared?.release();
      this.shared = null;
      this.textures = this.upload(view.frame.back, view.frame.front);
    }
    showSide(this.images.back, this.textures.back);
    showSide(this.images.front, this.textures.front);
    this.place();
  }

  moveTo(topLeft: { x: number; y: number }): void {
    this.topLeft = topLeft;
    this.place();
  }

  release(): void {
    for (const i of [this.images.back, this.images.front]) {
      i.color.destroy();
      i.light.destroy();
    }
    this.counts.images -= 4;
    for (const t of this.own?.textures ?? []) t.destroy();
    this.counts.textures -= this.own?.textures.length ?? 0;
    this.own = null;
    this.shared?.release();
    this.shared = null;
  }

  private place(): void {
    placeSide(this.images.back, this.textures.back, this.topLeft);
    placeSide(this.images.front, this.textures.front, this.topLeft);
  }

  /** Upload a fade frame into the effect's own textures, made the first time. */
  private upload(back: Side, front: Side): FrameTextures {
    if (!this.own) {
      const textures: GfxTexture[] = [];
      const box = { x: 0, y: 0, w: this.shape.width, h: this.shape.height };
      const make = (name: string): string => {
        const key = `${this.ownKey}${name}`;
        textures.push(this.gfx.texture(key, box.w, box.h));
        return key;
      };
      const keys = {
        back: { color: make("back-color"), light: make("back-light"), box },
        front: { color: make("front-color"), light: make("front-light"), box },
      };
      this.counts.textures += textures.length;
      this.own = { textures, keys };
    }
    const [backColor, backLight, frontColor, frontLight] = this.own.textures;
    const { width: w, height: h } = this.shape;
    backColor?.upload(new Uint8ClampedArray(back.color), w, h);
    backLight?.upload(new Uint8ClampedArray(back.light), w, h);
    frontColor?.upload(new Uint8ClampedArray(front.color), w, h);
    frontLight?.upload(new Uint8ClampedArray(front.light), w, h);
    return this.own.keys;
  }
}

function showSide(images: SideImages, t: SideTextures | null): void {
  images.color.setVisible(t !== null);
  images.light.setVisible(t !== null);
  if (!t) return;
  images.color.setTexture(t.color, t.box.w, t.box.h);
  images.light.setTexture(t.light, t.box.w, t.box.h);
}

function placeSide(images: SideImages, t: SideTextures | null, topLeft: { x: number; y: number }): void {
  if (!t) return;
  images.color.setPosition(topLeft.x + t.box.x, topLeft.y + t.box.y);
  images.light.setPosition(topLeft.x + t.box.x, topLeft.y + t.box.y);
}
