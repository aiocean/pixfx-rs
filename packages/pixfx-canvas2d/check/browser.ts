// `bun run check:browser`: in a clean Chrome, run the demo (painting on workers) and check what a player sees: the
// fires show and animate, a paused loop holds them still, an explosion's cues shake the view, a view away from
// the effects draws none of them, and `destroy()` leaves nothing drawn. Prints one line per check; exits 1 if any fails.
import { spawn } from "node:child_process";
import { join } from "node:path";
import { launchChrome, timeoutMessage } from "pixfx-web/tools/cdp";
import type { Browser } from "pixfx-web/tools/cdp";
import { z } from "zod";

const PORT = 5192;
const ROOT = `http://localhost:${PORT}`;

async function startServer(): Promise<() => void> {
  const vite = spawn("bunx", ["vite", "demo", "--port", String(PORT), "--strictPort"], { cwd: join(import.meta.dir, ".."), stdio: "ignore" });
  for (let i = 0; i < 100; i++) {
    try {
      if ((await fetch(`${ROOT}/index.html`)).ok) return () => vite.kill();
    } catch {
      // Vite is still starting.
    }
    await Bun.sleep(200);
  }
  vite.kill();
  throw new Error(`check: the demo server did not start on port ${PORT}`);
}

async function waitFor(b: Browser, expression: string, timeoutMs: number): Promise<void> {
  const until = Date.now() + timeoutMs;
  while (Date.now() < until) {
    if ((await b.evaluate(expression)) === true) return;
    await Bun.sleep(100);
  }
  throw new Error(timeoutMessage(expression, b.takeErrors()));
}

/** A hash of the canvas, and how many of its pixels are none of the demo's own colours (its ground, walker and hero),
 * so an effect drew them. */
const PICTURE = `(() => {
  const c = document.getElementById("scene");
  const px = c.getContext("2d").getImageData(0, 0, c.width, c.height).data;
  const own = new Set(["23,19,28", "42,34,51", "79,209,197", "242,199,155", "217,79,92", "59,74,140"]);
  let h = 2166136261 >>> 0, lit = 0;
  for (let i = 0; i < px.length; i += 4) {
    h = Math.imul(h ^ px[i] ^ (px[i + 1] << 8) ^ (px[i + 2] << 16), 16777619) >>> 0;
    if (!own.has(px[i] + "," + px[i + 1] + "," + px[i + 2])) lit++;
  }
  return { hash: h, lit };
})()`;

/** The view's shake each frame while an explosion plays out (5.4 s). */
const SHAKE = `(async () => {
  const { pixfx, events } = window.pixfxCanvasDemo;
  let frames = 0, moved = 0, on = true;
  const watch = () => { if (!on) return; frames++; const [x, y] = pixfx.feel.shake; if (x !== 0 || y !== 0) moved++; requestAnimationFrame(watch); };
  requestAnimationFrame(watch);
  pixfx.play("explosions/small-explosion", 168, 120).on("complete", () => events.push("explosion complete"));
  await new Promise((r) => setTimeout(r, 6500));
  on = false;
  const [x, y] = pixfx.feel.shake;
  return { frames, moved, rest: x === 0 && y === 0, complete: events.includes("explosion complete"), cues: events.filter((e) => e.startsWith("cue")).length };
})()`;

const pictureSchema = z.object({ hash: z.number(), lit: z.number() });

async function main(): Promise<void> {
  const stop = await startServer();
  const b = await launchChrome();
  let failed = false;
  const check = (name: string, ok: boolean, detail: string): void => {
    console.log(`${ok ? "PASS" : "FAIL"} ${name}: ${detail}`);
    if (!ok) failed = true;
  };
  const picture = async () => pictureSchema.parse(await b.evaluate(PICTURE));
  try {
    await b.send("Page.enable");
    await b.watchErrors();
    await b.send("Page.bringToFront");
    await b.send("Page.navigate", { url: `${ROOT}/index.html` });
    await waitFor(b, "window.pixfxCanvasDemo !== undefined && window.pixfxCanvasDemo.events.includes('ready')", 30000);
    await Bun.sleep(1500);
    const a = await picture();
    check("the fires show", a.lit > 2000, `${a.lit} pixels drawn by effects`);
    await Bun.sleep(400);
    const a2 = await picture();
    check("they animate", a2.hash !== a.hash, "two pictures 400 ms apart differ");
    await b.evaluate("window.pixfxCanvasDemo.running = false, true");
    await Bun.sleep(300);
    const p1 = await picture();
    await Bun.sleep(500);
    const p2 = await picture();
    check("a paused loop holds them still", p1.hash === p2.hash && p1.lit > 2000, "two pictures 500 ms apart while paused are equal");
    await b.evaluate("window.pixfxCanvasDemo.running = true, true");
    const s = z.object({ frames: z.number(), moved: z.number(), rest: z.boolean(), complete: z.boolean(), cues: z.number() }).parse(await b.evaluate(SHAKE));
    check(
      "an explosion shakes the view, which comes to rest",
      s.moved > 0 && s.rest && s.complete && s.cues > 0,
      `${s.moved}/${s.frames} frames shaken, at rest after: ${s.rest}, ${s.cues} cues, completed: ${s.complete}`,
    );
    await b.evaluate("window.pixfxCanvasDemo.pixfx.setView(5000, 5000, 100, 100), true");
    await Bun.sleep(400);
    const away = await picture();
    await b.evaluate("window.pixfxCanvasDemo.pixfx.clearView(), true");
    await Bun.sleep(400);
    const back = await picture();
    check(
      "a view away from the effects draws none of them; clearing it draws them again",
      away.lit === 0 && back.lit > 2000,
      `${away.lit} effect pixels with the view away, ${back.lit} after clearView`,
    );
    const d = z
      .object({ stats: z.object({ engines: z.number(), images: z.number() }).passthrough(), candle: z.boolean() })
      .parse(await b.evaluate(`(() => { const { pixfx, events } = window.pixfxCanvasDemo; pixfx.destroy(); return { stats: pixfx.stats(), candle: events.includes("candle destroy") }; })()`));
    await Bun.sleep(300);
    const after = await picture();
    check(
      "destroy() leaves nothing drawn",
      d.stats.engines === 0 && d.stats.images === 0 && d.candle && after.lit === 0,
      `stats ${JSON.stringify(d.stats)}; candle destroyed: ${d.candle}; ${after.lit} effect pixels after`,
    );
    const errors = b.takeErrors().filter((e) => !e.includes("favicon"));
    check("no page errors", errors.length === 0, JSON.stringify(errors));
  } finally {
    b.close();
    stop();
  }
  if (failed) process.exit(1);
}

await main();
