import { z } from "zod";
import type * as Wasm from "../wasm/pixfx_wasm.js";

export type WasmModule = typeof Wasm;

/** One body's squash and stretch: it lands, and each step it is scaled for how it moves. */
export interface SquashLike {
  land(strength: number): void;
  /**
   * Let `dt` seconds pass for a body now at `(x, y)`, moving at `(vx, vy)` px/s when known (else worked out from
   * how far it moved); returns `[sx, sy]`. With no time passing (a hit-stop) it holds its scale.
   */
  step(dt: number, x: number, y: number, vx?: number, vy?: number): [number, number];
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

const pair = z.tuple([z.number(), z.number()]);

const effectMetaSchema = z.object({
  id: z.string(),
  name: z.string(),
  renderKind: z.enum(["paint", "sprite", "hybrid"]),
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
  /** The palette presets every effect takes by name. */
  palettePresets(): readonly string[];
  /** A body's squash and stretch and its blink, with the default spec. */
  squash(): SquashLike;
  blink(): BlinkLike;
}

/** The effect ids the engine knows, as it types them. */
export function effectIds(wasm: WasmModule): (id: string) => Wasm.EffectId {
  const ids = wasm.effects();
  const known = new Map<string, Wasm.EffectId>(ids.map((id) => [id, id]));
  return (id) => {
    const found = known.get(id);
    if (found === undefined) throw new Error(`pixfx: unknown effect "${id}"`);
    return found;
  };
}

/** The engine over an initialised wasm module. */
export function wasmEngine(wasm: WasmModule): Engine {
  const ids = z.array(z.string()).parse(wasm.effects());
  const effectId = effectIds(wasm);
  const names = z.array(z.string());
  return {
    effects: () => ids,
    meta: (id) => effectMetaSchema.parse(wasm.meta(effectId(id))),
    palettePresets: () => names.parse(wasm.palettePresets()),
    squash: () => {
      const s = new wasm.Squash(undefined);
      return {
        land: (strength) => s.land(strength),
        step: (dt, x, y, vx, vy) => pair.parse(s.step(dt, x, y, vx, vy)),
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
