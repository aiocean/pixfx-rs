import { z } from "zod";
import type { FxOptions } from "../wasm/pixfx_wasm.js";

// An effect's own option, as the engine takes it (`FxOptions`): a number, a colour, a list of colours or a flag.
const ownOption = z.union([z.number(), z.string(), z.array(z.string()), z.boolean()]);

// The outline every effect takes (`OutlineOption`): `false`, `true`, a word ("off", a preset, a hex colour) or a ring.
const outlineOption = z.union([
  z.boolean(),
  z.string(),
  z.strictObject({
    color: z.string().optional(),
    width: z.number().int().min(1).max(3).optional(),
    corners: z.boolean().optional(),
  }),
]);

const playOptionsSchema = z.object({
  seed: z.number().int().nonnegative().optional(),
  flip: z.boolean().optional(),
  angle: z.number().optional(),
  light: z.number().nonnegative().optional(),
  margin: z.never({ error: "the plugin sizes the margin itself" }).optional(),
  palette: z.array(z.string()).optional(),
  outline: outlineOption.optional(),
  scrollFactor: z.number().default(1),
  depth: z.object({ back: z.number(), front: z.number() }).optional(),
}).catchall(ownOption);

export type PlayOptions = z.infer<typeof playOptionsSchema>;
export type EngineOptions = FxOptions;

/** Engine options as they cross to a worker: the ones every effect reads, `margin`, and the effect's own. */
export const engineOptionsSchema = z
  .object({
    margin: z.number().optional(),
    seed: z.number().optional(),
    flip: z.boolean().optional(),
    angle: z.number().optional(),
    light: z.number().optional(),
    palette: z.array(z.string()).optional(),
    outline: outlineOption.optional(),
  })
  // as `FxOptions` types any key (the engine checks each against the effect)
  .catchall(z.union([ownOption, outlineOption]).optional());

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
