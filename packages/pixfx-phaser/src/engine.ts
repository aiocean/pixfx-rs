import { z } from "zod";
import type { EngineOptions, PlayOptions } from "./options";
import { engineOptions, parsePlayOptions } from "./options";

export type { EngineOptions };

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
  color(): Uint32Array;
  light(): Uint32Array;
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

let loaded: Engine | null = null;

/** Load the wasm module once and return the engine; `getEngine()` works after this resolves. */
export async function loadPixfx(): Promise<Engine> {
  if (loaded) return loaded;
  const wasm = await import("../wasm/pixfx_wasm.js");
  const { memory } = await wasm.default();
  const ids = z.array(z.string()).parse(wasm.effects());
  loaded = {
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
        color: () => fx.color(),
        light: () => fx.light(),
        colorRgba: () => view(fx.colorRgbaPtr()),
        lightRgba: () => view(fx.lightRgbaPtr()),
        free: () => fx.free(),
      };
    },
  };
  return loaded;
}

export function getEngine(): Engine {
  if (!loaded) throw new Error("pixfx: call await loadPixfx() before play()");
  return loaded;
}

/** Validate `opts` and create an engine instance for `id`; throws on an unknown id or invalid options. */
export function spawnFx(engine: Engine, id: string, opts: unknown): { fx: FxLike; play: PlayOptions; fps: number } {
  const play = parsePlayOptions(opts);
  if (!engine.effects().includes(id)) throw new Error(`pixfx: unknown effect "${id}"`);
  const meta = engine.meta(id);
  return { fx: engine.create(id, engineOptions(play, meta.glowMargin)), play, fps: meta.fps };
}
