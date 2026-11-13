// Docs chrome, from one list: the sidebar, the previous/next links, the "on this page" list, copy buttons and
// the narrow-screen menu. Every docs page is plain HTML with an empty #sidebar and .toc for this to fill.

const SECTIONS = [
  ["Start", [
    ["index.html", "Overview"],
    ["getting-started.html", "Getting started"],
    ["concepts.html", "Concepts"],
    ["effects.html", "Effect catalog"],
  ]],
  ["Use pixfx", [
    ["cli.html", "Command line"],
    ["sheets.html", "Sprite sheet format"],
    ["canvas.html", "Canvas renderer"],
    ["js-engine.html", "JS engine"],
    ["phaser.html", "Phaser plugin"],
    ["rust.html", "Rust API"],
    ["gpui.html", "gpui"],
  ]],
  ["Develop", [
    ["writing-effects.html", "Write an effect"],
    ["parity.html", "Parity and tests"],
    ["credits.html", "Credits and license"],
  ]],
];

// hosts may serve pages without ".html" (Cloudflare Pages redirects /docs/cli.html to /docs/cli), so match on the name
const page = (href) => href.replace(/\.html$/, "") || "index";

/** Fill the docs chrome of the page now in the document. */
export function mount() {
  const here = page(location.pathname.split("/").pop());
  const pages = SECTIONS.flatMap(([, items]) => items);

  const sidebar = document.getElementById("sidebar");
  sidebar.innerHTML = SECTIONS.map(([title, items]) => `
    <h4>${title}</h4>
    <ul>${items.map(([href, label]) =>
      `<li><a href="${href}"${page(href) === here ? ' aria-current="page"' : ""}>${label}</a></li>`).join("")}</ul>`).join("");

  const toggle = document.querySelector(".menu-toggle");
  toggle?.addEventListener("click", () => {
    const open = sidebar.classList.toggle("open");
    toggle.setAttribute("aria-expanded", String(open));
  });

  const main = document.querySelector("main.doc");
  const i = pages.findIndex(([href]) => page(href) === here);
  if (i >= 0) {
    const link = (p, dir) => p ? `<a href="${p[0]}"${dir === "Next" ? ' style="text-align:right;margin-left:auto"' : ""}><small>${dir}</small>${p[1]}</a>` : "";
    main.insertAdjacentHTML("beforeend", `<nav class="pager" aria-label="Pages">${link(pages[i - 1], "Previous")}${link(pages[i + 1], "Next")}</nav>`);
  }

  // headings get ids from their text, so every section can be linked
  const slug = (t) => t.toLowerCase().replace(/[^a-z0-9]+/g, "-").replace(/^-|-$/g, "");
  const heads = [...main.querySelectorAll("h2")];
  for (const h of main.querySelectorAll("h2, h3")) if (!h.id) h.id = slug(h.textContent);
  const toc = document.querySelector(".toc");
  if (toc && heads.length > 1) {
    toc.innerHTML = `<h4>On this page</h4><ul>${heads.map((h) => `<li><a href="#${h.id}">${h.textContent}</a></li>`).join("")}</ul>`;
  }
  if (location.hash) document.getElementById(location.hash.slice(1))?.scrollIntoView();

  for (const pre of main.querySelectorAll("pre")) {
    const b = Object.assign(document.createElement("button"), { className: "copy", type: "button", textContent: "Copy" });
    b.addEventListener("click", async () => {
      await navigator.clipboard.writeText(pre.querySelector("code")?.textContent ?? pre.textContent);
      b.textContent = "Copied";
      setTimeout(() => { b.textContent = "Copy"; }, 1500);
    });
    pre.append(b);
  }
}
