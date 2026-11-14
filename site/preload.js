// Run by build.sh on site/dist: gives every page's <head> a <link rel="modulepreload"> for each module its page
// script imports, and a preload of the wasm engine those modules fetch. Without them the browser finds the modules
// one import level at a time and the wasm last, a network round trip per level before anything draws.
// The list comes from the import graph itself, so a new module needs no change here.
// Usage: bun site/preload.js site/dist
import { readFileSync, readdirSync, writeFileSync } from "node:fs";
import { dirname, join, relative, resolve, sep } from "node:path";

/** The relative specifiers `source` imports statically (`import x from "./a.js"`, `import "./b.js"`). */
export function imports(source) {
  const found = source.matchAll(/^\s*import\s+(?:[^;]*?\sfrom\s*)?["'](\.{1,2}\/[^"']+)["']/gm);
  return [...found].map((m) => m[1]);
}

/** The wasm file a wasm-bindgen module fetches beside itself (a bundler writes the path as a template literal), or null. */
export function wasmOf(source) {
  return source.match(/new URL\(\s*["'`]([^"'`]+\.wasm)["'`]\s*,\s*import\.meta\.url\s*\)/)?.[1] ?? null;
}

/**
 * Every module `entry` reaches by static imports, entry first, each once; and the wasm files they fetch.
 * @param {string} entry a module's path
 * @param {(path: string) => string} read a module's source
 */
export function moduleGraph(entry, read) {
  const modules = [];
  const wasm = [];
  const queue = [entry];
  while (queue.length) {
    const path = queue.shift();
    if (modules.includes(path)) continue;
    modules.push(path);
    const source = read(path);
    for (const spec of imports(source)) queue.push(join(dirname(path), spec));
    const bin = wasmOf(source);
    if (bin) wasm.push(join(dirname(path), bin));
  }
  return { modules, wasm };
}

/** `html` with preload links for `modules` and `wasm` (hrefs as the page should write them) at the end of its head. */
export function withPreloads(html, { modules, wasm }) {
  const links = [
    ...modules.map((href) => `  <link rel="modulepreload" href="${href}">`),
    // as=fetch with crossorigin matches wasm-bindgen's own fetch(), so the response is reused, not fetched twice
    ...wasm.map((href) => `  <link rel="preload" href="${href}" as="fetch" type="application/wasm" crossorigin>`),
  ];
  return html.replace("</head>", `${links.join("\n")}\n</head>`);
}

/** Add the preloads to every HTML page under `dist` that has a module script; returns the pages changed. */
export function preloadPages(dist) {
  const changed = [];
  for (const file of htmlFiles(dist)) {
    const html = readFileSync(file, "utf8");
    const src = html.match(/<script type="module" src="([^"]+)"/)?.[1];
    if (!src) continue;
    const dir = dirname(file);
    const graph = moduleGraph(resolve(dir, src), (path) => readFileSync(path, "utf8"));
    const href = (path) => relative(dir, path).split(sep).join("/");
    writeFileSync(file, withPreloads(html, { modules: graph.modules.map(href), wasm: graph.wasm.map(href) }));
    changed.push(file);
  }
  return changed;
}

function htmlFiles(dir) {
  return readdirSync(dir, { withFileTypes: true, recursive: true })
    .filter((e) => e.isFile() && e.name.endsWith(".html"))
    .map((e) => join(e.parentPath, e.name));
}

if (import.meta.main) {
  const dist = process.argv[2];
  if (!dist) {
    console.error("usage: preload.js <dist dir>");
    process.exit(2);
  }
  console.log(`preload: ${preloadPages(dist).length} pages`);
}
