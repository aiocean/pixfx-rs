import type { GfxBlend, GfxImage, GfxTexture } from "../src/gfx";

/** A texture that keeps what it was last filled with, so a test reads what the renderer was given. */
export class FakeTexture implements GfxTexture {
  readonly color = "color";
  readonly light = "light";
  colorBytes = new Uint8ClampedArray(0);
  lightBytes = new Uint8ClampedArray(0);
  box = { w: 0, h: 0 };
  fills = 0;
  destroyed = false;
  constructor(
    readonly key: string,
    readonly width: number,
    readonly height: number,
  ) {}
  fill(color: Uint8ClampedArray<ArrayBuffer>, light: Uint8ClampedArray<ArrayBuffer>, w: number, h: number): void {
    if (color.length !== w * h * 4 || light.length !== w * h * 4 || w * 2 > this.width || h > this.height) {
      throw new Error(`fill of ${w}×${h} into a ${this.width}×${this.height} texture`);
    }
    if (this.destroyed) throw new Error(`fill of destroyed texture ${this.key}`);
    this.colorBytes = color.slice();
    this.lightBytes = light.slice();
    this.box = { w, h };
    this.fills += 1;
  }
  destroy(): void {
    this.destroyed = true;
  }
}

/** What an image was last shown as. */
export interface Shown {
  key: string;
  frame: string;
  x: number;
  y: number;
  w: number;
  h: number;
  depth: number;
  place: number;
}

/** An image whose shell places effects by a number (as Phaser does by scroll factor). */
export class FakeImage implements GfxImage<number> {
  shown: Shown | null = null;
  destroyed = false;
  constructor(readonly blend: GfxBlend) {}
  show(key: string, frame: string, x: number, y: number, w: number, h: number, depth: number, place: number): void {
    this.shown = { key, frame, x, y, w, h, depth, place };
  }
  hide(): void {
    this.shown = null;
  }
  destroy(): void {
    this.destroyed = true;
    this.shown = null;
  }
}

export class FakeGfx {
  readonly textures: FakeTexture[] = [];
  readonly images: FakeImage[] = [];
  private next = 0;
  readonly texture = (width: number, height: number): FakeTexture => {
    const t = new FakeTexture(`t${this.next++}`, width, height);
    this.textures.push(t);
    return t;
  };
  readonly image = (blend: GfxBlend): FakeImage => {
    const i = new FakeImage(blend);
    this.images.push(i);
    return i;
  };
  /** Images shown now. */
  showing(): FakeImage[] {
    return this.images.filter((i) => i.shown !== null);
  }
  /** The texture an image shows. */
  textureOf(image: FakeImage): FakeTexture | undefined {
    return this.textures.find((t) => t.key === image.shown?.key && !t.destroyed);
  }
  /** What every image shown now shows: its place and the bytes of its side, in show order. */
  picture(): { blend: GfxBlend; at: Omit<Shown, "key">; bytes: number[] }[] {
    return this.showing().map((i) => {
      const t = this.textureOf(i);
      const { key: _key, ...at } = i.shown ?? { key: "", frame: "", x: 0, y: 0, w: 0, h: 0, depth: 0, place: 0 };
      return { blend: i.blend, at, bytes: Array.from((at.frame === "color" ? t?.colorBytes : t?.lightBytes) ?? []) };
    });
  }
}
