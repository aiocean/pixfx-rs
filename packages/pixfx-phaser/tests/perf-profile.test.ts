import { describe, expect, test } from "bun:test";
import { categorize, categoryTimes } from "../perf/profile";

describe("perf profile categories", () => {
  test("names each kind of frame work", () => {
    expect(categorize("wasm-function[40]", "http://x/wasm/pixfx_wasm_bg.wasm")).toBe("engine (wasm)");
    expect(categorize("__wbg_new_from_slice", "http://x/wasm/pixfx_wasm.js")).toBe("copy out of wasm");
    expect(categorize("lightToRgba", "http://x/src/pixels.ts")).toBe("pixel convert");
    expect(categorize("texImage2D", "")).toBe("texture upload");
    expect(categorize("draw", "http://x/src/surface.ts")).toBe("pixfx-phaser");
    expect(categorize("render", "http://x/node_modules/.vite/deps/phaser.js?v=1")).toBe("phaser");
    expect(categorize("(idle)", "")).toBe("idle");
  });

  test("sums self time per category in milliseconds", () => {
    const t = categoryTimes({
      nodes: [
        { id: 1, callFrame: { functionName: "lightToRgba", url: "http://x/src/pixels.ts" } },
        { id: 2, callFrame: { functionName: "(idle)", url: "" } },
      ],
      samples: [1, 1, 2],
      timeDeltas: [500, 1500, 3000],
    });
    expect(t["pixel convert"]).toBe(2);
    expect(t.idle).toBe(3);
    expect(t.phaser).toBe(0);
  });
});
