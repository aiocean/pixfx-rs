import { z } from "zod";
import type { Backend, FrameBuffers, Shape } from "./backend";
import type { CueSink, Counts, Layer, Layers } from "./effect";
import { PixfxEffect } from "./effect";
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

/** One member's share of a layer: keeps the member's latest frame and place; the layer composites them. */
class LayerSurface implements Surface {
  frame: FrameBuffers | null = null;
  topLeft = { x: 0, y: 0 };

  constructor(
    private readonly layer: PixfxLayer,
    readonly shape: Shape,
  ) {}

  /** No-op: the layer owns the depths of its four images. */
  setDepths(): void {}

  /** Keep the new frame for the next composite; the previous one is no longer read. */
  show(frame: FrameBuffers): FrameBuffers | null {
    const old = this.frame;
    this.frame = frame;
    this.layer.invalidate();
    return old;
  }

  /** Redraw the layer when the member moved; a still member costs nothing. */
  moveTo(topLeft: { x: number; y: number }): void {
    if (topLeft.x === this.topLeft.x && topLeft.y === this.topLeft.y) return;
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
  /** What the four textures show, composited here and uploaded on each redraw. */
  private readonly pixels: Record<keyof Layers, Uint8ClampedArray<ArrayBuffer>>;
  private readonly images: readonly GfxImage[];
  private readonly members = new Set<PixfxEffect>();
  private readonly surfaces = new Set<LayerSurface>();
  private readonly layers: Layers;
  private dirty = false;
  private destroyed = false;

  /** @internal Created by the plugin's `createLayer()`. */
  constructor(
    cfg: unknown,
    gfx: Gfx,
    private readonly backend: () => Backend,
    keyPrefix: string,
    private readonly counts: Counts,
    private readonly onGone: (layer: PixfxLayer) => void,
    private readonly sink: CueSink,
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
    const bytes = (): Uint8ClampedArray<ArrayBuffer> => new Uint8ClampedArray(this.bounds.w * this.bounds.h * 4);
    this.pixels = { backColor: bytes(), backLight: bytes(), frontColor: bytes(), frontLight: bytes() };
    this.images = made.map((l) => l.image);
    counts.textures += this.textures.length;
    counts.images += this.images.length;
  }

  /** Play effect `id` with its origin at world (x, y), drawn into this layer. Throws on an unknown id or invalid options. */
  play(id: string, x: number, y: number, opts?: unknown): PixfxEffect {
    if (this.destroyed) throw new Error("pixfx: layer is destroyed");
    const { instance } = this.backend().spawn(id, opts);
    this.counts.engines += 1;
    const surface = new LayerSurface(this, instance.shape);
    this.surfaces.add(surface);
    try {
      const effect = new PixfxEffect(
        instance,
        x,
        y,
        surface,
        { back: 0, front: 1 },
        this.counts,
        (e) => {
          this.members.delete(e);
        },
        this.sink,
      );
      this.members.add(effect);
      return effect;
    } catch (err) {
      this.surfaces.delete(surface);
      instance.free();
      this.counts.engines -= 1;
      throw err;
    }
  }

  /** Destroy every member and free the layer's textures and images. Does nothing if already destroyed. */
  destroy(): void {
    if (this.destroyed) return;
    this.destroyed = true;
    for (const e of [...this.members]) e.destroy();
    for (const i of this.images) i.destroy();
    for (const t of this.textures) t.destroy();
    this.counts.images -= this.images.length;
    this.counts.textures -= this.textures.length;
    this.onGone(this);
  }

  /** @internal Advance every member by `dt` seconds, then redraw once if anything changed. */
  update(dt: number): void {
    if (dt <= 0 || this.destroyed) return;
    for (const e of [...this.members]) e.step(dt);
    this.flushIfDirty();
  }

  /** @internal A member changed what it shows: mark for redraw at the next `flushIfDirty()` (the end of `update`, or the plugin's per-frame flush). */
  invalidate(): void {
    this.dirty = true;
  }

  /** @internal Redraw once if anything changed since the last redraw. Never touches a destroyed layer. */
  flushIfDirty(): void {
    if (this.dirty && !this.destroyed) this.flush();
  }

  /** @internal A member is gone. */
  drop(surface: LayerSurface): void {
    this.surfaces.delete(surface);
    if (!this.destroyed) this.invalidate();
  }

  private flush(): void {
    this.dirty = false;
    const p = this.pixels;
    for (const px of [p.backColor, p.backLight, p.frontColor, p.frontLight]) px.fill(0);
    for (const s of this.surfaces) {
      if (!s.frame) continue;
      const [backColor, backLight, frontColor, frontLight] = s.frame;
      this.compose(s.shape, s.topLeft, backColor, backLight, p.backColor, p.backLight);
      this.compose(s.shape, s.topLeft, frontColor, frontLight, p.frontColor, p.frontLight);
    }
    const l = this.layers;
    l.backColor.texture.upload(p.backColor);
    l.backLight.texture.upload(p.backLight);
    l.frontColor.texture.upload(p.frontColor);
    l.frontLight.texture.upload(p.frontLight);
  }

  /**
   * Composite one side of a member: its colour RGBA read as ABGR words, its light RGBA as light words (alpha 255
   * where lit, which `compositeLight` masks off).
   */
  private compose(
    shape: Shape,
    at: { x: number; y: number },
    color: ArrayBuffer,
    light: ArrayBuffer,
    colorOut: Uint8ClampedArray,
    lightOut: Uint8ClampedArray,
  ): void {
    const clip = clipRect(this.bounds, { x: at.x, y: at.y, w: shape.width, h: shape.height });
    if (!clip) return;
    const blit = {
      srcX: clip.x - at.x,
      srcY: clip.y - at.y,
      dstX: clip.x - this.bounds.x,
      dstY: clip.y - this.bounds.y,
      w: clip.w,
      h: clip.h,
      srcStride: shape.width,
      dstStride: this.bounds.w,
    };
    compositeColor(new Uint32Array(color), colorOut, blit);
    compositeLight(new Uint32Array(light), lightOut, blit);
  }
}
