// Messages between the main thread and an effect worker. Both sides parse what they receive.
import { z } from "zod";
import { engineOptionsSchema } from "./options";

const handle = z.number().int().nonnegative();
const buffer = z.instanceof(ArrayBuffer);

/** One drawn frame as RGBA bytes: back colour, back light, front colour, front light. */
export const frameBuffersSchema = z.tuple([buffer, buffer, buffer, buffer]);
export type FrameBuffers = z.infer<typeof frameBuffersSchema>;

const spawnSchema = z.object({
  handle,
  id: z.string(),
  opts: engineOptionsSchema,
  /** The effect's authored frame rate: the worker draws only on a new frame of it. */
  fps: z.number().positive(),
});

const itemSchema = z.object({
  handle,
  /** Scene seconds to advance (0: none). */
  dt: z.number().nonnegative(),
  stop: z.boolean(),
  /** Frames the main thread has shown, handed back for reuse. */
  give: z.array(frameBuffersSchema),
});

const batchSchema = z.object({
  type: z.literal("batch"),
  frees: z.array(handle),
  spawns: z.array(spawnSchema),
  items: z.array(itemSchema),
});

export type Spawn = z.infer<typeof spawnSchema>;
export type Item = z.infer<typeof itemSchema>;
export type Batch = z.infer<typeof batchSchema>;

export const toWorkerSchema = z.discriminatedUnion("type", [
  /** Load the engine. `slowdown` > 1 stretches each batch to that many times its time (phone emulation). */
  z.object({ type: z.literal("init"), slowdown: z.number().min(1) }),
  batchSchema,
]);
export type ToWorker = z.infer<typeof toWorkerSchema>;

const resultSchema = z.object({ handle, done: z.boolean(), frame: frameBuffersSchema.optional() });
export type Result = z.infer<typeof resultSchema>;

export const fromWorkerSchema = z.discriminatedUnion("type", [
  z.object({ type: z.literal("ready") }),
  z.object({ type: z.literal("error"), message: z.string() }),
  /** One reply per batch: every effect that drew a new frame or finished. */
  z.object({ type: z.literal("results"), results: z.array(resultSchema) }),
]);
export type FromWorker = z.infer<typeof fromWorkerSchema>;
