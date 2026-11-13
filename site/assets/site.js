// Every page loads this one script. It mounts the page's own script (picked by the body's class), starts the
// site's fx, and with fx on changes pages in place so the old page can burn open over the new one.
import { mount as mountDocs } from "./docs.js";
import { mount as mountLanding } from "./landing.js";
import { startNavigation } from "./nav.js";
import { startFx } from "./fx.js";

const PAGES = { landing: mountLanding, "docs-page": mountDocs };

const fx = await startFx();

async function mountPage() {
  const page = Object.keys(PAGES).find((name) => document.body.classList.contains(name));
  const unmount = page ? await PAGES[page]() : undefined;
  fx.decorate();
  return unmount;
}

if (fx.enabled) startNavigation({ mountPage, transition: fx.transition ?? undefined });
else {
  mountPage();
  prerenderOnIntent();
}

/** With fx off the browser changes pages itself: let it prerender a page once a click on its link looks likely
 *  (the pointer rests on it, or it is pressed), so the click shows it at once. Browsers without Speculation Rules
 *  skip this. */
function prerenderOnIntent() {
  if (!HTMLScriptElement.supports?.("speculationrules")) return;
  const rules = Object.assign(document.createElement("script"), { type: "speculationrules" });
  rules.textContent = JSON.stringify({
    prerender: [{
      where: { and: [{ href_matches: "/*" }, { not: { selector_matches: "[target], [download]" } }] },
      eagerness: "moderate",
    }],
  });
  document.head.append(rules);
}
