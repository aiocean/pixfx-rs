// `bun run perf:check`: in a clean Chrome, check that the demo (default worker pool) shows its fires, that pausing
// the scene freezes them, and that an effect drawn by a worker uploads exactly the bytes the main thread uploads
// for the same effect, seed and steps. Prints one line per check; exits 1 if any fails.
import { tmpdir } from "node:os";
import { join } from "node:path";
import { z } from "zod";
import type { Browser } from "./cdp";
import { launchChrome, timeoutMessage } from "./cdp";
import { ROOT, startServer } from "./server";

const SRC = `/@fs${join(import.meta.dir, "..", "src")}`;

async function waitFor(b: Browser, expression: string, timeoutMs: number): Promise<void> {
  const until = Date.now() + timeoutMs;
  while (Date.now() < until) {
    if ((await b.evaluate(expression)) === true) return;
    await Bun.sleep(100);
  }
  throw new Error(timeoutMessage(expression, b.takeErrors()));
}

async function screenshot(b: Browser): Promise<string> {
  return z.object({ data: z.string() }).parse(await b.send("Page.captureScreenshot", { format: "png" })).data;
}

/** Lit pixels in the demo's pixfx textures: the atlas pages read back through a framebuffer on WebGL, the canvases on Canvas. */
const LIT_PIXELS = `(() => {
  const s = window.pixfxDemo.scene;
  const gl = s.sys.game.renderer.gl;
  let lit = 0;
  for (const key of s.textures.getTextureKeys()) {
    if (!key.startsWith("pixfx:")) continue;
    const source = s.textures.get(key).source[0];
    let px;
    if (gl && source.glTexture) {
      const fb = gl.createFramebuffer();
      gl.bindFramebuffer(gl.FRAMEBUFFER, fb);
      gl.framebufferTexture2D(gl.FRAMEBUFFER, gl.COLOR_ATTACHMENT0, gl.TEXTURE_2D, source.glTexture.webGLTexture, 0);
      px = new Uint8Array(source.width * source.height * 4);
      gl.readPixels(0, 0, source.width, source.height, gl.RGBA, gl.UNSIGNED_BYTE, px);
      gl.bindFramebuffer(gl.FRAMEBUFFER, null);
      gl.deleteFramebuffer(fb);
    } else {
      const canvas = source.image;
      px = canvas.getContext("2d").getImageData(0, 0, canvas.width, canvas.height).data;
    }
    for (let i = 3; i < px.length; i += 4) if (px[i] !== 0) lit++;
  }
  return lit;
})()`;

/**
 * Play fire/campfire (seed 3) in a game painting on the main thread and in one painting on a real worker, each
 * with textures that record every fill, step both by the same scene times (the worker's game draws again once its
 * frames are back), and compare the filled sizes and bytes.
 */
const SAME_PIXELS = `(async () => {
  const wasm = await import("${SRC}/../wasm/pixfx_wasm.js");
  const { memory } = await wasm.default();
  const { Game } = await import("${SRC}/game.ts");
  const { Stage } = await import("${SRC}/stage.ts");
  const { openWorkers, workerPort } = await import("${SRC}/load.ts");
  const recorder = () => {
    const fills = [];
    const texture = () => ({
      key: "k", color: "c", light: "l",
      fill: (c, l, w, h) => fills.push(w + "x" + h + ":" + Array.from(c).join(",") + "|" + Array.from(l).join(",")),
      destroy() {},
    });
    const image = () => ({ show() {}, hide() {}, destroy() {} });
    return { fills, texture, image };
  };
  const workers = await openWorkers(1, 1);
  const a = recorder();
  const b = recorder();
  const inline = new Game(wasm, memory, []);
  const off = new Game(wasm, memory, workers.map(workerPort));
  inline.useTextures(a.texture);
  off.useTextures(b.texture);
  const sa = new Stage(inline, a.image, () => {});
  const sb = new Stage(off, b.image, () => {});
  sa.play("fire/campfire", 50, 50, { seed: 3 });
  sb.play("fire/campfire", 50, 50, { seed: 3 });
  const step = (game, stage, dt) => { stage.update(dt); stage.draw(); game.endFrame(); };
  for (let i = 0; i < 12; i++) {
    step(inline, sa, 1 / 12);
    step(off, sb, 1 / 12);
    const until = performance.now() + 500;
    while (b.fills.length < a.fills.length && performance.now() < until) await new Promise((r) => setTimeout(r, 5));
    step(off, sb, 0);
  }
  workers.forEach((w) => w.terminate());
  sa.clear();
  sb.clear();
  const nonEmpty = a.fills.filter((f) => /[1-9]/.test(f.split(":")[1])).length;
  return { uploads: a.fills.length, workerUploads: b.fills.length, nonEmpty, equal: a.fills.length === b.fills.length && a.fills.every((f, i) => f === b.fills[i]) };
})()`;

async function main(): Promise<void> {
  const stopServer = await startServer();
  const b = await launchChrome();
  let failed = false;
  const check = (name: string, ok: boolean, detail: string): void => {
    console.log(`${ok ? "PASS" : "FAIL"} ${name}: ${detail}`);
    if (!ok) failed = true;
  };
  try {
    await b.send("Page.enable");
    await b.watchErrors();
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
