// A minimal Chrome DevTools Protocol client: launch Chrome with a throwaway profile and drive its first page.
import { spawn } from "node:child_process";
import { mkdtempSync, readFileSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { z } from "zod";

const DEFAULT_CHROME = "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome";

const targetsSchema = z.array(z.object({ type: z.string(), webSocketDebuggerUrl: z.string().optional() }));
const replySchema = z.object({
  id: z.number().optional(),
  result: z.unknown().optional(),
  error: z.object({ message: z.string() }).optional(),
});
const evaluateSchema = z.object({
  result: z.object({ value: z.unknown().optional() }),
  exceptionDetails: z.unknown().optional(),
});

const eventSchema = z.object({ method: z.string(), params: z.unknown() });
const exceptionSchema = z.object({
  exceptionDetails: z.object({ text: z.string(), exception: z.object({ description: z.string().optional() }).optional() }),
});
const consoleSchema = z.object({
  type: z.string(),
  args: z.array(z.object({ value: z.unknown().optional(), description: z.string().optional() })),
});
const logSchema = z.object({ entry: z.object({ level: z.string(), text: z.string(), url: z.string().optional() }) });

/** The error a CDP event reports from the page (an uncaught exception, `console.error`, a failed load), or null. */
export function pageError(method: string, params: unknown): string | null {
  if (method === "Runtime.exceptionThrown") {
    const p = exceptionSchema.safeParse(params);
    return p.success ? (p.data.exceptionDetails.exception?.description ?? p.data.exceptionDetails.text) : null;
  }
  if (method === "Runtime.consoleAPICalled") {
    const p = consoleSchema.safeParse(params);
    if (!p.success || p.data.type !== "error") return null;
    return p.data.args.map((a) => a.description ?? String(a.value)).join(" ");
  }
  if (method === "Log.entryAdded") {
    const p = logSchema.safeParse(params);
    if (!p.success || p.data.entry.level !== "error") return null;
    return p.data.entry.url === undefined ? p.data.entry.text : `${p.data.entry.text} (${p.data.entry.url})`;
  }
  return null;
}

/** What a wait that timed out says: the expression, and the page errors seen while waiting. */
export function timeoutMessage(expression: string, errors: readonly string[]): string {
  const head = `perf: timed out waiting for ${expression}`;
  return errors.length === 0 ? `${head}; the page reported no errors` : `${head}; page errors:\n${errors.map((e) => `  ${e}`).join("\n")}`;
}

/**
 * The debugging port Chrome picked, from the `DevToolsActivePort` file it writes in its profile (the port, a newline,
 * the browser's socket path), or null while the file is missing or half written.
 */
export function devToolsPort(text: string): number | null {
  const lines = text.split("\n");
  if (lines.length < 2) return null;
  const port = Number(lines[0]);
  return Number.isInteger(port) && port > 0 && port < 65536 ? port : null;
}

function readPort(profile: string): number | null {
  try {
    return devToolsPort(readFileSync(join(profile, "DevToolsActivePort"), "utf8"));
  } catch {
    return null; // not written yet
  }
}

export interface Browser {
  /** Send one CDP command to the page; resolves with its `result`. */
  send(method: string, params?: Record<string, unknown>): Promise<unknown>;
  /** Evaluate `expression` in the page (promises awaited) and return its JSON value. */
  evaluate(expression: string): Promise<unknown>;
  /** Page errors seen since the last call ([`pageError`]); the list is empty afterwards. Needs `watchErrors()`. */
  takeErrors(): string[];
  /** Start collecting page errors (enables the Runtime and Log domains). */
  watchErrors(): Promise<void>;
  close(): void;
}

/**
 * Launch Chrome (`PIXFX_CHROME` or the default macOS path) and connect to its page over CDP. Chrome picks a free
 * debugging port and writes it in its own profile, so runs started at the same time (another session's) each get
 * their own browser instead of sharing one through a fixed port.
 */
export async function launchChrome(): Promise<Browser> {
  const profile = mkdtempSync(join(tmpdir(), "pixfx-perf-chrome-"));
  const proc = spawn(
    process.env["PIXFX_CHROME"] ?? DEFAULT_CHROME,
    [
      "--remote-debugging-port=0",
      `--user-data-dir=${profile}`,
      "--no-first-run",
      "--no-default-browser-check",
      // A measured page must keep full frame rate even when another window covers it.
      "--disable-backgrounding-occluded-windows",
      "--disable-renderer-backgrounding",
      "--disable-background-timer-throttling",
      "--window-size=1400,900",
      "about:blank",
    ],
    { stdio: "ignore" },
  );
  let url: string | undefined;
  let port: number | null = null;
  for (let i = 0; i < 100 && !url; i++) {
    port ??= readPort(profile);
    if (port !== null) {
      try {
        const targets = targetsSchema.parse(await (await fetch(`http://127.0.0.1:${port}/json`)).json());
        url = targets.find((t) => t.type === "page")?.webSocketDebuggerUrl;
      } catch {
        // Chrome is still starting.
      }
    }
    if (!url) await Bun.sleep(100);
  }
  if (!url) {
    proc.kill();
    throw new Error(port === null ? "perf: Chrome did not write its debugging port" : `perf: Chrome did not open a debugging page on port ${port}`);
  }
  const ws = new WebSocket(url);
  await new Promise<void>((resolve, reject) => {
    ws.addEventListener("open", () => resolve(), { once: true });
    ws.addEventListener("error", () => reject(new Error("perf: CDP socket failed to open")), { once: true });
  });
  let nextId = 0;
  const pending = new Map<number, { resolve: (v: unknown) => void; reject: (e: Error) => void; method: string }>();
  let errors: string[] = [];
  ws.addEventListener("message", (e) => {
    const data: unknown = JSON.parse(String(e.data));
    const event = eventSchema.safeParse(data);
    if (event.success) {
      const error = pageError(event.data.method, event.data.params);
      if (error !== null) errors.push(error);
      return;
    }
    const reply = replySchema.safeParse(data);
    if (!reply.success || reply.data.id === undefined) return;
    const p = pending.get(reply.data.id);
    if (!p) return;
    pending.delete(reply.data.id);
    if (reply.data.error) p.reject(new Error(`${p.method}: ${reply.data.error.message}`));
    else p.resolve(reply.data.result);
  });
  const send = (method: string, params: Record<string, unknown> = {}): Promise<unknown> =>
    new Promise((resolve, reject) => {
      const id = ++nextId;
      pending.set(id, { resolve, reject, method });
      ws.send(JSON.stringify({ id, method, params }));
    });
  return {
    send,
    async evaluate(expression) {
      const r = evaluateSchema.parse(await send("Runtime.evaluate", { expression, awaitPromise: true, returnByValue: true }));
      if (r.exceptionDetails !== undefined) throw new Error(`perf: page threw: ${JSON.stringify(r.exceptionDetails)}`);
      return r.result.value;
    },
    takeErrors() {
      const taken = errors;
      errors = [];
      return taken;
    },
    async watchErrors() {
      await send("Runtime.enable");
      await send("Log.enable");
    },
    close() {
      ws.close();
      proc.kill();
      rmSync(profile, { recursive: true, force: true });
    },
  };
}
