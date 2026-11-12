import { z } from "zod";
import type { Counts, Layer, Layers } from "./effect";
import { PixfxEffect } from "./effect";
import type { Engine, FxLike } from "./engine";
import { spawnFx } from "./engine";
import type { Gfx, GfxBlend, GfxImage, GfxTexture } from "./gfx";
import { clipRect } from "./layout";
import type { Rect } from "./layout";
import { compositeColor, compositeLight } from "./pixels";
import type { Surface } from "./surface";

const layerConfigSchema = z.object({
  bounds: z.object({
    x: z.number().int(),
    y: z.number().int(),
    width: z.number().int().positive(),
    height: z.number().int().positive(),
  }),
  depths: z.object({ back: z.number(), front: z.number() }).default({ back: 0, front: 1 }),
  scrollFactor: z.number().default(1),
});

/** Whole-pixel region a layer covers, plus the depths and scroll factor of its four images. */
export type LayerConfig = z.input<typeof layerConfigSchema>;

/** Parse an untrusted layer config; throws an Error naming the offending field. */
export function parseLayerConfig(input: unknown): z.output<typeof layerConfigSchema> {
  const r = layerConfigSchema.safeParse(input);
  if (r.success) return r.data;
  const msg = r.error.issues
    .map((i) => `${i.path.length > 0 ? i.path.join(".") : "config"}: ${i.message}`)
    .join("; ");
  throw new Error(`pixfx: invalid layer config (${msg})`);
}

/** One member's share of a layer: remembers what the effect shows; the layer composites it. */
class LayerSurface implements Surface {
  fx: FxLike | null = null;
  topLeft = { x: 0, y: 0 };

  constructor(private readonly layer: PixfxLayer) {}

  /** No-op: the layer owns the depths of its four images. */
  setDepths(): void {}

  show(fx: FxLike, topLeft: { x: number; y: number }): void {
    this.fx = fx;
    this.topLeft = topLeft;
    this.layer.invalidate();
  }

  release(): void {
    this.layer.drop(this);
  }
}

/**
 * Many effects sharing one set of four textures covering a region. Each tick the layer clears the
 * textures and composites every member into them, clipped to the region. Members are
 * `PixfxEffect`s with the usual API; the layer owns their depths and scroll factor.
 */
export class PixfxLayer {
  private readonly bounds: Rect;
  private readonly textures: readonly GfxTexture[];
  private readonly images: readonly GfxImage[];
  private readonly members = new Set<PixfxEffect>();
  private readonly surfaces = new Set<LayerSurface>();
  private readonly layers: Layers;
  private updating = false;
  private dirty = false;
  private destroyed = false;

  /** @internal Created by the plugin's `createLayer()`. */
  constructor(
    cfg: unknown,
    gfx: Gfx,
    private readonly engine: () => Engine,
    keyPrefix: string,
    private readonly counts: Counts,
    private readonly onGone: (layer: PixfxLayer) => void,
  ) {
    const c = parseLayerConfig(cfg);
    this.bounds = { x: c.bounds.x, y: c.bounds.y, w: c.bounds.width, h: c.bounds.height };
    const made: Layer[] = [];
    const make = (name: string, blend: GfxBlend, depth: number): Layer => {
      const key = `${keyPrefix}${name}`;
      const texture = gfx.texture(key, this.bounds.w, this.bounds.h);
      const image = gfx.image(key, blend, c.scrollFactor);
      image.setDepth(depth);
      image.setPosition(this.bounds.x, this.bounds.y);
      made.push({ texture, image });
      return { texture, image };
    };
    const layers: Layers = {
      backColor: make("back-color", "normal", c.depths.back),
      backLight: make("back-light", "add", c.depths.back),
      frontColor: make("front-color", "normal", c.depths.front),
      frontLight: make("front-light", "add", c.depths.front),
    };
    this.layers = layers;
    this.textures = made.map((l) => l.texture);
    this.images = made.map((l) => l.image);
    counts.textures += this.textures.length;
    counts.images += this.images.length;
  }

  /** Play effect `id` with its origin at world (x, y), drawn into this layer. Throws on an unknown id or invalid options. */
  play(id: string, x: number, y: number, opts?: unknown): PixfxEffect {
    if (this.destroyed) throw new Error("pixfx: layer is destroyed");
    const { fx } = spawnFx(this.engine(), id, opts);
    this.counts.engines += 1;
    const surface = new LayerSurface(this);
    this.surfaces.add(surface);
    try {
      const effect = new PixfxEffect(fx, x, y, surface, { back: 0, front: 1 }, this.counts, (e) => {
        this.members.delete(e);
      });
      this.members.add(effect);
      return effect;
    } catch (err) {
      this.surfaces.delete(surface);
      fx.free();
      this.counts.engines -= 1;
      throw err;
    }
  }

  /** Destroy every member and free the layer's textures and images. Does nothing if already destroyed. */
  destroy(): void {
    if (this.destroyed) return;
    for (const e of [...this.members]) e.destroy();
    this.destroyed = true;
    for (const i of this.images) i.destroy();
    for (const t of this.textures) t.destroy();
    this.counts.images -= this.images.length;
    this.counts.textures -= this.textures.length;
    this.onGone(this);
  }

  /** @internal Advance every member by `dt` seconds, then redraw once if anything changed. */
  update(dt: number): void {
    if (dt <= 0 || this.destroyed) return;
    this.updating = true;
    try {
      for (const e of [...this.members]) e.step(dt);
    } finally {
      this.updating = false;
    }
    if (this.dirty) this.flush();
  }

  /** @internal A member changed what it shows: redraw now, or at the end of the running update. */
  invalidate(): void {
    this.dirty = true;
    if (!this.updating) this.flush();
  }

  /** @internal A member is gone. */
  drop(surface: LayerSurface): void {
    this.surfaces.delete(surface);
    if (!this.destroyed) this.invalidate();
  }

  private flush(): void {
    this.dirty = false;
    const { backColor, backLight, frontColor, frontLight } = this.layers;
    for (const l of [backColor, backLight, frontColor, frontLight]) l.texture.pixels.fill(0);
    for (const s of this.surfaces) {
      if (!s.fx) continue;
      this.compose(s.fx, s.topLeft, "back", backColor, backLight);
      this.compose(s.fx, s.topLeft, "front", frontColor, frontLight);
    }
    for (const t of this.textures) t.upload();
  }

  private compose(fx: FxLike, at: { x: number; y: number }, which: "back" | "front", color: Layer, light: Layer): void {
    const w = fx.width();
    const clip = clipRect(this.bounds, { x: at.x, y: at.y, w, h: fx.height() });
    if (!clip) return;
    fx.draw(which);
    const blit = {
      srcX: clip.x - at.x,
      srcY: clip.y - at.y,
      dstX: clip.x - this.bounds.x,
      dstY: clip.y - this.bounds.y,
      w: clip.w,
      h: clip.h,
      srcStride: w,
      dstStride: this.bounds.w,
    };
    compositeColor(fx.color(), color.texture.pixels, blit);
    compositeLight(fx.light(), light.texture.pixels, blit);
  }
}
