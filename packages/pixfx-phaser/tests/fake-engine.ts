import type { EffectMeta, Engine, EngineOptions, FxLike } from "../src/engine";

/** Stopping lets an effect fade for this long before it is done. */
const FADE_SECONDS = 0.5;

export class FakeFx implements FxLike {
  elapsed = 0;
  stopped = false;
  freed = false;
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
  draw(_layer: "back" | "front"): void {}
  color(): Uint32Array {
    return new Uint32Array(this.size[0] * this.size[1]);
  }
  light(): Uint32Array {
    return new Uint32Array(this.size[0] * this.size[1]);
  }
  colorRgba(): Uint8ClampedArray {
    return new Uint8ClampedArray(this.size[0] * this.size[1] * 4);
  }
  lightRgba(): Uint8ClampedArray {
    return new Uint8ClampedArray(this.size[0] * this.size[1] * 4);
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
