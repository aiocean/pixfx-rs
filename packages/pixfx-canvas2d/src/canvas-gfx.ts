// Canvas 2D's host objects for the surfaces Rust paints: a small canvas per surface (colour on its left half, light
// on its right) filled with the bytes a worker painted, and a scene's images, which a host draws layer by layer onto
// its own canvas, colour normally and light with "lighter".
import type { GfxImage, GfxImages, GfxTexture, GfxTextures } from "pixfx-web";

/** Where a surface's pixels are held: a canvas to draw from, and how bytes get into it. */
export interface Holder {
  readonly source: CanvasImageSource;
  /** Its width: colour fills the left half, light the right. */
  readonly width: number;
  /** Put `bytes` (`w × h` RGBA, straight alpha, top row first) at (`x`, 0). */
  put(bytes: Uint8ClampedArray<ArrayBuffer>, x: number, w: number, h: number): void;
}

/** Makes a holder of `width × height` pixels. */
export type MakeHolder = (width: number, height: number) => Holder;

/** The context images are drawn on: what of `CanvasRenderingContext2D` the shell uses. */
export type DrawContext = Pick<
  CanvasRenderingContext2D,
  "drawImage" | "save" | "restore" | "globalCompositeOperation" | "imageSmoothingEnabled"
>;

/** A holder on an `OffscreenCanvas`. */
export function canvasHolder(width: number, height: number): Holder {
  const canvas = new OffscreenCanvas(width, height);
  const ctx = canvas.getContext("2d");
  if (!ctx) throw new Error("pixfx: a canvas has no 2d context");
  return {
    source: canvas,
    width,
    put(bytes, x, w, h) {
      ctx.putImageData(new ImageData(bytes, w, h), x, 0);
    },
  };
}

/** The holder each texture key names, so a scene's images draw from it. */
export type HolderOf = (key: string) => Holder | undefined;

/** The surfaces' textures: a holder each, colour beside light. */
export function canvasTextures(make: MakeHolder = canvasHolder): { textures: GfxTextures; holderOf: HolderOf } {
  const held = new Map<string, Holder>();
  let next = 0;
  const textures: GfxTextures = (width, height) => {
    const key = `s${next++}`;
    const holder = make(width, height);
    const half = width / 2;
    held.set(key, holder);
    const texture: GfxTexture = {
      key,
      color: "c",
      light: "l",
      fill(color, light, w, h) {
        holder.put(color, 0, w, h);
        holder.put(light, half, w, h);
      },
      destroy() {
        held.delete(key);
      },
    };
    return texture;
  };
  return { textures, holderOf: (key) => held.get(key) };
}

/** One image as last shown: the box of its holder it shows and where, in game pixels. */
interface Shown {
  source: CanvasImageSource;
  sx: number;
  sy: number;
  x: number;
  y: number;
  w: number;
  h: number;
  depth: number;
}

interface Image {
  add: boolean;
  shown: Shown | null;
}

/** A scene's images, in the order they were made (the order the scene shows them), and how to draw them. */
export interface CanvasImages {
  images: GfxImages<null>;
  /**
   * Draw the images shown at `depth` (every depth when null) onto `ctx` in its current transform, one game pixel as
   * `scale × scale` canvas pixels, lower depths first and equal depths in show order. The context's compositing and
   * smoothing are put back.
   */
  draw(ctx: DrawContext, scale: number, depth: number | null): void;
}

export function canvasImages(holderOf: HolderOf): CanvasImages {
  const all: Image[] = [];
  const images: GfxImages<null> = (blend) => {
    const image: Image = { add: blend === "add", shown: null };
    all.push(image);
    const it: GfxImage<null> = {
      show(key, frame, x, y, w, h, depth) {
        const holder = holderOf(key);
        if (!holder) {
          image.shown = null;
          return;
        }
        // the holder is kept, not looked up when drawn: a surface Rust frees at the end of the frame still shows
        // the pixels it held this frame
        image.shown = { source: holder.source, sx: frame === "l" ? holder.width / 2 : 0, sy: 0, x, y, w, h, depth };
      },
      hide() {
        image.shown = null;
      },
      destroy() {
        image.shown = null;
        const i = all.indexOf(image);
        if (i >= 0) all.splice(i, 1);
      },
    };
    return it;
  };
  const draw = (ctx: DrawContext, scale: number, depth: number | null): void => {
    const shown: { s: Shown; add: boolean }[] = [];
    for (const image of all) {
      const s = image.shown;
      if (s && (depth === null || s.depth === depth)) shown.push({ s, add: image.add });
    }
    if (shown.length === 0) return;
    // a stable sort: equal depths keep show order, each surface's colour before its light
    shown.sort((a, b) => a.s.depth - b.s.depth);
    ctx.save();
    ctx.imageSmoothingEnabled = false;
    for (const { s, add } of shown) {
      ctx.globalCompositeOperation = add ? "lighter" : "source-over";
      ctx.drawImage(s.source, s.sx, s.sy, s.w, s.h, s.x * scale, s.y * scale, s.w * scale, s.h * scale);
    }
    ctx.restore();
  };
  return { images, draw };
}
