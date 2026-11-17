// `bun run check:browser`: in a clean Chrome, run the demo (painting on workers) and check what a player sees: the
// effects show and animate, stopping the ticker freezes them, a body burst's cues shake the camera and the camera
// comes back, a view away from the effects draws none of them, and `destroy()` leaves no sprite behind. Prints one line per check; exits 1 if any fails.
import { spawn } from "node:child_process";
import { join } from "node:path";
import { launchChrome, timeoutMessage } from "pixfx-web/tools/cdp";
import type { Browser } from "pixfx-web/tools/cdp";
import { z } from "zod";

const PORT = 5191;
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

/** A hash of the stage as rendered now, and how many of its pixels differ from the same render without effects. */
const PICTURE = `(() => {
  const { app, pixfx } = window.pixfxPixiDemo;
  const hash = (px) => { let h = 2166136261 >>> 0; for (let i = 0; i < px.length; i++) h = Math.imul(h ^ px[i], 16777619) >>> 0; return h; };
  const shot = () => app.renderer.extract.pixels({ target: app.stage }).pixels;
  const withFx = shot();
  pixfx.container.visible = false;
  const without = shot();
  pixfx.container.visible = true;
  let lit = 0;
  for (let i = 0; i < withFx.length; i += 4) if (withFx[i] !== without[i] || withFx[i + 1] !== without[i + 1] || withFx[i + 2] !== without[i + 2]) lit++;
  return { hash: hash(withFx), lit };
})()`;

/** Where the camera sits before the step and as it is rendered, while a body burst plays out (3 s). */
const SHAKE = `(async () => {
  const { app, pixfx, world, events } = window.pixfxPixiDemo;
  const rest = [];
  const shown = [];
  const before = () => rest.push([world.x, world.y, world.rotation]);
  const during = () => shown.push([world.x, world.y, world.rotation]);
  app.ticker.add(before, undefined, 10);
  app.ticker.add(during, undefined, -20);
  pixfx.play("explosions/body-burst", 168, 120).on("complete", () => events.push("burst complete"));
  await new Promise((r) => setTimeout(r, 4500));
  app.ticker.remove(before);
  app.ticker.remove(during);
  const moved = shown.filter(([x, y, r]) => x !== 0 || y !== 0 || r !== 0).length;
  const still = rest.every(([x, y, r]) => x === 0 && y === 0 && r === 0);
  return { frames: shown.length, moved, still, complete: events.includes("burst complete"), cues: events.filter((e) => e.startsWith("cue")).length };
})()`;

const DESTROY = `(() => {
  const { pixfx, events } = window.pixfxPixiDemo;
  const before = pixfx.container.children.length;
  pixfx.destroy();
  return { before, after: pixfx.container.children.length, stats: pixfx.stats(), outline: events.includes("outline destroy") };
})()`;

/** How many pixels of the screen the effects' sprites change (the game's own figures in the container left in). */
const EFFECT_PIXELS = `(() => {
  const { app, pixfx } = window.pixfxPixiDemo;
  const shot = () => app.renderer.extract.pixels({ target: app.stage, frame: app.screen }).pixels;
  const sprites = pixfx.container.children.filter((c) => c.constructor.name === "Sprite" && c.visible);
  const withFx = shot();
  for (const c of sprites) c.visible = false;
  const without = shot();
  for (const c of sprites) c.visible = true;
  let lit = 0;
  for (let i = 0; i < withFx.length; i += 4) if (withFx[i] !== without[i] || withFx[i + 1] !== without[i + 1] || withFx[i + 2] !== without[i + 2]) lit++;
  return lit;
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
  try {
    await b.send("Page.enable");
    await b.watchErrors();
    await b.send("Page.bringToFront");
    await b.send("Page.navigate", { url: `${ROOT}/index.html` });
    await waitFor(b, "window.pixfxPixiDemo !== undefined && window.pixfxPixiDemo.events.includes('ready')", 30000);
    await Bun.sleep(1500);
    const a = pictureSchema.parse(await b.evaluate(PICTURE));
    check("the effects show", a.lit > 2000, `${a.lit} pixels change when the effects are hidden`);
    await Bun.sleep(400);
    const a2 = pictureSchema.parse(await b.evaluate(PICTURE));
    check("they animate", a2.hash !== a.hash, "two renders 400 ms apart differ");
    await b.evaluate("window.pixfxPixiDemo.app.ticker.stop(), true");
    await Bun.sleep(300);
    const p1 = pictureSchema.parse(await b.evaluate(PICTURE));
    await Bun.sleep(500);
    const p2 = pictureSchema.parse(await b.evaluate(PICTURE));
    check("stopping the ticker freezes them", p1.hash === p2.hash && p1.lit > 2000, "two renders 500 ms apart while stopped are equal");
    await b.evaluate("window.pixfxPixiDemo.app.ticker.start(), true");
    const s = z
      .object({ frames: z.number(), moved: z.number(), still: z.boolean(), complete: z.boolean(), cues: z.number() })
      .parse(await b.evaluate(SHAKE));
    check(
      "a body burst shakes the camera, which comes back",
      s.moved > 0 && s.still && s.complete && s.cues > 0,
      `${s.moved}/${s.frames} rendered frames moved, camera at rest before every step: ${s.still}, ${s.cues} cues, completed: ${s.complete}`,
    );
    await b.evaluate("window.pixfxPixiDemo.pixfx.setView(5000, 5000, 100, 100), true");
    await Bun.sleep(400);
    const away = z.number().parse(await b.evaluate(EFFECT_PIXELS));
    await b.evaluate("window.pixfxPixiDemo.pixfx.clearView(), true");
    await Bun.sleep(400);
    const back = z.number().parse(await b.evaluate(EFFECT_PIXELS));
    check(
      "a view away from the effects draws none of them; clearing it draws them again",
      away === 0 && back > 2000,
      `${away} pixels drawn by effects with the view away, ${back} after clearView`,
    );
    const d = z
      .object({ before: z.number(), after: z.number(), stats: z.object({ engines: z.number(), images: z.number() }).passthrough(), outline: z.boolean() })
      .parse(await b.evaluate(DESTROY));
    check(
      "destroy() leaves only the game's own figures",
      d.after === 1 && d.stats.engines === 0 && d.stats.images === 0 && d.outline,
      `${d.before} children before, ${d.after} after; stats ${JSON.stringify(d.stats)}; outline destroyed: ${d.outline}`,
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
