// The work of a painting worker, without the thread: paint every job of a message with the engine's `Painting`
// (it brings each effect's state to the frame, draws it, crops it and turns it into RGBA), then let go of the
// effects that ended.
import { z } from "zod";
import type { Painting } from "../wasm/pixfx_wasm.js";
import type { FromWorker, Painted, ToWorker } from "./protocol";
import { shapeSchema } from "./protocol";

/** What the worker reads of a job itself, to answer it; the engine reads the rest. */
const jobRefSchema = z.object({ job: z.number().int().nonnegative(), scene: z.number().int().nonnegative() });

/** Paint `msg`'s jobs; returns the reply and the buffers to transfer with it. */
export function paintAll(painting: Painting, msg: ToWorker & { type: "paint" }): { reply: FromWorker; transfer: ArrayBuffer[] } {
  const painted: Painted[] = [];
  const transfer: ArrayBuffer[] = [];
  for (const job of msg.jobs) {
    const ref = jobRefSchema.parse(job);
    const p = painting.paint(job);
    const shape = shapeSchema.parse(p.shape());
    const color = p.color().buffer;
    const light = p.light().buffer;
    p.free();
    if (!(color instanceof ArrayBuffer) || !(light instanceof ArrayBuffer)) throw new Error("painted bytes are not transferable");
    painted.push({ ...ref, shape, color, light });
    transfer.push(color, light);
  }
  // after the jobs: an effect may end in the same message as its last frame
  for (const [scene, handle] of msg.ended) painting.end(scene, handle);
  return { reply: { type: "painted", painted }, transfer };
}
