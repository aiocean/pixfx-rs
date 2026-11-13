import { stepSeconds } from "./clock";
import type { CueSink, Counts } from "./effect";
import { PixfxEffect } from "./effect";
import type { PixfxFeel } from "./feel";
import type { Backend } from "./backend";
import type { Gfx, GfxBlend, GfxImage } from "./gfx";
import type { EffectImages } from "./surface";
import { ImageSurface } from "./surface";
import { PixfxLayer } from "./layer";
import type { LayerConfig } from "./layer";

const DEFAULT_DEPTHS = { back: 0, front: 1 };

/** The per-effect objects of one scene: plays, ticks on scene time, and frees them. No Phaser calls. */
export class EffectManager {
  private readonly live = new Set<PixfxEffect>();
  private readonly layers = new Set<PixfxLayer>();
  private readonly counts: Counts = { engines: 0, textures: 0, images: 0 };
  private nextId = 0;

  /**
   * @param gfx creates textures and images.
   * @param backend returns where effects run (throws before `loadPixfx()` resolves).
   * @param keyPrefix makes texture keys unique in the game-wide texture manager.
   * @param feel the scene's answer to the cues its effects fire; it also says how much time they play.
   */
  constructor(
    private readonly gfx: Gfx,
    private readonly backend: () => Backend,
    private readonly keyPrefix: string,
    private readonly feel: PixfxFeel,
  ) {
    this.sink = (cues, effect) => this.feel.take(cues, effect);
  }

  private readonly sink: CueSink;

  /**
   * Play effect `id` with its origin at world (x, y): four images showing the frames it shares with every
   * identical effect, and its own textures once it fades. Throws on an unknown id or invalid options.
   */
  play(id: string, x: number, y: number, opts?: unknown): PixfxEffect {
    const { instance, play: p } = this.backend().spawn(id, opts);
    this.counts.engines += 1;
    const n = this.nextId++;
    const made: GfxImage[] = [];
    const image = (blend: GfxBlend): GfxImage => {
      const i = this.gfx.image(null, blend, p.scrollFactor);
      made.push(i);
      this.counts.images += 1;
      return i;
    };
    try {
      const images: EffectImages = {
        back: { color: image("normal"), light: image("add") },
        front: { color: image("normal"), light: image("add") },
      };
      const effect = new PixfxEffect(
        instance,
        x,
        y,
        new ImageSurface(images, this.gfx, `${this.keyPrefix}${n}-`, instance.shape, this.counts),
        p.depth ?? DEFAULT_DEPTHS,
        this.counts,
        (e) => this.live.delete(e),
        this.sink,
      );
      this.live.add(effect);
      return effect;
    } catch (err) {
      for (const i of made) i.destroy();
      this.counts.images -= made.length;
      instance.free();
      this.counts.engines -= 1;
      throw err;
    }
  }

  /** A layer: effects played into it share four textures covering `cfg.bounds`. Throws on an invalid config. */
  createLayer(cfg: LayerConfig): PixfxLayer {
    const layer = new PixfxLayer(
      cfg,
      this.gfx,
      this.backend,
      `${this.keyPrefix}L${this.nextId++}-`,
      this.counts,
      (l) => this.layers.delete(l),
      this.sink,
    );
    this.layers.add(layer);
    return layer;
  }

  /**
   * One scene update: `deltaMs` real milliseconds pass through the feel (nothing plays in a hit-stop, a fraction in
   * slow motion), then every effect advances by what played × `timeScale`. Returns those seconds of play.
   */
  update(deltaMs: number, timeScale: number): number {
    const dt = stepSeconds(this.feel.advance(deltaMs), timeScale);
    if (dt === 0) return 0;
    for (const e of [...this.live]) e.step(dt);
    for (const l of [...this.layers]) l.update(dt);
    return dt;
  }

  /** Once per frame, before render: redraw every live layer that changed, so many plays or moves cost one redraw. */
  flushLayers(): void {
    for (const l of [...this.layers]) l.flushIfDirty();
  }

  /** Destroy every effect and layer (scene shutdown or destroy). */
  destroyAll(): void {
    for (const l of [...this.layers]) l.destroy();
    for (const e of [...this.live]) e.destroy();
  }

  stats(): Counts {
    return { ...this.counts };
  }
}
