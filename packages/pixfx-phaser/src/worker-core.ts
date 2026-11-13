import type { Engine } from "./engine";
import type { Batch, FromWorker, Result } from "./protocol";
import { FxRunner } from "./runner";

/** The work of one effect worker, with no thread or messaging: runs a batch against the engine. */
export class WorkerCore {
  private readonly runners = new Map<number, FxRunner>();
  private allocatedByFreed = 0;

  constructor(private readonly engine: Engine) {}

  /** Buffer sets allocated by every instance this worker has run. */
  get allocated(): number {
    let n = this.allocatedByFreed;
    for (const r of this.runners.values()) n += r.allocated;
    return n;
  }

  /**
   * Free, spawn, then for each item: take back its buffers, stop, advance. Replies with every instance that drew
   * a new frame (its buffers in the transfer list), fired cues, or finished (freed here).
   */
  run(batch: Batch): { reply: FromWorker; transfer: ArrayBuffer[] } {
    for (const h of batch.frees) this.free(h);
    const spawned = new Set<number>();
    for (const s of batch.spawns) {
      this.runners.set(s.handle, new FxRunner(this.engine.create(s.id, s.opts), s.fps));
      spawned.add(s.handle);
    }
    const results: Result[] = [];
    const transfer: ArrayBuffer[] = [];
    for (const item of batch.items) {
      const runner = this.runners.get(item.handle);
      if (!runner) continue;
      for (const set of item.give) runner.give(set);
      if (item.stop) runner.stop();
      if (item.dt === 0 && !spawned.has(item.handle)) continue;
      const step = runner.step(item.dt);
      const cues = step.cues;
      if (step.kind === "done") {
        this.free(item.handle);
        results.push({ handle: item.handle, done: true, cues });
      } else if (step.kind === "frame") {
        results.push({ handle: item.handle, done: false, frame: step.frame, cues });
        transfer.push(...step.frame);
      } else if (cues.length > 0) {
        results.push({ handle: item.handle, done: false, cues });
      }
    }
    return { reply: { type: "results", results }, transfer };
  }

  private free(handle: number): void {
    const r = this.runners.get(handle);
    if (!r) return;
    r.free();
    this.allocatedByFreed += r.allocated;
    this.runners.delete(handle);
  }
}
