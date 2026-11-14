// A painting worker: loads the engine, then paints each message's jobs and transfers the bytes back.
import init, { Painting } from "../wasm/pixfx_wasm.js";
import { paintAll } from "./paint";
import type { FromWorker } from "./protocol";
import { toWorkerSchema } from "./protocol";

let painting: Painting | null = null;
let slowdown = 1;

function reply(message: FromWorker, transfer: ArrayBuffer[] = []): void {
  self.postMessage(message, { transfer });
}

async function handle(data: unknown): Promise<void> {
  const msg = toWorkerSchema.parse(data);
  if (msg.type === "init") {
    await init();
    painting = new Painting();
    slowdown = msg.slowdown;
    reply({ type: "ready" });
    return;
  }
  if (!painting) throw new Error("paint before init");
  const start = performance.now();
  const { reply: painted, transfer } = paintAll(painting, msg);
  // Phone emulation: Chrome cannot CPU-throttle a worker, so stretch the work to `slowdown` times its cost.
  const until = start + (performance.now() - start) * slowdown;
  while (performance.now() < until) {
    // busy-wait
  }
  reply(painted, transfer);
}

self.onmessage = (e: MessageEvent<unknown>) => {
  handle(e.data).catch((err: unknown) => reply({ type: "error", message: err instanceof Error ? err.message : String(err) }));
};
