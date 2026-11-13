// Frames effects share: identical effects (one clip key) show one drawn frame, held once for the whole game.
import type { Gfx, GfxTexture } from "./gfx";
import type { Box, Frame, Side } from "./protocol";

export type { Box, Frame, Side };

/** Bytes the shared frames may hold before the least recently shown go: 64 MiB. */
export const DEFAULT_FRAME_BUDGET = 64 * 1024 * 1024;

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
 * One frame shared by every effect showing it: its cropped bytes (layers composite from them) and, once a
 * per-object effect shows it, four textures. Held by the cache while any effect shows it.
 */
export class SharedFrame {
  /** Effects showing it now. */
  private shows = 0;
  private made: { keys: FrameTextures; textures: GfxTexture[] } | null = null;

  /** @internal Made by `FrameCache.put`. */
  constructor(
    readonly id: string,
    readonly frame: Frame,
    readonly bytes: number,
    private readonly cache: FrameCache,
    private readonly keyPrefix: string,
  ) {}

  /** Its textures, made and uploaded with `gfx` the first time any effect asks. */
  textures(gfx: Gfx): FrameTextures {
    if (this.made) return this.made.keys;
    const textures: GfxTexture[] = [];
    const side = (name: string, s: Side): SideTextures | null => {
      if (s.box.w === 0 || s.box.h === 0) return null;
      const upload = (layer: string, bytes: ArrayBuffer): string => {
        const key = `${this.keyPrefix}${name}-${layer}`;
        const t = gfx.texture(key, s.box.w, s.box.h);
        textures.push(t);
        t.upload(new Uint8ClampedArray(bytes));
        return key;
      };
      return { color: upload("color", s.color), light: upload("light", s.light), box: s.box };
    };
    const keys = { back: side("back", this.frame.back), front: side("front", this.frame.front) };
    this.made = { keys, textures };
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

  /** @internal Free its textures. */
  free(): void {
    for (const t of this.made?.textures ?? []) t.destroy();
    this.made = null;
  }
}

/**
 * The shared frames of the game by id, under a byte budget. `trim()` (once per game step) drops the least
 * recently shown frames until the cache fits, never one an effect shows or one shown this step, and returns
 * their ids so the workers draw them again when an effect reaches them.
 */
export class FrameCache {
  private readonly held = new Map<string, { frame: SharedFrame; shown: number }>();
  private step = 0;
  private nextKey = 0;
  private total = 0;

  constructor(private budget = DEFAULT_FRAME_BUDGET) {}

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
    const shared = new SharedFrame(id, frame, bytes, this, `pixfx:frame${this.nextKey++}:`);
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
    const dropped: string[] = [];
    // a Map iterates in insertion order and `touch` re-inserts: the first entries are the least recently shown
    for (const [id, h] of this.held) {
      if (this.total <= this.budget) break;
      if (h.frame.shown() || h.shown === this.step) continue;
      this.held.delete(id);
      this.total -= h.frame.bytes;
      h.frame.free();
      dropped.push(id);
    }
    this.step += 1;
    return dropped;
  }
}

function sideBytes(s: Side): number {
  return s.color.byteLength + s.light.byteLength;
}
