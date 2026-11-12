// Serve the demo directory with Vite for the browser runs.
import { spawn } from "node:child_process";
import { join } from "node:path";

const PORT = 5181;
export const ROOT = `http://localhost:${PORT}`;

/** Start Vite on the demo and wait until it answers; returns the stop function. */
export async function startServer(): Promise<() => void> {
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
