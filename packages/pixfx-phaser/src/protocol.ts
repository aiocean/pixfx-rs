// Messages between the main thread and an effect worker. Both sides parse what they receive.
import { z } from "zod";
import { engineOptionsSchema } from "./options";

const handle = z.number().int().nonnegative();
const buffer = z.instanceof(ArrayBuffer);

/** A cue an effect fired (as `Fx.takeCues()` gives it): a ripple's and a sound's `x`/`y` relative to its origin. */
export const cueSchema = z.discriminatedUnion("type", [
  z.object({ type: z.literal("shake"), trauma: z.number() }),
  z.object({ type: z.literal("hitstop"), dur: z.number() }),
  z.object({ type: z.literal("ripple"), x: z.number(), y: z.number(), r0: z.number(), r1: z.number(), dur: z.number(), amp: z.number() }),
  z.object({ type: z.literal("flash"), k: z.number(), dur: z.number() }),
  z.object({ type: z.literal("sound"), name: z.string(), gain: z.number(), pitch: z.number(), len: z.number().nullable(), x: z.number().nullable() }),
]);
export type Cue = z.infer<typeof cueSchema>;

/** A cue and when it fired, in seconds of the effect's playback; `pan` -1 to 1 once a screen width is known. */
export const timedCueSchema = z.object({ t: z.number(), pan: z.number().nullable(), cue: cueSchema });
export type TimedCue = z.infer<typeof timedCueSchema>;

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

const resultSchema = z.object({ handle, done: z.boolean(), frame: frameBuffersSchema.optional(), cues: z.array(timedCueSchema) });
export type Result = z.infer<typeof resultSchema>;

export const fromWorkerSchema = z.discriminatedUnion("type", [
  z.object({ type: z.literal("ready") }),
  z.object({ type: z.literal("error"), message: z.string() }),
  /** One reply per batch: every effect that drew a new frame, fired cues or finished. */
  z.object({ type: z.literal("results"), results: z.array(resultSchema) }),
]);
export type FromWorker = z.infer<typeof fromWorkerSchema>;
