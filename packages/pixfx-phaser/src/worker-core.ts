import type { Engine } from "./engine";
import type { Batch, FromWorker, Result, Shared } from "./protocol";
import { frameBuffers } from "./protocol";
import { FxRunner } from "./runner";

/**
 * The work of one effect worker, with no thread or messaging: runs a batch against the engine. It remembers the
 * shared frames the main thread holds (every one it sent, less the ones the main thread dropped), so each is
 * drawn and sent once.
 */
export class WorkerCore {
  private readonly runners = new Map<number, FxRunner>();
  private readonly held = new Set<string>();

  constructor(private readonly engine: Engine) {}

  /**
   * Forget dropped frames, free, spawn, then for each item: stop, advance. Replies with the shared frames drawn
   * (their buffers in the transfer list) and every instance that moved to a new frame, fired cues, or finished
   * (freed here).
   */
  run(batch: Batch): { reply: FromWorker; transfer: ArrayBuffer[] } {
    for (const id of batch.drops) this.held.delete(id);
    for (const h of batch.frees) this.free(h);
    const spawned = new Set<number>();
    for (const s of batch.spawns) {
      this.runners.set(s.handle, new FxRunner(this.engine.create(s.id, s.opts)));
      spawned.add(s.handle);
    }
    const frames: Shared[] = [];
    const results: Result[] = [];
    const transfer: ArrayBuffer[] = [];
    const held = (id: string): boolean => this.held.has(id);
    for (const item of batch.items) {
      const runner = this.runners.get(item.handle);
      if (!runner) continue;
      if (item.stop) runner.stop();
      if (item.dt === 0 && !item.want && !spawned.has(item.handle)) continue;
      const step = runner.step(item.dt, held, item.want);
      const base = { handle: item.handle, done: false, cues: step.cues };
      if (step.kind === "done") {
        this.free(item.handle);
        results.push({ ...base, done: true });
      } else if (step.kind === "shared") {
        if (step.frame) {
          this.held.add(step.id);
          frames.push({ id: step.id, frame: step.frame });
          transfer.push(...frameBuffers(step.frame));
        }
        results.push({ ...base, shown: step.id });
      } else if (step.kind === "own") {
        results.push({ ...base, own: step.frame });
        transfer.push(...frameBuffers(step.frame));
      } else if (step.cues.length > 0) {
        results.push(base);
      }
    }
    return { reply: { type: "results", frames, results }, transfer };
  }

  private free(handle: number): void {
    this.runners.get(handle)?.free();
    this.runners.delete(handle);
  }
}
