import { fileURLToPath } from "node:url";
import { defineConfig } from "vite";

// `bun run build`: the shell as one ES module, `dist/pixfx-canvas2d.js`, with its painting worker and the engine's
// wasm as files beside it, for pages that load it without a bundler (the site). Not library mode: that would inline
// the wasm into the module and the worker, so a page would download it twice, as base64. pixfx-web lives outside
// this package, so the packages folder is allowed.
export default defineConfig({
  // assets are found beside the module, wherever a page serves it from
  base: "./",
  build: {
    target: "es2022",
    rollupOptions: {
      input: { "pixfx-canvas2d": "src/index.ts" },
      preserveEntrySignatures: "strict",
      output: { entryFileNames: "[name].js" },
    },
  },
  worker: { format: "es" },
  server: { fs: { allow: [fileURLToPath(new URL("..", import.meta.url))] } },
});
