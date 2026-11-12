/** How an image blends onto what is beneath it. `add`: dst.rgb += src.rgb. */
export type GfxBlend = "normal" | "add";

/** A pixel-crisp RGBA texture. */
export interface GfxTexture {
  /** Show `rgba` (`width × height × 4` bytes, row-major, straight alpha); the bytes are copied, not kept. */
  upload(rgba: Uint8ClampedArray<ArrayBuffer>): void;
  /** Remove the texture from the game. */
  destroy(): void;
}

/** One image showing a texture, its top-left at its position. */
export interface GfxImage {
  setPosition(x: number, y: number): void;
  setDepth(depth: number): void;
  destroy(): void;
}

/** Everything the effect code needs from the renderer; the plugin implements it with Phaser. */
export interface Gfx {
  texture(key: string, width: number, height: number): GfxTexture;
  image(textureKey: string, blend: GfxBlend, scrollFactor: number): GfxImage;
}
