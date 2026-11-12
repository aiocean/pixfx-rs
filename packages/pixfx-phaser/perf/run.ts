// `bun run perf`: serve the demo, open the stress page in a clean Chrome, sweep effect counts at 1× and 4× CPU
// throttle (4× is Chrome's mid-range phone setting), profile one load per throttle, and print the tables.
// Env: PIXFX_CHROME (Chrome binary), PIXFX_PERF_COUNTS (e.g. "10,50,100"), PIXFX_PERF_PROFILE_N (default 100).
import { spawn } from "node:child_process";
import { writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { z } from "zod";
import type { Browser } from "./cdp";
import { launchChrome } from "./cdp";
import type { Category } from "./profile";
import { CATEGORIES, categoryTimes, cpuProfileSchema } from "./profile";

const PORT = 5181;
const CDP_PORT = 9333;
const ROOT = `http://localhost:${PORT}`;
const COUNTS = (process.env["PIXFX_PERF_COUNTS"] ?? "10,25,50,100,200,400").split(",").map(Number).filter((n) => Number.isInteger(n) && n > 0);
const PROFILE_N = Number(process.env["PIXFX_PERF_PROFILE_N"] ?? 100);
const THROTTLES = [1, 4];
const SCENARIOS = [
  { name: "fires, one object each", query: "mix=mixed&mode=objects" },
  { name: "fires, one shared layer", query: "mix=mixed&mode=layer" },
  { name: "game-like: half follow sprites, 10 replaced/s", query: "mix=mixed&mode=objects&follow=0.5&churn=10" },
];

const resultSchema = z.object({
  n: z.number(),
  mode: z.string(),
  mix: z.string(),
  follow: z.number(),
  churn: z.number(),
  fps: z.number(),
  cpuMedian: z.number(),
  cpuP95: z.number(),
  frames: z.number(),
  stats: z.object({ engines: z.number(), textures: z.number(), images: z.number() }),
});
type Result = z.infer<typeof resultSchema>;

interface ProfileRow {
  throttle: number;
  n: number;
  frames: number;
  msPerFrame: Record<Category, number>;
}

async function waitFor(b: Browser, expression: string, timeoutMs: number): Promise<void> {
  const until = Date.now() + timeoutMs;
  while (Date.now() < until) {
    if ((await b.evaluate(expression)) === true) return;
    await Bun.sleep(250);
  }
  throw new Error(`perf: timed out waiting for ${expression}`);
}

async function open(b: Browser, query: string): Promise<void> {
  await b.send("Page.navigate", { url: `${ROOT}/stress.html?${query}` });
  await Bun.sleep(300);
  await waitFor(b, "window.pixfxStress !== undefined", 20000);
}

async function sweep(b: Browser, query: string): Promise<Result[]> {
  await open(b, `${query}&sweep=${COUNTS.join(",")}`);
  await waitFor(b, "window.pixfxStress.done === true", COUNTS.length * 8000 + 20000);
  return z.array(resultSchema).parse(await b.evaluate("window.pixfxStress.results"));
}

async function profile(b: Browser, throttle: number): Promise<ProfileRow> {
  await open(b, `n=${PROFILE_N}&mix=mixed&mode=objects&warm=0&measure=600000`);
  await Bun.sleep(1500);
  const frame = (): Promise<number> => b.evaluate("window.pixfxStress.game.loop.frame").then((v) => z.number().parse(v));
  await b.send("Profiler.enable");
  await b.send("Profiler.setSamplingInterval", { interval: 200 });
  const f0 = await frame();
  await b.send("Profiler.start");
  await Bun.sleep(4000);
  const stopped = z.object({ profile: cpuProfileSchema }).parse(await b.send("Profiler.stop"));
  const frames = Math.max(1, (await frame()) - f0);
  const times = categoryTimes(stopped.profile);
  const msPerFrame = { ...times };
  for (const c of CATEGORIES) msPerFrame[c] = Math.round((times[c] / frames) * 100) / 100;
  return { throttle, n: PROFILE_N, frames, msPerFrame };
}

/** Largest count whose measured fps reached `fps`. */
function maxAt(rs: Result[], fps: number): string {
  const ok = rs.filter((r) => r.fps >= fps).map((r) => r.n);
  return ok.length > 0 ? String(Math.max(...ok)) : `< ${Math.min(...rs.map((r) => r.n))}`;
}

function table(rs: Result[]): string {
  const lines = ["| effects | fps | frame cpu median ms | p95 ms | textures |", "|---:|---:|---:|---:|---:|"];
  for (const r of rs) lines.push(`| ${r.n} | ${r.fps} | ${r.cpuMedian} | ${r.cpuP95} | ${r.stats.textures} |`);
  return lines.join("\n");
}

async function startServer(): Promise<() => void> {
  const vite = spawn("bunx", ["vite", "demo", "--port", String(PORT), "--strictPort"], { cwd: join(import.meta.dir, ".."), stdio: "ignore" });
  for (let i = 0; i < 100; i++) {
    try {
      if ((await fetch(`${ROOT}/stress.html`)).ok) return () => vite.kill();
    } catch {
      // Vite is still starting.
    }
    await Bun.sleep(200);
  }
  vite.kill();
  throw new Error(`perf: the demo server did not start on port ${PORT}`);
}

async function main(): Promise<void> {
  const stopServer = await startServer();
  const b = await launchChrome(CDP_PORT);
  const out: { throttle: number; scenario: string; results: Result[] }[] = [];
  const profiles: ProfileRow[] = [];
  try {
    await b.send("Page.enable");
    await b.send("Page.bringToFront");
    console.log(`browser: ${String(await b.evaluate("navigator.userAgent"))}`);
    for (const throttle of THROTTLES) {
      await b.send("Emulation.setCPUThrottlingRate", { rate: throttle });
      for (const s of SCENARIOS) {
        const results = await sweep(b, s.query);
        out.push({ throttle, scenario: s.name, results });
        console.log(`\n### ${s.name} — CPU ${throttle}×  (60 fps up to ${maxAt(results, 55)}, 30 fps up to ${maxAt(results, 28)})\n${table(results)}`);
      }
      const p = await profile(b, throttle);
      profiles.push(p);
      console.log(`\n### where a frame's main-thread time goes — ${p.n} fires, CPU ${throttle}× (${p.frames} frames)`);
      for (const c of CATEGORIES) if (c !== "idle") console.log(`- ${c}: ${p.msPerFrame[c]} ms`);
    }
  } finally {
    b.close();
    stopServer();
  }
  const file = join(tmpdir(), `pixfx-perf-${Date.now()}.json`);
  writeFileSync(file, JSON.stringify({ sweeps: out, profiles }, null, 2));
  console.log(`\nraw results: ${file}`);
}

await main();
