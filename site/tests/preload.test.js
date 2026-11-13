// The build step that preloads each page's modules and wasm, run on a copy of the real site the way build.sh runs it.
import { describe, expect, test } from "bun:test";
import { cpSync, existsSync, mkdtempSync, mkdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { preloadPages } from "../preload.js";

const SITE = resolve(import.meta.dir, "..");

/** A dist like build.sh makes: the site's pages and assets, and a wasm-bindgen module that fetches its wasm. */
function makeDist() {
  const dist = mkdtempSync(join(tmpdir(), "pixfx-site-"));
  for (const part of ["index.html", "assets", "docs"]) cpSync(join(SITE, part), join(dist, part), { recursive: true });
  mkdirSync(join(dist, "pkg"));
  writeFileSync(join(dist, "pkg/pixfx_canvas.js"),
    "export default async function init() { const u = new URL('pixfx_canvas_bg.wasm', import.meta.url); }\n");
  writeFileSync(join(dist, "pkg/pixfx_canvas_bg.wasm"), "");
  return dist;
}

/** The preload links in a page's head, as [rel, href]. */
function preloads(file) {
  const head = readFileSync(file, "utf8").split("</head>")[0];
  return [...head.matchAll(/<link rel="(modulepreload|preload)" href="([^"]+)"/g)].map((m) => [m[1], m[2]]);
}

describe("preloadPages", () => {
  test("every page's head preloads its whole module graph and the wasm, and every href is a real file", () => {
    const dist = makeDist();
    try {
      const changed = preloadPages(dist);
      expect(changed.length).toBeGreaterThan(10);
      for (const file of changed) {
        const links = preloads(file);
        const hrefs = links.map(([, href]) => href);
        for (const href of hrefs) expect(existsSync(resolve(dirname(file), href))).toBe(true);
        const names = hrefs.map((h) => h.split("/").pop());
        // the page script, a module only the landing imports, one two levels down, and the engine
        for (const name of ["site.js", "landing.js", "duel.js", "pixfx_canvas.js"]) expect(names).toContain(name);
        expect(links).toContainEqual(["preload", hrefs.find((h) => h.endsWith("pixfx_canvas_bg.wasm"))]);
        expect(new Set(hrefs).size).toBe(hrefs.length);
      }
      expect(preloads(join(dist, "docs/cli.html"))[0]).toEqual(["modulepreload", "../assets/site.js"]);
      expect(preloads(join(dist, "index.html"))[0]).toEqual(["modulepreload", "assets/site.js"]);
    } finally {
      rmSync(dist, { recursive: true, force: true });
    }
  });

  test("the wasm preload is fetched the way wasm-bindgen fetches it, so the browser reuses it", () => {
    const dist = makeDist();
    try {
      preloadPages(dist);
      expect(readFileSync(join(dist, "index.html"), "utf8")).toContain(
        '<link rel="preload" href="pkg/pixfx_canvas_bg.wasm" as="fetch" type="application/wasm" crossorigin>');
    } finally {
      rmSync(dist, { recursive: true, force: true });
    }
  });
});
