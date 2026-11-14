/** How an image blends onto what is beneath it. `add`: dst.rgb += src.rgb. */
export type GfxBlend = "normal" | "add";

/**
 * Where one surface's pixels live on the GPU: two boxes of a texture (its own, or a region of an atlas), its colour
 * and its light, each at most half the width and the height it was made with.
 */
export interface GfxTexture {
  /** The texture the boxes are in. */
  readonly key: string;
  /** The names of the boxes' frames in that texture. */
  readonly color: string;
  readonly light: string;
  /**
   * Put `color` and `light` (each `w × h × 4` bytes, row-major, straight alpha) in their boxes and make the boxes
   * `w × h`. The bytes are copied, not kept.
   */
  fill(color: Uint8ClampedArray<ArrayBuffer>, light: Uint8ClampedArray<ArrayBuffer>, w: number, h: number): void;
  /** Give the boxes back. */
  destroy(): void;
}

/**
 * One image showing a frame of a texture, its top-left at its position. `P` is how the shell places an effect's
 * images beyond their depth (Phaser: the scroll factor; a shell with nothing more uses `null`).
 */
export interface GfxImage<P> {
  /** Show frame `frame` of texture `key` at `(x, y)`, `w × h` pixels, at `depth`, placed by `place`. */
  show(key: string, frame: string, x: number, y: number, w: number, h: number, depth: number, place: P): void;
  hide(): void;
  destroy(): void;
}

/** Makes room for one surface of up to `width / 2 × height` pixels per side; shared by every scene of the game. */
export type GfxTextures = (width: number, height: number) => GfxTexture;

/** Makes one scene's images. */
export type GfxImages<P> = (blend: GfxBlend) => GfxImage<P>;
