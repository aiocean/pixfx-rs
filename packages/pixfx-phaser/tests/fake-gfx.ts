import type { Gfx, GfxBlend, GfxImage, GfxTexture } from "../src/gfx";

/** Keeps a copy of the last upload in `pixels`, so a test reads what the renderer was given. */
export class FakeTexture implements GfxTexture {
  readonly pixels: Uint8ClampedArray;
  uploads = 0;
  uploadsWhenDestroyed = 0;
  destroyed = false;
  constructor(
    readonly key: string,
    readonly width: number,
    readonly height: number,
  ) {
    this.pixels = new Uint8ClampedArray(width * height * 4);
  }
  upload(rgba: Uint8ClampedArray<ArrayBuffer>, width: number, height: number): void {
    if (rgba.length !== width * height * 4 || width > this.width || height > this.height) {
      throw new Error(`upload of ${rgba.length} bytes (${width}×${height}) into a ${this.width}×${this.height} texture`);
    }
    for (let y = 0; y < height; y++) this.pixels.set(rgba.subarray(y * width * 4, (y + 1) * width * 4), y * this.width * 4);
    this.uploads += 1;
    if (this.destroyed) this.uploadsWhenDestroyed += 1;
  }
  destroy(): void {
    this.destroyed = true;
  }
  px(x: number, y: number): number[] {
    const o = (y * this.width + x) * 4;
    return Array.from(this.pixels.slice(o, o + 4));
  }
  written(): boolean {
    return this.pixels.some((v) => v !== 0);
  }
}

export class FakeImage implements GfxImage {
  x = Number.NaN;
  y = Number.NaN;
  depth = Number.NaN;
  destroyed = false;
  visible: boolean;
  constructor(
    /** The key of the texture it shows; null until it is given one. */
    public key: string | null,
    readonly blend: GfxBlend,
    readonly scrollFactor: number,
  ) {
    this.visible = key !== null;
  }
  /** The top-left box of its texture it shows. */
  crop = { w: 0, h: 0 };
  setTexture(key: string, width: number, height: number): void {
    this.key = key;
    this.crop = { w: width, h: height };
  }
  setVisible(visible: boolean): void {
    this.visible = visible;
  }
  setPosition(x: number, y: number): void {
    this.x = x;
    this.y = y;
  }
  setDepth(depth: number): void {
    this.depth = depth;
  }
  destroy(): void {
    this.destroyed = true;
  }
}

export class FakeGfx implements Gfx {
  readonly textures: FakeTexture[] = [];
  readonly images: FakeImage[] = [];
  texture(key: string, width: number, height: number): FakeTexture {
    const t = new FakeTexture(key, width, height);
    this.textures.push(t);
    return t;
  }
  image(key: string | null, blend: GfxBlend, scrollFactor: number): FakeImage {
    const i = new FakeImage(key, blend, scrollFactor);
    this.images.push(i);
    return i;
  }
  uploads(): number {
    return this.textures.reduce((n, t) => n + t.uploads, 0);
  }
  /** The live texture an image shows. */
  shownBy(image: FakeImage): FakeTexture | undefined {
    return this.textures.find((t) => t.key === image.key && !t.destroyed);
  }
}
