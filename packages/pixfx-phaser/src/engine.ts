import { z } from "zod";
import type * as Wasm from "../wasm/pixfx_wasm.js";
import type { EngineOptions, PlayOptions } from "./options";
import { engineOptions, parsePlayOptions } from "./options";
import type { TimedCue } from "./protocol";
import { timedCueSchema } from "./protocol";

export type { EngineOptions };

/** One engine instance of an effect. */
export interface FxLike {
  update(dt: number): void;
  stop(): void;
  done(): boolean;
  t(): number;
  width(): number;
  height(): number;
  originX(): number;
  originY(): number;
  draw(layer: "back" | "front"): void;
  /** The colour layer as RGBA bytes (straight alpha); a view valid until the next call on this instance. */
  colorRgba(): Uint8ClampedArray;
  /** The light layer as RGBA bytes (lit pixels opaque, unlit `0,0,0,0`); valid until the next call. */
  lightRgba(): Uint8ClampedArray;
  /** The cues fired since the last call, oldest first; clears them. */
  takeCues(): TimedCue[];
  free(): void;
}

/** One body's squash and stretch: it lands, time passes, it is scaled for its speed. */
export interface SquashLike {
  land(strength: number): void;
  advance(dt: number): void;
  /** `[sx, sy]` for a body moving up or down at `vy` px/s; the area is kept. */
  scale(vy: number): [number, number];
  free(): void;
}

/** One body's blink while it cannot be hurt. */
export interface BlinkLike {
  start(secs: number): void;
  advance(dt: number): void;
  active(): boolean;
  alpha(): number;
  free(): void;
}

/** The engine's answer to impacts: trauma and shake, hit-stop, slow motion, flash. */
export interface FeelLike {
  /** Take in cues effects fired (sounds are skipped). */
  take(cues: readonly TimedCue[]): void;
  addTrauma(k: number): void;
  freeze(secs: number): void;
  slow(secs: number, speed: number): void;
  /** Let `dt` real seconds pass; returns the seconds play runs for them. */
  advance(dt: number): number;
  /** Where the view sits, `[dx, dy]` in game pixels, at `strength` (1 as designed, 0 still). */
  shake(strength: number): [number, number];
  trauma(): number;
  frozen(): boolean;
  /** How white the view is lifted, 0 to 1. */
  flash(): number;
  free(): void;
}

const pair = z.tuple([z.number(), z.number()]);

const effectMetaSchema = z.object({
  id: z.string(),
  name: z.string(),
  size: pair,
  origin: pair,
  duration: z.number(),
  loop: z.boolean(),
  fps: z.number(),
  seed: z.number(),
  glowMargin: z.number(),
});

export type EffectMeta = z.infer<typeof effectMetaSchema>;

export interface Engine {
  effects(): readonly string[];
  meta(id: string): EffectMeta;
  create(id: string, opts: EngineOptions): FxLike;
  /** A fresh answer to impacts, with the default spec. */
  feel(): FeelLike;
  /** A body's squash and stretch and its blink, with the default spec. */
  squash(): SquashLike;
  blink(): BlinkLike;
}

/** The engine over an initialised wasm module (`memory` is what its default init resolved to). */
export function wasmEngine(wasm: typeof Wasm, memory: WebAssembly.Memory): Engine {
  const ids = z.array(z.string()).parse(wasm.effects());
  const isEffectId = (id: string): id is Wasm.EffectId => ids.includes(id);
  const effectId = (id: string): Wasm.EffectId => {
    if (!isEffectId(id)) throw new Error(`pixfx: unknown effect "${id}"`);
    return id;
  };
  return {
    effects: () => ids,
    meta: (id) => effectMetaSchema.parse(wasm.meta(effectId(id))),
    create: (id, opts) => {
      const fx = new wasm.Fx(effectId(id), opts);
      const bytes = fx.width() * fx.height() * 4;
      // A view on wasm memory, made after the call: a call may grow the memory and detach older buffers.
      const view = (ptr: number): Uint8ClampedArray => new Uint8ClampedArray(memory.buffer, ptr, bytes);
      return {
        update: (dt) => fx.update(dt),
        stop: () => fx.stop(),
        done: () => fx.done(),
        t: () => fx.t(),
        width: () => fx.width(),
        height: () => fx.height(),
        originX: () => fx.originX(),
        originY: () => fx.originY(),
        draw: (layer) => fx.draw(layer),
        colorRgba: () => view(fx.colorRgbaPtr()),
        lightRgba: () => view(fx.lightRgbaPtr()),
        takeCues: () => z.array(timedCueSchema).parse(fx.takeCues()),
        free: () => fx.free(),
      };
    },
    feel: () => {
      const f = new wasm.Feel(undefined);
      return {
        take: (cues) => f.take([...cues]),
        addTrauma: (k) => f.addTrauma(k),
        freeze: (secs) => f.freeze(secs),
        slow: (secs, speed) => f.slow(secs, speed),
        advance: (dt) => f.advance(dt),
        shake: (strength) => pair.parse(f.shake(strength)),
        trauma: () => f.trauma(),
        frozen: () => f.frozen(),
        flash: () => f.flash(),
        free: () => f.free(),
      };
    },
    squash: () => {
      const s = new wasm.Squash(undefined);
      return {
        land: (strength) => s.land(strength),
        advance: (dt) => s.advance(dt),
        scale: (vy) => pair.parse(s.scale(vy)),
        free: () => s.free(),
      };
    },
    blink: () => {
      const b = new wasm.Blink(undefined);
      return {
        start: (secs) => b.start(secs),
        advance: (dt) => b.advance(dt),
        active: () => b.active(),
        alpha: () => b.alpha(),
        free: () => b.free(),
      };
    },
  };
}

/** A validated play: what to hand the engine, and the Phaser-side options. */
export interface Prepared {
  id: string;
  play: PlayOptions;
  engineOpts: EngineOptions;
  fps: number;
}

/** Validate `opts` and `id` for `engine`; throws on an unknown id or invalid options. Creates nothing. */
export function prepare(engine: Engine, id: string, opts: unknown): Prepared {
  const play = parsePlayOptions(opts);
  if (!engine.effects().includes(id)) throw new Error(`pixfx: unknown effect "${id}"`);
  const meta = engine.meta(id);
  return { id, play, engineOpts: engineOptions(play, meta.glowMargin), fps: meta.fps };
}
