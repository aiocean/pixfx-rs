/** How an image blends onto what is beneath it. `add`: dst.rgb += src.rgb. */
export type GfxBlend = "normal" | "add";

/** A pixel-crisp RGBA texture the effect writes into, then uploads. */
export interface GfxTexture {
  /** `width × height × 4` RGBA bytes, row-major. */
  readonly pixels: Uint8ClampedArray;
  /** Push `pixels` to the renderer. */
  upload(): void;
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
