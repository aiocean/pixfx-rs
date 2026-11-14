import { z } from "zod";
import { splitPlayOptions } from "pixfx-web";

/** Phaser's own play options: the images' camera scroll factor and depths. Every other field is the engine's. */
const OWN = {
  scrollFactor: z.number().default(1),
  depth: z.object({ back: z.number(), front: z.number() }).optional(),
};

/** Phaser's side of a play, and the engine's. */
export interface Play {
  scrollFactor: number;
  depth: { back: number; front: number } | undefined;
  engine: Record<string, unknown>;
}

/** Split untrusted play options into Phaser's and the engine's; throws an Error naming a bad Phaser field. */
export function parsePlayOptions(input: unknown): Play {
  const { own, engine } = splitPlayOptions(OWN, input);
  return { scrollFactor: own.scrollFactor, depth: own.depth, engine };
}
