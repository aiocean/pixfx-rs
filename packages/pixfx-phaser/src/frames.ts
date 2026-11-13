// Frames effects share: identical effects (one clip key) show one drawn frame, held once for the whole game.
import type { Gfx, GfxTexture } from "./gfx";
import type { Box, Frame, Side } from "./protocol";

export type { Box, Frame, Side };

/** Bytes the shared frames may hold before the least recently shown go: 64 MiB. */
export const DEFAULT_FRAME_BUDGET = 64 * 1024 * 1024;

/**
 * Shared frames' textures come in sizes rounded up to this many pixels, so a texture a frame gave back fits later
 * frames of about its size; an image shows the frame's box of it.
 */
export const TEXTURE_STEP = 32;

/**
 * Bytes the textures of frames no effect shows may hold (as RGBA at their bucket size) before the least recently
 * shown give theirs back: 32 MiB. Under it a looping clip replays with no upload; past it textures are reused.
 */
export const DEFAULT_TEXTURE_BUDGET = 32 * 1024 * 1024;

/** Textures no frame shows, kept for reuse: at most this many; more are destroyed. */
export const TEXTURE_POOL_MAX = 256;

/** A texture of the cache's pool and its key. */
interface Pooled {
  key: string;
  texture: GfxTexture;
  /** Its size bucket, `"{w}x{h}"`. */
  bucket: string;
  /** Bytes it holds as RGBA. */
  bytes: number;
}

/** The clip of a shared frame id (`"{key}:{index}"`): every effect of one clip lives in one worker. */
export function clipOf(id: string): string {
  const i = id.lastIndexOf(":");
  return i < 0 ? id : id.slice(0, i);
}

/** One side of a shared frame as textures: the colour's key, the light's key, and where they sit on the canvas. */
export interface SideTextures {
  color: string;
  light: string;
  box: Box;
}

/** A frame as textures; a side that shows nothing has none. */
export interface FrameTextures {
  back: SideTextures | null;
  front: SideTextures | null;
}

/**
 * One frame shared by every effect showing it: its cropped bytes (layers composite from them) and, while a
 * per-object effect shows it, four textures from the cache's pool. Held by the cache while any effect shows it.
 */
export class SharedFrame {
  /** Effects showing it now. */
  private shows = 0;
  private made: { keys: FrameTextures; textures: Pooled[] } | null = null;

  /** @internal Made by `FrameCache.put`. */
  constructor(
    readonly id: string,
    readonly frame: Frame,
    readonly bytes: number,
    private readonly cache: FrameCache,
  ) {}

  /** Its textures, taken from the pool (or made with `gfx`) and uploaded the first time an effect asks. */
  textures(gfx: Gfx): FrameTextures {
    if (this.made) return this.made.keys;
    const textures: Pooled[] = [];
    const side = (s: Side): SideTextures | null => {
      if (s.box.w === 0 || s.box.h === 0) return null;
      const upload = (bytes: ArrayBuffer): string => {
        const t = this.cache.takeTexture(gfx, s.box.w, s.box.h);
        textures.push(t);
        t.texture.upload(new Uint8ClampedArray(bytes), s.box.w, s.box.h);
        return t.key;
      };
      return { color: upload(s.color), light: upload(s.light), box: s.box };
    };
    const keys = { back: side(this.frame.back), front: side(this.frame.front) };
    this.made = { keys, textures };
    this.cache.textured(this);
    return keys;
  }

  /** An effect shows it from now on: it is not dropped until every effect showing it lets go. */
  retain(): void {
    this.shows += 1;
    this.cache.touch(this);
  }

  /** An effect no longer shows it. */
  release(): void {
    this.shows = Math.max(0, this.shows - 1);
    this.cache.touch(this);
  }

  /** @internal */
  shown(): boolean {
    return this.shows > 0;
  }

  /** @internal Bytes its textures hold, 0 when it has none. */
  textureBytes(): number {
    return this.made?.textures.reduce((n, t) => n + t.bytes, 0) ?? 0;
  }

  /** @internal Give its textures back to the pool; its bytes stay, so it is uploaded again if shown again. */
  free(): void {
    for (const t of this.made?.textures ?? []) this.cache.giveTexture(t);
    this.made = null;
  }
}

/**
 * The shared frames of the game by id, under a byte budget. `trim()` (once per game step) first gives back, past
 * the texture budget, the textures of the least recently shown frames off screen, then drops the least recently shown frames until the cache fits, never
 * one an effect shows or one shown this step, and returns their ids so the workers draw them again when an effect
 * reaches them. Textures are reused from a pool of free ones (by size, rounded up to `TEXTURE_STEP`) before any is
 * made, so effects that share nothing stop making textures once the texture budget is full.
 */
export class FrameCache {
  private readonly held = new Map<string, { frame: SharedFrame; shown: number }>();
  /** Frames holding textures. */
  private readonly withTextures = new Set<SharedFrame>();
  /** Free textures by size bucket. */
  private readonly pool = new Map<string, Pooled[]>();
  private pooled = 0;
  /** Bytes the textures of frames hold (not the pool's). */
  private textureTotal = 0;
  private step = 0;
  private nextKey = 0;
  private total = 0;

  constructor(
    private budget = DEFAULT_FRAME_BUDGET,
    private readonly textureBudget = DEFAULT_TEXTURE_BUDGET,
  ) {}

  /** Bytes the frames hold. */
  get bytes(): number {
    return this.total;
  }

  get size(): number {
    return this.held.size;
  }

  has(id: string): boolean {
    return this.held.has(id);
  }

  get(id: string): SharedFrame | undefined {
    return this.held.get(id)?.frame;
  }

  /** Hold `frame` as `id`; an id already held keeps the frame it has (equal ids are equal bytes). */
  put(id: string, frame: Frame): SharedFrame {
    const old = this.held.get(id);
    if (old) return old.frame;
    const bytes = sideBytes(frame.back) + sideBytes(frame.front);
    const shared = new SharedFrame(id, frame, bytes, this);
    this.held.set(id, { frame: shared, shown: this.step });
    this.total += bytes;
    return shared;
  }

  /** Change the budget; takes effect at the next `trim()`. */
  setBudget(bytes: number): void {
    this.budget = bytes;
  }

  /** @internal The frame was shown or let go now: it moves to the back of the drop order. */
  touch(frame: SharedFrame): void {
    const h = this.held.get(frame.id);
    if (!h || h.frame !== frame) return;
    this.held.delete(frame.id);
    this.held.set(frame.id, { frame, shown: this.step });
  }

  /** Drop the least recently shown frames until the cache fits its budget, then start a new step. Returns the ids dropped. */
  trim(): string[] {
    // past the texture budget, frames off screen give their textures back, least recently shown first
    for (const [, h] of this.held) {
      if (this.textureTotal <= this.textureBudget) break;
      if (!this.withTextures.has(h.frame) || h.frame.shown() || h.shown === this.step) continue;
      this.untexture(h.frame);
    }
    const dropped: string[] = [];
    // a Map iterates in insertion order and `touch` re-inserts: the first entries are the least recently shown
    for (const [id, h] of this.held) {
      if (this.total <= this.budget) break;
      if (h.frame.shown() || h.shown === this.step) continue;
      this.held.delete(id);
      this.total -= h.frame.bytes;
      this.untexture(h.frame);
      dropped.push(id);
    }
    this.step += 1;
    return dropped;
  }

  /** @internal A texture of at least `w × h` pixels: a free one of that size bucket, or a new one made with `gfx`. */
  takeTexture(gfx: Gfx, w: number, h: number): Pooled {
    const bw = Math.ceil(w / TEXTURE_STEP) * TEXTURE_STEP;
    const bh = Math.ceil(h / TEXTURE_STEP) * TEXTURE_STEP;
    const bucket = `${bw}x${bh}`;
    const free = this.pool.get(bucket)?.pop();
    if (free) {
      this.pooled -= 1;
      return free;
    }
    const key = `pixfx:frame${this.nextKey++}`;
    return { key, texture: gfx.texture(key, bw, bh), bucket, bytes: bw * bh * 4 };
  }

  /** @internal A frame took textures. */
  textured(frame: SharedFrame): void {
    this.withTextures.add(frame);
    this.textureTotal += frame.textureBytes();
  }

  /** A frame gives its textures back to the pool. */
  private untexture(frame: SharedFrame): void {
    if (!this.withTextures.delete(frame)) return;
    this.textureTotal -= frame.textureBytes();
    frame.free();
  }

  /** @internal A texture no frame shows: kept for reuse, or destroyed once the pool is full. */
  giveTexture(t: Pooled): void {
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

function sideBytes(s: Side): number {
  return s.color.byteLength + s.light.byteLength;
}
