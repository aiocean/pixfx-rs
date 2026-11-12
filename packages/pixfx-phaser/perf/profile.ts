// Turn a V8 CPU profile into time per category, so a frame's cost reads as "engine / convert / upload / Phaser".
import { z } from "zod";

export const cpuProfileSchema = z.object({
  nodes: z.array(z.object({ id: z.number(), callFrame: z.object({ functionName: z.string(), url: z.string() }) })),
  samples: z.array(z.number()),
  timeDeltas: z.array(z.number()),
});

export type CpuProfile = z.infer<typeof cpuProfileSchema>;

export const CATEGORIES = ["engine (wasm)", "copy out of wasm", "pixel convert", "texture upload", "pixfx-phaser", "phaser", "gc", "browser (program)", "idle", "other"] as const;
export type Category = (typeof CATEGORIES)[number];

const UPLOAD = new Set(["putImageData", "getImageData", "texImage2D", "texSubImage2D", "drawImage"]);

/** Which category one profiled function belongs to. */
export function categorize(functionName: string, url: string): Category {
  if (functionName === "(idle)") return "idle";
  if (functionName === "(garbage collector)") return "gc";
  if (functionName === "(program)") return "browser (program)";
  if (url.endsWith(".wasm")) return "engine (wasm)";
  if (url.includes("pixfx_wasm.js")) return "copy out of wasm";
  if (url.includes("/src/pixels.ts")) return "pixel convert";
  if (UPLOAD.has(functionName)) return "texture upload";
  if (url.includes("/src/")) return "pixfx-phaser";
  if (url.includes("phaser")) return "phaser";
  return "other";
}

/** Self time per category in milliseconds. */
export function categoryTimes(p: CpuProfile): Record<Category, number> {
  const byId = new Map(p.nodes.map((n) => [n.id, n.callFrame]));
  const out: Record<Category, number> = {
    "engine (wasm)": 0,
    "copy out of wasm": 0,
    "pixel convert": 0,
    "texture upload": 0,
    "pixfx-phaser": 0,
    phaser: 0,
    gc: 0,
    "browser (program)": 0,
    idle: 0,
    other: 0,
  };
  p.samples.forEach((id, i) => {
    const frame = byId.get(id);
    const c = frame ? categorize(frame.functionName, frame.url) : "other";
    out[c] += (p.timeDeltas[i] ?? 0) / 1000;
  });
  return out;
}
