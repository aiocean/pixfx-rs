// The character an effect plays on, as the host shows it: a sprite frame's pixels, read once per image region and
// kept, and where it shows. Rust decides what the effect does with it (`scene.setActor`).
import type * as Wasm from "../wasm/pixfx_wasm.js";

/** A sprite frame as the host shows it now: where (`Wasm.ActorAt`) and its straight-alpha RGBA bytes, row by row. */
export interface ActorFrame {
  at: Wasm.ActorAt;
  rgba: Uint8Array;
}

/** Read each update: the sprite's frame as it shows now, or null to play on no character. */
export type ActorSource = () => ActorFrame | null;

/** A region of an image: what a sprite's current frame cuts from its texture. */
export interface Cut {
  x: number;
  y: number;
  w: number;
  h: number;
}

/** An image a game draws itself: `cut` of `image` drawn with its top-left at game pixel (`x`, `y`), `w` x `h` game
 * pixels (the cut's size unless given), mirrored with `flipX`. */
export interface DrawnImage {
  image: CanvasImageSource;
  cut: Cut;
  x: number;
  y: number;
  w?: number;
  h?: number;
  flipX?: boolean;
}

/**
 * A character a game draws itself (on a Canvas 2D context, say): `drawn` is read each update and says what it draws
 * now, or null for none. Each region of an image is read once.
 */
export function imageActor(drawn: () => DrawnImage | null): ActorSource {
  return () => {
    const d = drawn();
    if (!d) return null;
    const [w, h] = [Math.max(1, Math.round(d.cut.w)), Math.max(1, Math.round(d.cut.h))];
    return {
      at: {
        x: Math.round(d.x),
        y: Math.round(d.y),
        w,
        h,
        shownW: Math.max(1, Math.round(d.w ?? w)),
        shownH: Math.max(1, Math.round(d.h ?? h)),
        flipX: d.flipX ?? false,
      },
      rgba: cutPixels(d.image, d.cut),
    };
  };
}

/** Bytes read per image, by region: a sprite's frames are read once each, however long it plays. */
const held = new WeakMap<object, Map<string, Uint8Array>>();

/**
 * The RGBA bytes of `cut` of `image`, read once and kept while the image lives. Throws when the image cannot be read
 * (a cross-origin image without CORS taints the canvas).
 */
export function cutPixels(image: CanvasImageSource, cut: Cut): Uint8Array {
  const [x, y, w, h] = [Math.round(cut.x), Math.round(cut.y), Math.max(1, Math.round(cut.w)), Math.max(1, Math.round(cut.h))];
  let byCut = held.get(image);
  if (!byCut) {
    byCut = new Map();
    held.set(image, byCut);
  }
  const key = `${x},${y},${w},${h}`;
  const kept = byCut.get(key);
  if (kept) return kept;
  const ctx = context(w, h);
  ctx.clearRect(0, 0, w, h);
  ctx.drawImage(image, x, y, w, h, 0, 0, w, h);
  const data = ctx.getImageData(0, 0, w, h).data;
  const bytes = new Uint8Array(data.buffer, data.byteOffset, data.byteLength);
  byCut.set(key, bytes);
  return bytes;
}

/** A 2D context of `w` x `h` to read pixels through. */
function context(w: number, h: number): OffscreenCanvasRenderingContext2D | CanvasRenderingContext2D {
  const ctx =
    typeof OffscreenCanvas === "function"
      ? new OffscreenCanvas(w, h).getContext("2d", { willReadFrequently: true })
      : sized(document.createElement("canvas"), w, h).getContext("2d", { willReadFrequently: true });
  if (!ctx) throw new Error("pixfx: no 2D context to read a sprite's pixels");
  return ctx;
}

function sized(canvas: HTMLCanvasElement, w: number, h: number): HTMLCanvasElement {
  canvas.width = w;
  canvas.height = h;
  return canvas;
}
