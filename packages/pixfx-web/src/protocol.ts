// Messages between the main thread and a painting worker. Both sides parse what they receive; a paint job itself
// is read strictly by the engine (`Painting.paint`), so it crosses as it came from `frames.takeJobs()`.
import { z } from "zod";

const n = z.number().int().nonnegative();

/** Where a painted frame's bytes sit on the frame, and which halves hold any (`Shape` of `pixfx-wasm`). */
export const shapeSchema = z.strictObject({ frameW: n, frameH: n, x: n, y: n, w: n, h: n, sides: n });
export type Shape = z.infer<typeof shapeSchema>;

export const toWorkerSchema = z.discriminatedUnion("type", [
  /** Load the engine. `slowdown` > 1 stretches each message's work to that many times its time (phone emulation). */
  z.object({ type: z.literal("init"), slowdown: z.number().min(1) }),
  /** Frames to paint, oldest first, then effects (`[scene, handle]`) whose state can go. */
  z.object({ type: z.literal("paint"), jobs: z.array(z.unknown()), ended: z.array(z.tuple([n, n])) }),
]);
export type ToWorker = z.infer<typeof toWorkerSchema>;

/** A painted frame: the job it answers, the shape of its bytes, and the bytes (transferred). */
const paintedSchema = z.object({ job: n, scene: n, shape: shapeSchema, color: z.instanceof(ArrayBuffer), light: z.instanceof(ArrayBuffer) });
export type Painted = z.infer<typeof paintedSchema>;

export const fromWorkerSchema = z.discriminatedUnion("type", [
  z.object({ type: z.literal("ready") }),
  z.object({ type: z.literal("error"), message: z.string() }),
  /** One reply per paint message: every job's frame, in order. */
  z.object({ type: z.literal("painted"), painted: z.array(paintedSchema) }),
]);
export type FromWorker = z.infer<typeof fromWorkerSchema>;
