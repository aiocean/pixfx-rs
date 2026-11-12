// An effect worker: loads the engine, then runs each batch the pool sends and replies with the frames drawn.
import init, * as wasm from "../wasm/pixfx_wasm.js";
import { wasmEngine } from "./engine";
import type { FromWorker } from "./protocol";
import { toWorkerSchema } from "./protocol";
import { WorkerCore } from "./worker-core";

let core: WorkerCore | null = null;
let slowdown = 1;

function reply(message: FromWorker, transfer: ArrayBuffer[] = []): void {
  self.postMessage(message, { transfer });
}

async function handle(data: unknown): Promise<void> {
  const msg = toWorkerSchema.parse(data);
  if (msg.type === "init") {
    const { memory } = await init();
    core = new WorkerCore(wasmEngine(wasm, memory));
    slowdown = msg.slowdown;
    reply({ type: "ready" });
    return;
  }
  if (!core) throw new Error("batch before init");
  const start = performance.now();
  const { reply: results, transfer } = core.run(msg);
  // Phone emulation: Chrome cannot CPU-throttle a worker, so stretch the batch to `slowdown` times its cost.
  const until = start + (performance.now() - start) * slowdown;
  while (performance.now() < until) {
    // busy-wait
  }
  reply(results, transfer);
}

self.onmessage = (e: MessageEvent<unknown>) => {
  handle(e.data).catch((err: unknown) => reply({ type: "error", message: err instanceof Error ? err.message : String(err) }));
};
