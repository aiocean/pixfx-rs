import type { EffectMeta, Engine, EngineOptions, FxLike } from "../src/engine";

/** Stopping lets an effect fade for this long before it is done. */
const FADE_SECONDS = 0.5;

export class FakeFx implements FxLike {
  elapsed = 0;
  stopped = false;
  freed = false;
  /** Number of `draw` calls; every RGBA byte of a frame is this count (mod 256), so a test can tell frames apart. */
  draws = 0;
  constructor(
    private readonly owner: FakeEngine,
    private duration: number,
    private readonly size: readonly [number, number],
    private readonly origin: readonly [number, number],
  ) {}
  update(dt: number): void {
    this.elapsed += dt;
  }
  stop(): void {
    this.stopped = true;
    this.duration = Math.min(this.duration, this.elapsed + FADE_SECONDS);
  }
  done(): boolean {
    return this.elapsed >= this.duration;
  }
  t(): number {
    return this.elapsed;
  }
  width(): number {
    return this.size[0];
  }
  height(): number {
    return this.size[1];
  }
  originX(): number {
    return this.origin[0];
  }
  originY(): number {
    return this.origin[1];
  }
  draw(_layer: "back" | "front"): void {
    this.draws += 1;
  }
  colorRgba(): Uint8ClampedArray {
    return new Uint8ClampedArray(this.size[0] * this.size[1] * 4).fill(this.draws & 0xff);
  }
  lightRgba(): Uint8ClampedArray {
    return new Uint8ClampedArray(this.size[0] * this.size[1] * 4).fill(this.draws & 0xff);
  }
  free(): void {
    if (this.freed) throw new Error("double free");
    this.freed = true;
    this.owner.live -= 1;
  }
}

/** Deterministic Engine: one known effect, "fake/spark", lasting 1 s. */
export class FakeEngine implements Engine {
  live = 0;
  readonly created: FakeFx[] = [];
  readonly metas: Record<string, EffectMeta> = {
    "fake/spark": {
      id: "fake/spark",
      name: "Spark",
      size: [8, 6],
      origin: [4, 5],
      duration: 1,
      loop: false,
      fps: 30,
      seed: 1,
      glowMargin: 4,
    },
  };
  effects(): readonly string[] {
    return Object.keys(this.metas);
  }
  meta(id: string): EffectMeta {
    const m = this.metas[id];
    if (!m) throw new Error(`unknown effect: ${id}`);
    return m;
  }
  create(id: string, _opts: EngineOptions): FxLike {
    const m = this.meta(id);
    const fx = new FakeFx(this, m.duration, m.size, m.origin);
    this.live += 1;
    this.created.push(fx);
    return fx;
  }
}

/** Every frame fills its colour with one ABGR word and its light with one xBGR word. */
export class PaintedFx extends FakeFx {
  colorWord = 0;
  lightWord = 0;
  override colorRgba(): Uint8ClampedArray {
    const w = this.colorWord;
    return fill(this.width() * this.height(), [w & 0xff, (w >>> 8) & 0xff, (w >>> 16) & 0xff, w >>> 24]);
  }
  override lightRgba(): Uint8ClampedArray {
    const w = this.lightWord & 0xffffff;
    return fill(this.width() * this.height(), w === 0 ? [0, 0, 0, 0] : [w & 0xff, (w >>> 8) & 0xff, (w >>> 16) & 0xff, 255]);
  }
}

function fill(pixels: number, rgba: readonly [number, number, number, number]): Uint8ClampedArray {
  const out = new Uint8ClampedArray(pixels * 4);
  for (let i = 0; i < out.length; i += 4) out.set(rgba, i);
  return out;
}

/** Effects it creates paint `paint` (read at creation). */
export class PaintedEngine extends FakeEngine {
  paint = { color: 0, light: 0 };
  override create(id: string, _opts: EngineOptions): FxLike {
    const m = this.meta(id);
    const fx = new PaintedFx(this, m.duration, m.size, m.origin);
    fx.colorWord = this.paint.color;
    fx.lightWord = this.paint.light;
    this.live += 1;
    this.created.push(fx);
    return fx;
  }
}
