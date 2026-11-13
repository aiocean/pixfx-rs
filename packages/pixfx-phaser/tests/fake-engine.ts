import type { BlinkLike, EffectMeta, Engine, EngineOptions, FeelLike, FxLike, SquashLike } from "../src/engine";
import type { Box, TimedCue } from "../src/protocol";

/** Stopping lets an effect fade for this long before it is done. */
const FADE_SECONDS = 0.5;

/**
 * A one-shot of `duration` seconds at `meta.fps`. Its shown frame is `"{clip}:{k}"`, `clip` its id and options;
 * every RGBA byte a draw writes is `2k + 1` (back) or `2k + 2` (front), mod 256, so equal frames are equal bytes
 * and a test can tell frames apart. While fading, a draw writes `128 + 2c + 1` or `+ 2` for clock frame `c`.
 */
export class FakeFx implements FxLike {
  elapsed = 0;
  stopped = false;
  freed = false;
  /** Number of `draw` calls. */
  draws = 0;
  /** Cues it fires once its time passes them (`t` in seconds of playback). */
  schedule: TimedCue[] = [];
  /** The lit box every draw reports; the whole canvas when null. */
  box: Box | null = null;
  /** The byte the last draw wrote. */
  protected painted = 0;
  private fired: TimedCue[] = [];
  private readonly last: number;
  constructor(
    private readonly owner: FakeEngine,
    private duration: number,
    private readonly size: readonly [number, number],
    private readonly origin: readonly [number, number],
    private readonly fps = 30,
    private readonly clip = "fake",
  ) {
    this.last = Math.round(duration * fps);
  }
  clockFrame(): number {
    return Math.floor(this.elapsed * this.fps + 1e-9);
  }
  shown(): string | undefined {
    return this.stopped ? undefined : `${this.clip}:${Math.min(this.clockFrame(), this.last)}`;
  }
  litBox(): Box {
    return this.box ?? { x: 0, y: 0, w: this.size[0], h: this.size[1] };
  }
  update(dt: number): void {
    const from = this.elapsed;
    this.elapsed += dt;
    this.fired.push(...this.schedule.filter((c) => c.t > from && c.t <= this.elapsed));
  }
  takeCues(): TimedCue[] {
    const out = this.fired;
    this.fired = [];
    return out;
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
  draw(layer: "back" | "front"): void {
    this.draws += 1;
    const side = layer === "back" ? 1 : 2;
    this.painted = (this.stopped ? 128 + 2 * this.clockFrame() + side : 2 * Math.min(this.clockFrame(), this.last) + side) & 0xff;
  }
  colorRgba(box: Box): Uint8ClampedArray {
    return new Uint8ClampedArray(box.w * box.h * 4).fill(this.painted);
  }
  lightRgba(box: Box): Uint8ClampedArray {
    return new Uint8ClampedArray(box.w * box.h * 4).fill(this.painted);
  }
  free(): void {
    if (this.freed) throw new Error("double free");
    this.freed = true;
    this.owner.live -= 1;
  }
}

/**
 * A feel that keeps only what the glue needs: a hit-stop holds play, trauma shakes by one pixel and tilts by 0.01
 * radians, flash as given.
 */
export class FakeFeel implements FeelLike {
  taken: TimedCue[] = [];
  freed = false;
  private stop = 0;
  private shaken = 0;
  private lifted = 0;
  take(cues: readonly TimedCue[]): void {
    this.taken.push(...cues);
    for (const c of cues) {
      if (c.cue.type === "hitstop") this.freeze(c.cue.dur);
      if (c.cue.type === "shake") this.addTrauma(c.cue.trauma);
      if (c.cue.type === "flash") this.lifted = Math.max(this.lifted, c.cue.k);
    }
  }
  addTrauma(k: number): void {
    this.shaken = Math.min(1, this.shaken + k);
  }
  freeze(secs: number): void {
    this.stop = Math.max(this.stop, secs);
  }
  slow(): void {}
  advance(dt: number): number {
    if (this.stop > 1e-9) {
      this.stop = Math.max(0, this.stop - dt);
      return 0;
    }
    return dt;
  }
  shake(strength: number): [number, number] {
    const d = this.shaken > 0 && strength > 0 ? 1 : 0;
    return [d, d === 0 ? 0 : -d];
  }
  roll(strength: number): number {
    return this.shaken > 0 && strength > 0 ? 0.01 : 0;
  }
  trauma(): number {
    return this.shaken;
  }
  frozen(): boolean {
    return this.stop > 1e-9;
  }
  flash(): number {
    return this.lifted;
  }
  free(): void {
    this.freed = true;
  }
}

/** Stretches 1.5 along its faster axis at any speed, squashes 0.5 tall until 0.1 s after a landing. */
export class FakeSquash implements SquashLike {
  freed = false;
  private since: number | null = null;
  land(): void {
    this.since = 0;
  }
  advance(dt: number): void {
    if (this.since !== null) this.since = this.since + dt >= 0.1 ? null : this.since + dt;
  }
  scale(vx: number, vy: number): [number, number] {
    if (this.since !== null) return [2, 0.5];
    if (vx === 0 && vy === 0) return [1, 1];
    return Math.abs(vx) > Math.abs(vy) ? [1.5, 1 / 1.5] : [1 / 1.5, 1.5];
  }
  free(): void {
    this.freed = true;
  }
}

/** Faint (0.5) for as long as it blinks. */
export class FakeBlink implements BlinkLike {
  freed = false;
  private left = 0;
  start(secs: number): void {
    this.left = secs;
  }
  advance(dt: number): void {
    this.left = Math.max(0, this.left - dt);
  }
  active(): boolean {
    return this.left > 0;
  }
  alpha(): number {
    return this.left > 0 ? 0.5 : 1;
  }
  free(): void {
    this.freed = true;
  }
}

/** Deterministic Engine: one known effect, "fake/spark", lasting 1 s. */
export class FakeEngine implements Engine {
  live = 0;
  readonly created: FakeFx[] = [];
  readonly feels: FakeFeel[] = [];
  /** Cues every effect it creates from now on fires. */
  schedule: TimedCue[] = [];
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
  /** The lit box every effect it creates from now on reports (null: the whole canvas). */
  box: Box | null = null;
  create(id: string, opts: EngineOptions): FxLike {
    const m = this.meta(id);
    const fx = new FakeFx(this, m.duration, m.size, m.origin, m.fps, clipOf(id, opts));
    fx.schedule = this.schedule;
    fx.box = this.box;
    this.live += 1;
    this.created.push(fx);
    return fx;
  }
  /** `draw` calls of every effect it created. */
  draws(): number {
    return this.created.reduce((n, fx) => n + fx.draws, 0);
  }
  feel(): FeelLike {
    const f = new FakeFeel();
    this.feels.push(f);
    return f;
  }
  squash(): SquashLike {
    return new FakeSquash();
  }
  blink(): BlinkLike {
    return new FakeBlink();
  }
}

/** Every frame fills its colour with one ABGR word and its light with one xBGR word. */
export class PaintedFx extends FakeFx {
  colorWord = 0;
  lightWord = 0;
  override colorRgba(box: Box): Uint8ClampedArray {
    const w = this.colorWord;
    return fill(box.w * box.h, [w & 0xff, (w >>> 8) & 0xff, (w >>> 16) & 0xff, w >>> 24]);
  }
  override lightRgba(box: Box): Uint8ClampedArray {
    const w = this.lightWord & 0xffffff;
    return fill(box.w * box.h, w === 0 ? [0, 0, 0, 0] : [w & 0xff, (w >>> 8) & 0xff, (w >>> 16) & 0xff, 255]);
  }
}

function fill(pixels: number, rgba: readonly [number, number, number, number]): Uint8ClampedArray {
  const out = new Uint8ClampedArray(pixels * 4);
  for (let i = 0; i < out.length; i += 4) out.set(rgba, i);
  return out;
}

/** The clip of effect `id` with `opts`: equal for equal ids and options, as the engine's key is. */
function clipOf(id: string, opts: EngineOptions): string {
  return `${id}~${JSON.stringify(opts)}`;
}

/** Effects it creates paint `paint` (read at creation; part of their clip, as an option would be). */
export class PaintedEngine extends FakeEngine {
  paint = { color: 0, light: 0 };
  override create(id: string, opts: EngineOptions): FxLike {
    const m = this.meta(id);
    const clip = `${clipOf(id, opts)}~${this.paint.color}-${this.paint.light}`;
    const fx = new PaintedFx(this, m.duration, m.size, m.origin, m.fps, clip);
    fx.colorWord = this.paint.color;
    fx.lightWord = this.paint.light;
    this.live += 1;
    this.created.push(fx);
    return fx;
  }
}
