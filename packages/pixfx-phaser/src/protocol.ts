// Messages between the main thread and an effect worker. Both sides parse what they receive.
import { z } from "zod";
import { engineOptionsSchema } from "./options";

const handle = z.number().int().nonnegative();
const buffer = z.instanceof(ArrayBuffer);
const px = z.number().int().nonnegative();

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

/** A box of whole pixels on an effect's canvas: top-left (`x`, `y`), `w` × `h` pixels. */
export const boxSchema = z.object({ x: px, y: px, w: px, h: px });
export type Box = z.infer<typeof boxSchema>;

/** One side (back or front) of a frame: its colour and its light as RGBA bytes (`w × h × 4`) of the pixels in `box`. */
export const sideSchema = z.object({ box: boxSchema, color: buffer, light: buffer });
export type Side = z.infer<typeof sideSchema>;

/** One drawn frame: its back and its front. */
export const frameSchema = z.object({ back: sideSchema, front: sideSchema });
export type Frame = z.infer<typeof frameSchema>;

const spawnSchema = z.object({ handle, id: z.string(), opts: engineOptionsSchema });

const itemSchema = z.object({
  handle,
  /** Scene seconds to advance (0: none). */
  dt: z.number().nonnegative(),
  stop: z.boolean(),
  /** The main thread lacks the frame the effect last reported: report it again, drawn. */
  want: z.boolean(),
});

const batchSchema = z.object({
  type: z.literal("batch"),
  /** Shared frames the main thread no longer holds. */
  drops: z.array(z.string()),
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

const resultSchema = z.object({
  handle,
  done: z.boolean(),
  cues: z.array(timedCueSchema),
  /** The shared frame the effect moved to (its bytes in the reply's `frames` when the main thread lacked it). */
  shown: z.string().optional(),
  /** The effect's own frame while it fades, the whole canvas. */
  own: frameSchema.optional(),
});
export type Result = z.infer<typeof resultSchema>;

/** A shared frame the main thread lacked: sent once, cropped to each side's lit box. */
const sharedSchema = z.object({ id: z.string(), frame: frameSchema });
export type Shared = z.infer<typeof sharedSchema>;

export const fromWorkerSchema = z.discriminatedUnion("type", [
  z.object({ type: z.literal("ready") }),
  z.object({ type: z.literal("error"), message: z.string() }),
  /** One reply per batch: the shared frames it drew, then every effect that moved to a new frame, fired cues or finished. */
  z.object({ type: z.literal("results"), frames: z.array(sharedSchema), results: z.array(resultSchema) }),
]);
export type FromWorker = z.infer<typeof fromWorkerSchema>;

/** Every buffer of a frame, for a transfer list. */
export function frameBuffers(f: Frame): ArrayBuffer[] {
  return [f.back.color, f.back.light, f.front.color, f.front.light];
}
