import { fileURLToPath } from "node:url";
import { defineConfig } from "vite";

// pixfx-web loads its wasm and starts its worker through `new URL(..., import.meta.url)`, which pre-bundling would
// break: Vite serves it as source. In this repo it resolves to `packages/pixfx-web`, outside the demo, so the
// packages folder is served too.
export default defineConfig({
  optimizeDeps: { exclude: ["pixfx-web"] },
  server: { fs: { allow: [fileURLToPath(new URL("../..", import.meta.url))] } },
});
