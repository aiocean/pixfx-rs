// Page changes without a reload: a click on a link to another page of this site fetches that page, puts its body in
// place of this one, and mounts its page script. The old page is not dropped at once: its content moves into a
// layer on top (`.page-leaving`), so a transition can take it away; then the layer goes.
// Back and forward work the same way, each history entry keeping its scroll position.

/**
 * @param {object} o
 * @param {() => (() => void) | void | Promise<(() => void) | void>} o.mountPage mounts the page now in the
 *   document and returns how to unmount it
 * @param {(layer: HTMLElement, at: {x: number, y: number} | null) => Promise<void>} [o.transition] takes the old page
 *   away; `at` is where the click was, in CSS pixels, or null for back and forward
 */
export async function startNavigation({ mountPage, transition }) {
  let unmount = await mountPage();
  let busy = false;
  history.scrollRestoration = "manual";
  history.replaceState({ scroll: scrollY }, "");

  addEventListener("click", (e) => {
    const url = target(e);
    if (!url) return;
    e.preventDefault();
    go(url, { x: e.clientX, y: e.clientY }, true);
  });
  addEventListener("popstate", (e) => go(new URL(location.href), null, false, e.state?.scroll ?? 0));

  async function go(url, at, push, scroll = 0) {
    if (busy) return;
    busy = true;
    try {
      const next = await fetchPage(url);
      // not an HTML page of ours: let the browser load it
      if (!next) return location.assign(url.href);
      if (push) {
        history.replaceState({ scroll: scrollY }, "");
        history.pushState({ scroll: 0 }, "", url.href);
      }
      const layer = swap(next);
      unmount?.();
      unmount = await mountPage();
      // docs.js scrolls to the hash itself once the headings have ids
      if (!url.hash) scrollTo(0, scroll);
      focusMain();
      if (transition) await transition(layer, at);
      layer.remove();
    } finally {
      busy = false;
    }
  }
}

/** The URL a click should take in place, or null when the browser should handle it. */
function target(e) {
  const link = e.target.closest?.("a[href]");
  if (!link || e.defaultPrevented || e.button !== 0 || e.metaKey || e.ctrlKey || e.shiftKey || e.altKey) return null;
  if ((link.target && link.target !== "_self") || link.hasAttribute("download")) return null;
  const url = new URL(link.href);
  if (url.origin !== location.origin) return null;
  // a link within this page only scrolls
  if (url.pathname === location.pathname && url.search === location.search) return null;
  return url;
}

async function fetchPage(url) {
  try {
    const res = await fetch(url.href, { headers: { Accept: "text/html" } });
    if (!res.ok || !res.headers.get("content-type")?.includes("text/html")) return null;
    return new DOMParser().parseFromString(await res.text(), "text/html");
  } catch {
    return null;
  }
}

/** Put `next`'s page in the document; the old content goes into a layer on top, scrolled as it was. */
function swap(next) {
  const body = document.body;
  // fx canvases belong to the site, not the page: they stay
  const keep = (el) => el.matches(".fx-overlay, .fx-screen");
  const layer = document.createElement("div");
  layer.className = `page-leaving ${body.className}`;
  layer.setAttribute("aria-hidden", "true");
  layer.inert = true;
  const scroll = scrollY;
  layer.append(...[...body.children].filter((el) => !keep(el)));
  body.append(layer);
  layer.scrollTop = scroll;

  document.title = next.title;
  const description = next.querySelector('meta[name="description"]')?.content;
  if (description) document.querySelector('meta[name="description"]')?.setAttribute("content", description);
  body.className = next.body.className;
  // the parsed page's scripts never run (DOMParser leaves them inert); its page script is mounted instead
  const incoming = [...next.body.children].filter((el) => el.tagName !== "SCRIPT");
  body.prepend(...incoming.map((el) => document.adoptNode(el)));
  return layer;
}

function focusMain() {
  const main = document.querySelector("main");
  if (!main) return;
  main.tabIndex = -1;
  main.focus({ preventScroll: true });
}
