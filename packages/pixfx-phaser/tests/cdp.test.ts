import { describe, expect, test } from "bun:test";
import { devToolsPort, pageError, timeoutMessage } from "../perf/cdp";

describe("pageError", () => {
  test("an uncaught exception gives its description", () => {
    const params = { exceptionDetails: { text: "Uncaught", exception: { description: "SyntaxError: Unexpected token ';'" } } };
    expect(pageError("Runtime.exceptionThrown", params)).toBe("SyntaxError: Unexpected token ';'");
  });

  test("an exception without a description gives its text", () => {
    expect(pageError("Runtime.exceptionThrown", { exceptionDetails: { text: "Uncaught (in promise)" } })).toBe("Uncaught (in promise)");
  });

  test("console.error gives its arguments joined", () => {
    const params = { type: "error", args: [{ value: "[vite] Internal server error:" }, { description: "Transform failed" }] };
    expect(pageError("Runtime.consoleAPICalled", params)).toBe("[vite] Internal server error: Transform failed");
  });

  test("console.log is not an error", () => {
    expect(pageError("Runtime.consoleAPICalled", { type: "log", args: [{ value: "hi" }] })).toBeNull();
  });

  test("a browser log entry at error level gives its text and url", () => {
    const params = { entry: { level: "error", text: "Failed to load resource: 500", url: "http://localhost/src/plugin.ts" } };
    expect(pageError("Log.entryAdded", params)).toBe("Failed to load resource: 500 (http://localhost/src/plugin.ts)");
  });

  test("a browser log entry below error level is not an error", () => {
    expect(pageError("Log.entryAdded", { entry: { level: "warning", text: "slow" } })).toBeNull();
  });

  test("other events and malformed params are not errors", () => {
    expect(pageError("Page.loadEventFired", { timestamp: 1 })).toBeNull();
    expect(pageError("Runtime.exceptionThrown", { nope: true })).toBeNull();
  });
});

describe("timeoutMessage", () => {
  test("names the page errors seen", () => {
    expect(timeoutMessage("window.x", ["SyntaxError: a", "Failed to load resource: 500"])).toBe(
      "perf: timed out waiting for window.x; page errors:\n  SyntaxError: a\n  Failed to load resource: 500",
    );
  });

  test("says when the page reported none", () => {
    expect(timeoutMessage("window.x", [])).toBe("perf: timed out waiting for window.x; the page reported no errors");
  });
});

describe("devToolsPort", () => {
  test("reads the port Chrome wrote on the first line", () => {
    expect(devToolsPort("53117\n/devtools/browser/0b1c\n")).toBe(53117);
  });

  test("a file Chrome has not finished writing gives no port", () => {
    expect(devToolsPort("")).toBeNull();
    expect(devToolsPort("531")).toBeNull();
    expect(devToolsPort("abc\n/devtools/browser/x\n")).toBeNull();
    expect(devToolsPort("0\n/devtools/browser/x\n")).toBeNull();
  });
});
