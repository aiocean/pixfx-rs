// The surfaces of a game, as the Rust scene makes, refills and frees them: room on the GPU for each (a region of an
// atlas, or a texture of its own), colour and light side by side. Shared by every scene of the game. Room is kept by
// size for reuse, so frames coming and going stop asking for more once the game has enough.
import type { GfxTexture, GfxTextures } from "./gfx";

/** Numbers per op in a draw list, and the ops (as `pixfx-wasm` `scene.rs` writes them). */
export const OP_STRIDE = 8;
export const OP_MAKE = 1;
export const OP_REFILL = 2;
export const OP_SHOW = 3;
export const OP_FREE = 4;

/** Textures come in sizes rounded up to this many pixels, so one a frame gave back fits later frames of about its size. */
export const TEXTURE_STEP = 32;

/** Textures no surface holds, kept for reuse: at most this many; more are destroyed. */
export const TEXTURE_POOL_MAX = 256;

/** The bytes of a surface made or refilled: its colour and its light, `w × h × 4` each. */
export interface Pixels {
  color: Uint8ClampedArray<ArrayBuffer>;
  light: Uint8ClampedArray<ArrayBuffer>;
}

/** Room for a surface, at its rounded size. */
interface Pooled {
  texture: GfxTexture;
  bucket: string;
}

/** What a surface holds: its texture (none while it shows nothing), its size, and which sides hold pixels. */
export interface Held {
  pooled: Pooled | null;
  w: number;
  h: number;
  sides: number;
}

export class Surfaces {
  private readonly held = new Map<number, Held>();
  private readonly pool = new Map<string, Pooled[]>();
  private pooled = 0;

  constructor(private readonly textures: GfxTextures) {}

  /** Surfaces held. */
  get size(): number {
    return this.held.size;
  }

  /** Textures held, by surfaces and kept for reuse. */
  get textureCount(): number {
    let n = this.pooled;
    for (const h of this.held.values()) if (h.pooled) n += 1;
    return n;
  }

  get(id: number): Held | undefined {
    return this.held.get(id);
  }

  /**
   * Carry out the makes, refills and frees of draw list `ops`, in order; `pixels(id, w, h)` gives the bytes of a
   * surface made or refilled. Shows are a scene's to carry out.
   */
  apply(ops: Int32Array, pixels: (id: number, w: number, h: number) => Pixels): void {
    for (let i = 0; i + OP_STRIDE <= ops.length; i += OP_STRIDE) {
      const kind = ops[i];
      const id = ops[i + 1] ?? 0;
      if (kind === OP_MAKE || kind === OP_REFILL) this.fill(id, ops[i + 2] ?? 0, ops[i + 3] ?? 0, ops[i + 4] ?? 0, pixels);
      else if (kind === OP_FREE) this.free(id);
    }
  }

  /** Let go of every surface (the game shuts down). */
  clear(): void {
    for (const id of [...this.held.keys()]) this.free(id);
    for (const list of this.pool.values()) for (const t of list) t.texture.destroy();
    this.pool.clear();
    this.pooled = 0;
  }

  private fill(id: number, w: number, h: number, sides: number, pixels: (id: number, w: number, h: number) => Pixels): void {
    const old = this.held.get(id);
    if (w === 0 || h === 0 || sides === 0) {
      if (old?.pooled) this.give(old.pooled);
      this.held.set(id, { pooled: null, w, h, sides });
      return;
    }
    const bucket = bucketOf(w, h);
    let pooled = old?.pooled ?? null;
    if (pooled && !fits(pooled.bucket, w, h)) {
      this.give(pooled);
      pooled = null;
    }
    pooled ??= this.take(bucket);
    const { color, light } = pixels(id, w, h);
    pooled.texture.fill(color, light, w, h);
    this.held.set(id, { pooled, w, h, sides });
  }

  private free(id: number): void {
    const h = this.held.get(id);
    if (!h) return;
    this.held.delete(id);
    if (h.pooled) this.give(h.pooled);
  }

  private take(bucket: string): Pooled {
    const free = this.pool.get(bucket)?.pop();
    if (free) {
      this.pooled -= 1;
      return free;
    }
    const [bw = TEXTURE_STEP, bh = TEXTURE_STEP] = bucket.split("x").map(Number);
    return { texture: this.textures(bw * 2, bh), bucket };
  }

  private give(t: Pooled): void {
    if (this.pooled >= TEXTURE_POOL_MAX) {
      t.texture.destroy();
      return;
    }
    const list = this.pool.get(t.bucket);
    if (list) list.push(t);
    else this.pool.set(t.bucket, [t]);
    this.pooled += 1;
  }
}

function round(n: number): number {
  return Math.max(1, Math.ceil(n / TEXTURE_STEP)) * TEXTURE_STEP;
}

function bucketOf(w: number, h: number): string {
  return `${round(w)}x${round(h)}`;
}

function fits(bucket: string, w: number, h: number): boolean {
  return bucket === bucketOf(w, h);
}
