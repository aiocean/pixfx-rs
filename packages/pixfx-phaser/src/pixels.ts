/** Unpack ABGR words into RGBA bytes. Returns true if any pixel has non-zero alpha. */
export function colorToRgba(words: Uint32Array, out: Uint8ClampedArray): boolean {
  let visible = false;
  for (let i = 0; i < words.length; i++) {
    const w = words[i] ?? 0;
    const o = i * 4;
    out[o] = w & 0xff;
    out[o + 1] = (w >>> 8) & 0xff;
    out[o + 2] = (w >>> 16) & 0xff;
    const a = (w >>> 24) & 0xff;
    out[o + 3] = a;
    if (a !== 0) visible = true;
  }
  return visible;
}

/** Unpack xBGR light words: lit pixels become `r,g,b,255`, unlit `0,0,0,0`. Returns true if any pixel is lit. */
export function lightToRgba(words: Uint32Array, out: Uint8ClampedArray): boolean {
  let lit = false;
  for (let i = 0; i < words.length; i++) {
    const w = (words[i] ?? 0) & 0x00ffffff;
    const o = i * 4;
    if (w === 0) {
      out[o] = 0;
      out[o + 1] = 0;
      out[o + 2] = 0;
      out[o + 3] = 0;
    } else {
      out[o] = w & 0xff;
      out[o + 1] = (w >>> 8) & 0xff;
      out[o + 2] = (w >>> 16) & 0xff;
      out[o + 3] = 255;
      lit = true;
    }
  }
  return lit;
}

/** Snap a world coordinate to a whole effect pixel. */
export function snap(v: number): number {
  return Math.round(v);
}
