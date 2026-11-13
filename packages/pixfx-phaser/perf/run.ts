// `bun run perf`: serve the demo, open the stress page in a clean Chrome, sweep effect counts at 1× and 4× CPU
// throttle (4× is Chrome's mid-range phone setting), with effects drawn on the main thread (`workers=0`) and on
// the default worker pool, profile one load per throttle and pool, and print the tables. Chrome throttles the
// page only, never a worker, so at 4× each worker batch is stretched 4× too (`slowdown=4`).
// Vite serves `src/` live, so run it from a tree no other session is editing (a worktree): a file saved half-written
// stops the page, and a wait that times out prints the page errors it saw.
// Env: PIXFX_CHROME (Chrome binary), PIXFX_PERF_COUNTS (e.g. "10,50,100"), PIXFX_PERF_PROFILE_N (default 100).
import { writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { z } from "zod";
import type { Browser } from "./cdp";
import { launchChrome, timeoutMessage } from "./cdp";
import type { Category } from "./profile";
import { CATEGORIES, categoryTimes, cpuProfileSchema } from "./profile";
import { ROOT, startServer } from "./server";

const COUNTS = (process.env["PIXFX_PERF_COUNTS"] ?? "10,20,25,50,100,200,400").split(",").map(Number).filter((n) => Number.isInteger(n) && n > 0);
const PROFILE_N = Number(process.env["PIXFX_PERF_PROFILE_N"] ?? 100);
const THROTTLES = [1, 4];
/** `workers=0`: drawn on the main thread; the empty query: the default pool. */
const POOLS = [
  { name: "main thread", query: "workers=0" },
  { name: "default worker pool", query: "" },
];
const SCENARIOS = [
  { name: "fires, one object each", query: "mix=mixed&mode=objects" },
  { name: "fires, one object each, shared seeds (identical fires share frames)", query: "mix=mixed&mode=objects&seeds=shared" },
  { name: "fires, one shared layer", query: "mix=mixed&mode=layer" },
  { name: "game-like: half follow sprites, 10 replaced/s", query: "mix=mixed&mode=objects&follow=0.5&churn=10" },
];

const resultSchema = z.object({
  n: z.number(),
  workers: z.number(),
  slowdown: z.number(),
  mode: z.string(),
  mix: z.string(),
  follow: z.number(),
  churn: z.number(),
  seeds: z.string(),
  fps: z.number(),
  cpuMedian: z.number(),
  cpuP95: z.number(),
  frames: z.number(),
  stats: z.object({ engines: z.number(), textures: z.number(), images: z.number() }),
});
type Result = z.infer<typeof resultSchema>;

interface ProfileRow {
  throttle: number;
  pool: string;
  n: number;
  frames: number;
  msPerFrame: Record<Category, number>;
}

/** Poll `expression` until it is true; `guard` runs around each poll and throws to stop the wait early. */
async function waitFor(b: Browser, expression: string, timeoutMs: number, guard: () => void = () => {}): Promise<void> {
  const until = Date.now() + timeoutMs;
  while (Date.now() < until) {
    guard();
    let ok: unknown;
    try {
      ok = await b.evaluate(expression);
    } catch (e) {
      guard(); // a reload mid-evaluate surfaces as a page error; let the guard name the cause
      throw e;
    }
    if (ok === true) return guard();
    await Bun.sleep(250);
  }
  throw new Error(timeoutMessage(expression, b.takeErrors()));
}

/** Throws if the page reloaded since it was opened: its numbers would mix two builds. */
function failIfReloaded(b: Browser): void {
  const reloads = b.takeNavigations();
  if (reloads.length > 0) {
    throw new Error(`perf: the page reloaded during measurement (${reloads.join(", ")}); did vite see a change in src/ or demo/? Run from a tree no one else edits.`);
  }
}

async function open(b: Browser, query: string): Promise<void> {
  b.takeErrors(); // errors of the page being left are not this page's
  await b.send("Page.navigate", { url: `${ROOT}/stress.html?${query}` });
  await Bun.sleep(300);
  // a loaded machine at 4x CPU throttle can take tens of seconds to load the page
  await waitFor(b, "window.pixfxStress !== undefined", 60000);
  b.takeNavigations(); // the navigation that opened it
}

async function sweep(b: Browser, query: string): Promise<Result[]> {
  await open(b, `${query}&sweep=${COUNTS.join(",")}`);
  await waitFor(b, "window.pixfxStress.done === true", COUNTS.length * 8000 + 20000, () => failIfReloaded(b));
  return z.array(resultSchema).parse(await b.evaluate("window.pixfxStress.results"));
}

async function profile(b: Browser, throttle: number, pool: { name: string; query: string }, workerQuery: string): Promise<ProfileRow> {
  await open(b, `n=${PROFILE_N}&mix=mixed&mode=objects&warm=0&measure=600000&${workerQuery}`);
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
  return { throttle, pool: pool.name, n: PROFILE_N, frames, msPerFrame };
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

async function main(): Promise<void> {
  const stopServer = await startServer();
  const b = await launchChrome();
  const out: { throttle: number; pool: string; scenario: string; results: Result[] }[] = [];
  const profiles: ProfileRow[] = [];
  try {
    await b.send("Page.enable");
    await b.watchErrors();
    await b.send("Page.bringToFront");
    console.log(`browser: ${String(await b.evaluate("navigator.userAgent"))}`);
    console.log(`navigator.hardwareConcurrency: ${String(await b.evaluate("navigator.hardwareConcurrency"))}`);
    for (const throttle of THROTTLES) {
      await b.send("Emulation.setCPUThrottlingRate", { rate: throttle });
      for (const pool of POOLS) {
        const workerQuery = [pool.query, pool.query === "workers=0" ? "" : `slowdown=${throttle}`].filter(Boolean).join("&");
        for (const s of SCENARIOS) {
          const results = await sweep(b, `${s.query}&${workerQuery}`);
          out.push({ throttle, pool: pool.name, scenario: s.name, results });
          const workers = results[0]?.workers ?? 0;
          console.log(
            `\n### ${s.name} — CPU ${throttle}×, ${pool.name} (workers=${workers}, slowdown=${results[0]?.slowdown ?? 1})  (60 fps up to ${maxAt(results, 55)}, 30 fps up to ${maxAt(results, 28)})\n${table(results)}`,
          );
        }
        const p = await profile(b, throttle, pool, workerQuery);
        profiles.push(p);
        console.log(`\n### where a frame's main-thread time goes — ${p.n} fires, CPU ${throttle}×, ${pool.name} (${p.frames} frames)`);
        for (const c of CATEGORIES) if (c !== "idle") console.log(`- ${c}: ${p.msPerFrame[c]} ms`);
      }
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
