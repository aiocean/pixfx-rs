/** How an image blends onto what is beneath it. `add`: dst.rgb += src.rgb. */
export type GfxBlend = "normal" | "add";

/** A pixel-crisp RGBA texture, known to the game by its key. */
export interface GfxTexture {
  /** Show `rgba` (`width × height × 4` bytes, row-major, straight alpha); the bytes are copied, not kept. */
  upload(rgba: Uint8ClampedArray<ArrayBuffer>): void;
  /** Remove the texture from the game. */
  destroy(): void;
}

/** One image showing a texture, its top-left at its position. */
export interface GfxImage {
  /** Show the texture with key `key`, at its own size. */
  setTexture(key: string): void;
  setVisible(visible: boolean): void;
  setPosition(x: number, y: number): void;
  setDepth(depth: number): void;
  destroy(): void;
}

/** Everything the effect code needs from the renderer; the plugin implements it with Phaser. */
export interface Gfx {
  texture(key: string, width: number, height: number): GfxTexture;
  /** An image showing the texture `textureKey`, or hidden until it is given one (`null`). */
  image(textureKey: string | null, blend: GfxBlend, scrollFactor: number): GfxImage;
}
