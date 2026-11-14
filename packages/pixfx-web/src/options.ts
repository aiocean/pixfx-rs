import { z } from "zod";

/** A shell's side of a play, and the engine's. */
export interface Play<O> {
  own: O;
  engine: Record<string, unknown>;
}

/**
 * Split untrusted play options into the shell's own fields (`shape`, e.g. Phaser's `scrollFactor` and `depth`) and
 * the engine's: every other field, read strictly by Rust when the effect plays (an unknown preset or option throws,
 * naming it). Throws an Error naming a bad shell field.
 */
export function splitPlayOptions<S extends z.ZodRawShape>(shape: S, input: unknown): Play<z.output<z.ZodObject<S>>> {
  const r = z.object(shape).catchall(z.unknown()).safeParse(input ?? {});
  if (!r.success) {
    const msg = r.error.issues.map((i) => `${i.path.length > 0 ? i.path.join(".") : "options"}: ${i.message}`).join("; ");
    throw new Error(`pixfx: invalid play options (${msg})`);
  }
  const own = z.object(shape).parse(r.data);
  const engine = Object.fromEntries(Object.entries(r.data).filter(([k]) => !Object.hasOwn(shape, k)));
  return { own, engine };
}
