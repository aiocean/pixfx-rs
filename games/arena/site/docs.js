// Run by build.sh: turns the player docs (games/arena/docs/*.md, the source of truth) into the site's docs pages, each
// with the menu, an "on this page" list and previous/next links written in, so a docs page needs no script to read.
// Usage: bun games/arena/site/docs.js <docs dir> <out dir>
import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";

/** The docs in reading order: file name (without .md) and menu label. */
export const DOCS = [
  ["rules", "Rules"],
  ["bot", "The bot"],
  ["training", "Training"],
  ["tournament", "Tournament"],
  ["roadmap", "Roadmap"],
];

/** A heading's id, from its text. */
export const slug = (text) => text.toLowerCase().replace(/<[^>]+>/g, "").replace(/&[a-z]+;/g, "").replace(/[^a-z0-9]+/g, "-").replace(/^-|-$/g, "");

const escape = (s) => s.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;").replace(/"/g, "&quot;");

/**
 * One doc as a page. `markdown` is the doc's source; its first `# ` heading is the page title and its first
 * paragraph the description. Links to another doc (`bot.md#x`) become links to its page.
 */
export function docPage(name, markdown) {
  const i = DOCS.findIndex(([n]) => n === name);
  const title = markdown.match(/^# (.+)$/m)?.[1] ?? name;
  const lead = markdown.split(/\n\s*\n/).find((p) => p.trim() && !p.startsWith("#"))?.replace(/\s+/g, " ").replace(/`/g, "").trim() ?? "";
  const toc = [];
  const body = Bun.markdown
    .html(markdown)
    .replace(/<(h[23])>(.*?)<\/\1>/g, (_, tag, text) => {
      const id = slug(text);
      if (tag === "h2") toc.push([id, text]);
      return `<${tag} id="${id}">${text}</${tag}>`;
    })
    .replace(/href="(?:\.\/)?([a-z-]+)\.md(#[^"]*)?"/g, (m, doc, hash = "") => (DOCS.some(([n]) => n === doc) ? `href="${doc}.html${hash}"` : m))
    .replace(/<table>/g, '<div class="table-wrap"><table>')
    .replace(/<\/table>/g, "</table></div>");
  const menu = DOCS.map(([n, label]) => `<li><a href="${n}.html"${n === name ? ' aria-current="page"' : ""}>${label}</a></li>`).join("");
  const pager = (d, dir) => (d ? `<a href="${d[0]}.html"${dir === "Next" ? ' style="text-align:right;margin-left:auto"' : ""}><small>${dir}</small>${d[1]}</a>` : "");
  return `<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>${escape(title)} · Arena docs</title>
  <meta name="description" content="${escape(lead)}">
  <link rel="preconnect" href="https://fonts.googleapis.com">
  <link rel="preconnect" href="https://fonts.gstatic.com" crossorigin>
  <link rel="stylesheet" href="https://fonts.googleapis.com/css2?family=IBM+Plex+Mono:wght@400;500&family=IBM+Plex+Sans:wght@400;500;600&family=Pixelify+Sans:wght@500&display=swap">
  <link rel="stylesheet" href="../assets/site.css">
  <link rel="stylesheet" href="../assets/arena.css">
</head>
<body class="docs-page arena">
  <header class="topbar">
    <a class="brand" href="../">arena</a>
    <nav aria-label="Site">
      <a href="../watch.html">Watch</a>
      <a href="rules.html"${name === "rules" ? ' aria-current="page"' : ""}>Rules</a>
      <a href="training.html"${name === "training" ? ' aria-current="page"' : ""}>Train a bot</a>
      <a href="tournament.html"${name === "tournament" ? ' aria-current="page"' : ""}>Tournament</a>
    </nav>
    <button class="menu-toggle" type="button" aria-expanded="false" aria-controls="sidebar">Menu</button>
  </header>
  <div class="docs-shell">
    <nav class="sidebar" id="sidebar" aria-label="Docs"><h4>For bot authors</h4><ul>${menu}</ul></nav>
    <main class="doc">
${body}
<nav class="pager" aria-label="Pages">${pager(DOCS[i - 1], "Previous")}${pager(DOCS[i + 1], "Next")}</nav>
    </main>
    <aside class="toc">${toc.length > 1 ? `<h4>On this page</h4><ul>${toc.map(([id, t]) => `<li><a href="#${id}">${t}</a></li>`).join("")}</ul>` : ""}</aside>
  </div>
  <script type="module" src="../assets/menu.js"></script>
</body>
</html>
`;
}

if (import.meta.main) {
  const [src, out] = process.argv.slice(2);
  if (!src || !out) {
    console.error("usage: docs.js <docs dir> <out dir>");
    process.exit(2);
  }
  mkdirSync(out, { recursive: true });
  for (const [name] of DOCS) writeFileSync(join(out, `${name}.html`), docPage(name, readFileSync(join(src, `${name}.md`), "utf8")));
  // /docs/ opens the first doc
  writeFileSync(join(out, "index.html"), `<!doctype html><meta charset="utf-8"><title>Arena docs</title><meta http-equiv="refresh" content="0; url=${DOCS[0][0]}.html"><a href="${DOCS[0][0]}.html">Rules</a>\n`);
  console.log(`docs: ${DOCS.length} pages`);
}
