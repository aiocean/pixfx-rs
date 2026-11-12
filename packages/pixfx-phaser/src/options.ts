import { z } from "zod";

const playOptionsSchema = z.looseObject({
  seed: z.number().int().nonnegative().optional(),
  flip: z.boolean().optional(),
  angle: z.number().optional(),
  light: z.number().nonnegative().optional(),
  margin: z.never({ error: "the plugin sizes the margin itself" }).optional(),
  palette: z.array(z.string()).optional(),
  scrollFactor: z.number().default(1),
  depth: z.object({ back: z.number(), front: z.number() }).optional(),
});

export type PlayOptions = z.infer<typeof playOptionsSchema>;
export type EngineOptions = Record<string, unknown>;

/** Parse untrusted play options; throws an Error naming the offending field. */
export function parsePlayOptions(input: unknown): PlayOptions {
  const r = playOptionsSchema.safeParse(input ?? {});
  if (r.success) return r.data;
  const msg = r.error.issues
    .map((i) => `${i.path.length > 0 ? i.path.join(".") : "options"}: ${i.message}`)
    .join("; ");
  throw new Error(`pixfx: invalid play options (${msg})`);
}

/** Options handed to the wasm engine: everything but the Phaser-side `scrollFactor`/`depth`, plus `margin`. */
export function engineOptions(p: PlayOptions, margin: number): EngineOptions {
  const { scrollFactor: _s, depth: _d, ...rest } = p;
  return { ...rest, margin };
}
