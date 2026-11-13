import type { Engine } from "../src/engine";
import type { WorkerPort } from "../src/pool";
import type { Batch, ToWorker } from "../src/protocol";
import { toWorkerSchema } from "../src/protocol";
import { WorkerCore } from "../src/worker-core";

/**
 * A worker without a thread: posts are structured-cloned with their transfer list (so a transferred buffer is
 * detached on the sending side, as in a browser) and wait until the test calls `answer()`, which runs them
 * through a real `WorkerCore` on its own engine.
 */
export class LoopPort<E extends Engine = Engine> implements WorkerPort {
  readonly core: WorkerCore;
  readonly posted: ToWorker[] = [];
  private queue: ToWorker[] = [];
  private deliver: ((data: unknown) => void) | null = null;

  constructor(readonly engine: E) {
    this.core = new WorkerCore(engine);
  }
  post(message: ToWorker, transfer: ArrayBuffer[]): void {
    const copy = toWorkerSchema.parse(structuredClone(message, { transfer }));
    this.posted.push(copy);
    this.queue.push(copy);
  }
  listen(onMessage: (data: unknown) => void): void {
    this.deliver = onMessage;
  }
  /** Run the oldest posted batch and deliver the reply; false when nothing is in flight. */
  answer(): boolean {
    const msg = this.queue.shift();
    if (!msg) return false;
    if (msg.type !== "batch") throw new Error("the pool only posts batches");
    const { reply, transfer } = this.core.run(msg);
    this.deliver?.(structuredClone(reply, { transfer }));
    return true;
  }
  batches(): Batch[] {
    return this.posted.flatMap((m) => (m.type === "batch" ? [m] : []));
  }
}
