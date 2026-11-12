import { z } from "zod";
import type * as Wasm from "../wasm/pixfx_wasm.js";
import type { EngineOptions, PlayOptions } from "./options";
import { engineOptions, parsePlayOptions } from "./options";

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
}

/** The engine over an initialised wasm module (`memory` is what its default init resolved to). */
export function wasmEngine(wasm: typeof Wasm, memory: WebAssembly.Memory): Engine {
  const ids = z.array(z.string()).parse(wasm.effects());
  return {
    effects: () => ids,
    meta: (id) => effectMetaSchema.parse(wasm.meta(id)),
    create: (id, opts) => {
      const fx = new wasm.Fx(id, opts);
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
        free: () => fx.free(),
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
