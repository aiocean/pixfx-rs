import { z } from "zod";

/**
 * Play options as the game gives them: Phaser's own (`scrollFactor`, `depth`) are read here; every other field is
 * the engine's, read strictly by Rust when the effect plays (an unknown preset or option throws, naming it).
 */
const playOptionsSchema = z
  .object({
    scrollFactor: z.number().default(1),
    depth: z.object({ back: z.number(), front: z.number() }).optional(),
  })
  .catchall(z.unknown());

/** Phaser's side of a play, and the engine's. */
export interface Play {
  scrollFactor: number;
  depth: { back: number; front: number } | undefined;
  engine: Record<string, unknown>;
}

/** Split untrusted play options into Phaser's and the engine's; throws an Error naming a bad Phaser field. */
export function parsePlayOptions(input: unknown): Play {
  const r = playOptionsSchema.safeParse(input ?? {});
  if (!r.success) {
    const msg = r.error.issues.map((i) => `${i.path.length > 0 ? i.path.join(".") : "options"}: ${i.message}`).join("; ");
    throw new Error(`pixfx: invalid play options (${msg})`);
  }
  const { scrollFactor, depth, ...engine } = r.data;
  return { scrollFactor, depth, engine };
}
