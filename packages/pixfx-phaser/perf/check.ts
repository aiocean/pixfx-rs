// `bun run perf:check`: in a clean Chrome, check that the demo (default worker pool) shows its fires, that pausing
// the scene freezes them, and that an effect drawn by a worker uploads exactly the bytes the main thread uploads
// for the same effect, seed and steps. Prints one line per check; exits 1 if any fails.
import { tmpdir } from "node:os";
import { join } from "node:path";
import { z } from "zod";
import type { Browser } from "./cdp";
import { launchChrome } from "./cdp";
import { ROOT, startServer } from "./server";

const CDP_PORT = 9334;
const SRC = `/@fs${join(import.meta.dir, "..", "src")}`;

async function waitFor(b: Browser, expression: string, timeoutMs: number): Promise<void> {
  const until = Date.now() + timeoutMs;
  while (Date.now() < until) {
    if ((await b.evaluate(expression)) === true) return;
    await Bun.sleep(100);
  }
  throw new Error(`check: timed out waiting for ${expression}`);
}

async function screenshot(b: Browser): Promise<string> {
  return z.object({ data: z.string() }).parse(await b.send("Page.captureScreenshot", { format: "png" })).data;
}

/** Lit pixels in the demo's pixfx textures (their canvases hold the last upload). */
const LIT_PIXELS = `(() => {
  const s = window.pixfxDemo.scene;
  let lit = 0;
  for (const key of s.textures.getTextureKeys()) {
    if (!key.startsWith("pixfx:")) continue;
    const canvas = s.textures.get(key).getSourceImage();
    const px = canvas.getContext("2d").getImageData(0, 0, canvas.width, canvas.height).data;
    for (let i = 3; i < px.length; i += 4) if (px[i] !== 0) lit++;
  }
  return lit;
})()`;

/**
 * Play fire/campfire (seed 3) through an EffectManager on the main thread and one on a worker pool, each with a
 * Gfx that records every upload, step both by the same scene times, and compare the uploaded bytes.
 */
const SAME_PIXELS = `(async () => {
  const { EffectManager } = await import("${SRC}/effects.ts");
  const { InlineBackend } = await import("${SRC}/inline.ts");
  const { WorkerPool, openWorkers, workerPort } = await import("${SRC}/pool.ts");
  const engine = window.pixfxDemo.engine;
  const recorder = () => {
    const uploads = [];
    return {
      uploads,
      texture: (key) => ({ upload: (rgba) => uploads.push({ key, bytes: Array.from(rgba) }), destroy() {} }),
      image: () => ({ setPosition() {}, setDepth() {}, destroy() {} }),
    };
  };
  const workers = await openWorkers(1, 1);
  const pool = new WorkerPool(engine, workers.map(workerPort));
  const inline = new InlineBackend(engine);
  const a = recorder();
  const b = recorder();
  const main = new EffectManager(a, () => inline, "a:");
  const off = new EffectManager(b, () => pool, "b:");
  main.play("fire/campfire", 50, 50, { seed: 3 });
  off.play("fire/campfire", 50, 50, { seed: 3 });
  const settle = async (n) => {
    pool.flush();
    const until = performance.now() + 300;
    while (b.uploads.length < n && performance.now() < until) await new Promise((r) => setTimeout(r, 5));
  };
  await settle(4);
  for (let i = 0; i < 12; i++) {
    main.update(83, 1);
    off.update(83, 1);
    await settle(a.uploads.length);
  }
  workers.forEach((w) => w.terminate());
  const strip = (u) => u.map((x) => x.key.slice(2) + ":" + x.bytes.join(","));
  const sa = strip(a.uploads);
  const sb = strip(b.uploads);
  const nonEmpty = a.uploads.filter((u) => u.bytes.some((v) => v !== 0)).length;
  return { uploads: sa.length, workerUploads: sb.length, nonEmpty, equal: sa.length === sb.length && sa.every((s, i) => s === sb[i]) };
})()`;

async function main(): Promise<void> {
  const stopServer = await startServer();
  const b = await launchChrome(CDP_PORT);
  let failed = false;
  const check = (name: string, ok: boolean, detail: string): void => {
    console.log(`${ok ? "PASS" : "FAIL"} ${name}: ${detail}`);
    if (!ok) failed = true;
  };
  try {
    await b.send("Page.enable");
    await b.send("Page.bringToFront");
    await b.send("Page.navigate", { url: `${ROOT}/index.html` });
    await waitFor(b, "window.pixfxDemo !== undefined && window.pixfxDemo.events.includes('create')", 20000);
    await Bun.sleep(1000);
    const stats = await b.evaluate("window.pixfxDemo.stats()");
    const lit = z.number().parse(await b.evaluate(LIT_PIXELS));
    check("demo shows its fires", lit > 0, `${lit} non-transparent pixels in pixfx textures, stats ${JSON.stringify(stats)}`);
    const r1 = await screenshot(b);
    await Bun.sleep(400);
    const r2 = await screenshot(b);
    check("running fires animate", r1 !== r2, "two screenshots 400 ms apart differ");
    await b.evaluate("window.pixfxDemo.scene.scene.pause(), true");
    await Bun.sleep(300);
    const p1 = await screenshot(b);
    await Bun.sleep(600);
    const p2 = await screenshot(b);
    check("pause freezes them", p1 === p2, "two screenshots 600 ms apart while paused are identical");
    const shot = join(tmpdir(), "pixfx-check-paused.png");
    await Bun.write(shot, Buffer.from(p1, "base64"));
    console.log(`paused screenshot: ${shot}`);
    await b.evaluate("window.pixfxDemo.scene.scene.resume(), true");
    const same = z
      .object({ uploads: z.number(), workerUploads: z.number(), nonEmpty: z.number(), equal: z.boolean() })
      .parse(await b.evaluate(SAME_PIXELS));
    check(
      "worker uploads equal main-thread uploads",
      same.equal && same.uploads > 4 && same.nonEmpty > 0,
      `${same.uploads} main-thread uploads (${same.nonEmpty} non-empty), ${same.workerUploads} worker uploads, byte-equal: ${same.equal}`,
    );
  } finally {
    b.close();
    stopServer();
  }
  if (failed) process.exit(1);
}

await main();
