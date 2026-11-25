// The landing page plays a live effect showcase through the Canvas 2D shell.
import { loadPixfx } from "../pkg/pixfx-canvas2d.js";
import { createShow, loadFighters, showable } from "./sprite-show.js";

export async function mount() {
  setupTabs(document.getElementById("targets"));
  const canvas = document.getElementById("scene");
  const caption = document.getElementById("demo-caption");
  const toggle = document.getElementById("demo-toggle");

  let sheets;
  try {
    [, sheets] = await Promise.all([loadPixfx(), loadFighters()]);
  } catch (error) {
    caption.textContent = "The live demo could not load.";
    console.error(error);
    return;
  }

  const reduceMotion = matchMedia("(prefers-reduced-motion: reduce)").matches;
  const featured = "feedback/power-up";
  const ids = showable();
  if (ids.includes(featured)) {
    ids.splice(ids.indexOf(featured), 1);
    ids.unshift(featured);
  }

  let paused = reduceMotion;
  let visible = false;
  let pending = null;
  let last = null;

  const hero = createShow(canvas, {
    sheets,
    ids,
    still: reduceMotion,
    told: (take) => { caption.textContent = take; },
  });
  if (reduceMotion) hero.draw();
  else hero.step(0);

  const seen = new IntersectionObserver(([entry]) => {
    visible = entry.isIntersecting;
    schedule();
  });
  seen.observe(canvas);

  function schedule() {
    if (!paused && visible && hero) {
      pending ??= requestAnimationFrame(frame);
    } else {
      cancelAnimationFrame(pending);
      pending = last = null;
    }
  }

  function frame(now) {
    pending = null;
    const dt = last === null ? 0 : Math.min(0.1, (now - last) / 1000);
    last = now;
    hero.step(dt);
    schedule();
  }

  function updateToggle() {
    toggle.textContent = paused ? "Play demo" : "Pause demo";
  }
  updateToggle();
  toggle.hidden = false;
  toggle.addEventListener("click", () => {
    paused = !paused;
    updateToggle();
    schedule();
  });

  return () => {
    cancelAnimationFrame(pending);
    seen.disconnect();
    hero.free();
  };
}

// ---------- tabs (WAI-ARIA tabs pattern) ----------

function setupTabs(root) {
  const tabs = [...root.querySelectorAll('[role="tab"]')];
  const select = (tab) => {
    for (const t of tabs) {
      const on = t === tab;
      t.setAttribute("aria-selected", String(on));
      t.tabIndex = on ? 0 : -1;
      document.getElementById(t.getAttribute("aria-controls")).hidden = !on;
    }
    tab.focus();
  };
  for (const [i, tab] of tabs.entries()) {
    tab.addEventListener("click", () => select(tab));
    tab.addEventListener("keydown", (e) => {
      const step = { ArrowRight: 1, ArrowLeft: -1 }[e.key];
      const next = step ? tabs[(i + step + tabs.length) % tabs.length] : { Home: tabs[0], End: tabs.at(-1) }[e.key];
      if (!next) return;
      e.preventDefault();
      select(next);
    });
  }
}
