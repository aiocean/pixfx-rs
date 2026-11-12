import { z } from "zod";
import type { EngineOptions } from "./options";

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
  await wasm.default();
  const ids = z.array(z.string()).parse(wasm.effects());
  loaded = {
    effects: () => ids,
    meta: (id) => effectMetaSchema.parse(wasm.meta(id)),
    create: (id, opts) => new wasm.Fx(id, opts),
  };
  return loaded;
}

export function getEngine(): Engine {
  if (!loaded) throw new Error("pixfx: call await loadPixfx() before play()");
  return loaded;
}
