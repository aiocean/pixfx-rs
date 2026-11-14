import { expect, test } from "bun:test";
import type { WorkerPort } from "../src/game";
import { Game } from "../src/game";
import { paintAll } from "../src/paint";
import type { ToWorker } from "../src/protocol";
import { toWorkerSchema } from "../src/protocol";
import { FakeGfx } from "./fake-gfx";
import { Stage } from "../src/stage";
import { memory, wasm } from "./wasm";

/**
 * A painting worker in the test's thread: it keeps what the game posts and, on `run()`, paints it with the engine
 * as `worker.ts` does and hands the reply back, through structured clone as a real worker's message would be.
 */
class LoopPort implements WorkerPort {
  readonly posted: (ToWorker & { type: "paint" })[] = [];
  private queue: unknown[] = [];
  private onMessage: (data: unknown) => void = () => {};
  private readonly painting = new wasm.Painting();

  post(message: ToWorker): void {
    const msg = toWorkerSchema.parse(structuredClone(message));
    if (msg.type === "paint") this.posted.push(msg);
    this.queue.push(msg);
  }

  listen(onMessage: (data: unknown) => void): void {
    this.onMessage = onMessage;
  }

  run(): void {
    const queue = this.queue;
    this.queue = [];
    for (const m of queue) {
      const msg = toWorkerSchema.parse(m);
      if (msg.type !== "paint") continue;
      const { reply, transfer } = paintAll(this.painting, msg);
      this.onMessage(structuredClone(reply, { transfer }));
    }
  }
}

function setup(ports: LoopPort[]) {
  const gfx = new FakeGfx();
  const game = new Game(wasm, memory, ports);
  game.useTextures(gfx.texture);
  const stage = new Stage<number>(game, gfx.image, () => {}, 1);
  return { gfx, game, stage };
}

/**
 * Play a candle (stopped at `stopAt`) for `steps` host frames of 1/30 s. With workers, each step's frames are
 * painted and delivered, then the scene draws again without time passing, so it shows the frames of that step.
 */
function run(ports: LoopPort[], steps: number, stopAt: number) {
  const { gfx, game, stage } = setup(ports);
  const fx = stage.play("fire/candle", 30, 40, { seed: 4 }, 1);
  const pictures: ReturnType<FakeGfx["picture"]>[] = [];
  for (let i = 0; i < steps; i++) {
    if (i === stopAt) fx.stop();
    stage.update(1 / 30);
    stage.draw();
    game.endFrame();
    if (ports.length > 0) {
      for (const p of ports) p.run();
      stage.update(0);
      stage.draw();
      game.endFrame();
    }
    pictures.push(gfx.picture());
  }
  return pictures;
}

test("frames painted on workers show the same pixels at the same places as frames painted on the spot", () => {
  for (const stopAt of [Number.POSITIVE_INFINITY, 4]) {
    const inline = run([], 10, stopAt);
    const workers = run([new LoopPort(), new LoopPort()], 10, stopAt);
    expect(inline.some((p) => p.length > 0)).toBe(true);
    expect(workers).toEqual(inline);
  }
});

test("until a worker sends its frame back, an effect shows the last one it had", () => {
  const port = new LoopPort();
  const { gfx, game, stage } = setup([port]);
  stage.play("fire/candle", 30, 40, {}, 1);
  stage.update(1 / 30);
  stage.draw();
  game.endFrame();
  expect(gfx.showing()).toEqual([]);
  port.run();
  stage.update(0);
  stage.draw();
  game.endFrame();
  const shown = gfx.picture();
  expect(shown.length).toBeGreaterThan(0);
  stage.update(1 / 30);
  stage.draw();
  game.endFrame();
  expect(gfx.picture()).toEqual(shown);
});

test("an effect is painted by one worker, which lets go of its state when it ends", () => {
  const ports = [new LoopPort(), new LoopPort()];
  const { game, stage } = setup(ports);
  const fxs = [0, 1, 2, 3].map((i) => stage.play("fire/candle", 20 * i, 40, { seed: i }, 1));
  for (let i = 0; i < 5; i++) {
    stage.update(1 / 30);
    stage.draw();
    game.endFrame();
    for (const p of ports) p.run();
  }
  for (const fx of fxs) fx.destroy();
  stage.update(1 / 30);
  stage.draw();
  game.endFrame();
  const painters = new Map<string, Set<LoopPort>>();
  for (const p of ports) {
    for (const m of p.posted) {
      for (const job of m.jobs) {
        const { scene, handle } = jobKey(job);
        const key = `${scene}:${handle}`;
        const set = painters.get(key) ?? new Set();
        set.add(p);
        painters.set(key, set);
      }
    }
  }
  expect(painters.size).toBe(4);
  for (const set of painters.values()) expect(set.size).toBe(1);
  expect(new Set(ports.map((p) => p.posted.length > 0))).toEqual(new Set([true]));
  const ended = ports.flatMap((p) => p.posted.flatMap((m) => m.ended.map(([s, h]) => `${s}:${h}`)));
  expect(new Set(ended)).toEqual(new Set(painters.keys()));
});

function jobKey(job: unknown): { scene: number; handle: number } {
  if (typeof job !== "object" || job === null || !("scene" in job) || !("handle" in job)) throw new Error("not a job");
  const { scene, handle } = job;
  if (typeof scene !== "number" || typeof handle !== "number") throw new Error("not a job");
  return { scene, handle };
}
