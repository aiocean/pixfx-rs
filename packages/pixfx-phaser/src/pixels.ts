/** Snap a world coordinate to a whole effect pixel. */
export function snap(v: number): number {
  return Math.round(v);
}

/** A `w × h` block copied from a source buffer (row length `srcStride`) to a destination (row length `dstStride`). */
export interface Blit {
  srcX: number;
  srcY: number;
  dstX: number;
  dstY: number;
  w: number;
  h: number;
  srcStride: number;
  dstStride: number;
}

/** Source-over (straight alpha) of ABGR `words` onto RGBA bytes `out`. */
export function compositeColor(words: Uint32Array, out: Uint8ClampedArray, b: Blit): void {
  for (let r = 0; r < b.h; r++) {
    for (let c = 0; c < b.w; c++) {
      const w = words[(b.srcY + r) * b.srcStride + b.srcX + c] ?? 0;
      const sa = (w >>> 24) & 0xff;
      if (sa === 0) continue;
      const o = ((b.dstY + r) * b.dstStride + b.dstX + c) * 4;
      const da = out[o + 3] ?? 0;
      if (sa === 255 || da === 0) {
        out[o] = w & 0xff;
        out[o + 1] = (w >>> 8) & 0xff;
        out[o + 2] = (w >>> 16) & 0xff;
        out[o + 3] = sa;
        continue;
      }
      const s = sa / 255;
      const d = (da / 255) * (1 - s);
      const a = s + d;
      out[o] = ((w & 0xff) * s + (out[o] ?? 0) * d) / a;
      out[o + 1] = (((w >>> 8) & 0xff) * s + (out[o + 1] ?? 0) * d) / a;
      out[o + 2] = (((w >>> 16) & 0xff) * s + (out[o + 2] ?? 0) * d) / a;
      out[o + 3] = a * 255;
    }
  }
}

/** Add xBGR light `words` onto RGBA bytes `out`: rgb saturates at 255, alpha is 255 wherever light is present. */
export function compositeLight(words: Uint32Array, out: Uint8ClampedArray, b: Blit): void {
  for (let r = 0; r < b.h; r++) {
    for (let c = 0; c < b.w; c++) {
      const w = (words[(b.srcY + r) * b.srcStride + b.srcX + c] ?? 0) & 0x00ffffff;
      if (w === 0) continue;
      const o = ((b.dstY + r) * b.dstStride + b.dstX + c) * 4;
      out[o] = (out[o] ?? 0) + (w & 0xff);
      out[o + 1] = (out[o + 1] ?? 0) + ((w >>> 8) & 0xff);
      out[o + 2] = (out[o + 2] ?? 0) + ((w >>> 16) & 0xff);
      out[o + 3] = 255;
    }
  }
}
