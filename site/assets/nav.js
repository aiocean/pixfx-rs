// Page changes without a reload: a click on a link to another page of this site fetches that page, puts its body in
// place of this one, and mounts its page script. The old page is not dropped at once: its content moves into a
// layer on top (`.page-leaving`), so a transition can take it away; then the layer goes.
// Back and forward work the same way, each history entry keeping its scroll position.
// Pages are fetched before the click: when the pointer rests on a link, when a link gets focus, and when it is
// pressed (a click comes some 100 ms after the press). Each page is fetched once and kept, so back and forward and
// pages seen before change at once. While a click waits on the network the document is `aria-busy`; a later click
// wins over one still waiting.

/** How long the pointer rests on a link before its page is fetched: long enough to skip links it only crosses. */
const HOVER_DELAY = 65;

/**
 * @param {object} o
 * @param {() => (() => void) | void | Promise<(() => void) | void>} o.mountPage mounts the page now in the
 *   document and returns how to unmount it
 * @param {(layer: HTMLElement, at: {x: number, y: number} | null) => Promise<void>} [o.transition] takes the old page
 *   away; `at` is where the click was, in CSS pixels, or null for back and forward
 */
export async function startNavigation({ mountPage, transition }) {
  let unmount = await mountPage();
  /** Bumped by every navigation; one that finds it changed after a wait has been overtaken and stops. */
  let latest = 0;
  /** The page change now running (swap, mount, transition); the next one waits for it. */
  let changing = Promise.resolve();
  history.scrollRestoration = "manual";
  history.replaceState({ scroll: scrollY }, "");
  // the page itself, as served (the document now holds what its script mounted), for coming back to it
  page(new URL(location.href));
  prefetchOnIntent();

  addEventListener("click", (e) => {
    const url = target(e);
    if (!url) return;
    e.preventDefault();
    go(url, { x: e.clientX, y: e.clientY }, true);
  });
  addEventListener("popstate", (e) => go(new URL(location.href), null, false, e.state?.scroll ?? 0));

  async function go(url, at, push, scroll = 0) {
    const ticket = ++latest;
    document.documentElement.setAttribute("aria-busy", "true");
    const html = await page(url);
    if (ticket !== latest) return;
    await changing;
    if (ticket !== latest) return;
    document.documentElement.removeAttribute("aria-busy");
    // not an HTML page of ours: let the browser load it
    if (html === null) return location.assign(url.href);
    if (push) {
      history.replaceState({ scroll: scrollY }, "");
      history.pushState({ scroll: 0 }, "", url.href);
    }
    changing = change(new DOMParser().parseFromString(html, "text/html"), url, at, scroll);
    await changing;
  }

  async function change(next, url, at, scroll) {
    const layer = swap(next);
    unmount?.();
    unmount = await mountPage();
    // docs.js scrolls to the hash itself once the headings have ids
    if (!url.hash) scrollTo(0, scroll);
    focusMain();
    try {
      if (transition) await transition(layer, at);
    } finally {
      layer.remove();
    }
  }
}

/** The URL a click should take in place, or null when the browser should handle it. */
function target(e) {
  if (e.defaultPrevented || e.button !== 0 || e.metaKey || e.ctrlKey || e.shiftKey || e.altKey) return null;
  const url = pageLink(e.target);
  // a link within this page only scrolls
  if (!url || (url.pathname === location.pathname && url.search === location.search)) return null;
  return url;
}

/** The URL of the link `el` is in, when it goes to a page of this site in this tab; otherwise null. */
function pageLink(el) {
  const link = el?.closest?.("a[href]");
  if (!link || (link.target && link.target !== "_self") || link.hasAttribute("download")) return null;
  const url = new URL(link.href);
  return url.origin === location.origin ? url : null;
}

// ---------- the fetched pages ----------

/** Page HTML by URL (hash dropped), or null for a response that is not an HTML page. Static site: kept for good. */
const pages = new Map();

const key = (url) => url.href.split("#")[0];

/** The HTML of the page at `url`, fetched once. A failed fetch is forgotten, so the next try fetches again. */
function page(url) {
  const k = key(url);
  let html = pages.get(k);
  if (!html) {
    html = fetchHtml(k);
    pages.set(k, html);
    html.then((text) => text === null && pages.delete(k));
  }
  return html;
}

async function fetchHtml(href) {
  try {
    const res = await fetch(href, { headers: { Accept: "text/html" } });
    if (!res.ok || !res.headers.get("content-type")?.includes("text/html")) return null;
    return await res.text();
  } catch {
    return null;
  }
}

/** Fetch a link's page as soon as a click on it looks likely. With Save-Data on, only a press does. */
function prefetchOnIntent() {
  const prefetch = (el) => {
    const url = pageLink(el);
    if (url) page(url);
  };
  addEventListener("pointerdown", (e) => prefetch(e.target), { passive: true });
  if (navigator.connection?.saveData) return;
  addEventListener("focusin", (e) => prefetch(e.target));
  let resting = null;
  addEventListener("pointerover", (e) => {
    if (e.pointerType !== "mouse") return;
    clearTimeout(resting);
    resting = setTimeout(prefetch, HOVER_DELAY, e.target);
  }, { passive: true });
  addEventListener("pointerout", () => clearTimeout(resting), { passive: true });
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
