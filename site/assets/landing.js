// The landing page: a live duel in the hero, drawn by pixfx-canvas2d (workers paint the effects). The duel is the
// arena's (projects/arena), embedded by `site/build.sh` into site/arena: its page kit and its wasm, beside the engine's.
// The page owns the animation-frame loop.
import { loadPixfx } from "../pkg/pixfx-canvas2d.js";
import loadRules, * as rules from "../arena/wasm/arena_wasm.js";
import { usePolicies, useRules } from "../arena/duel.js";
import { createHero, loadIcons, loadPolicies } from "../arena/duel-hero.js";

/** Each corner's trained policy, `[left, right]`: a weights file of the arena's (`site/arena/weights/`). */
const POLICY_FILES = ["policy.bin", "policy.bin"];
const reduceMotion = matchMedia("(prefers-reduced-motion: reduce)").matches;

/** Mount the landing page: the tabs and the hero. Returns how to unmount it. */
export async function mount() {
  const off = new AbortController();
  setupTabs(document.getElementById("targets"));

  // the engine, the arena's rules and its skill icon atlases, by icon size; the hero waits for all three
  const [engine, icons] = await Promise.all([loadPixfx(), loadIcons([32, 24]), loadRules()]);
  useRules(rules);
  // each corner's trained policy is a file of its own, about as heavy as the engine: fetched only once the engine is
  // in, so it never holds back the first frame, and seated from the next duel on. A file named twice is fetched once. Until both
  // arrive, or if one never does, the duel seats the brain.
  loadPolicies(POLICY_FILES)
    .then(usePolicies)
    .catch((e) => console.warn("landing: the duel seats the brain, a trained policy did not load:", e));

  const hero = createHero(document.getElementById("scene"), off.signal, icons, engine);
  let last = null;
  let running = true;
  // the hero plays only while it is on screen; scrolled away, it waits where it was
  let visible = true;
  const seen = new IntersectionObserver(([e]) => {
    visible = e.isIntersecting;
  });
  seen.observe(hero.canvas);
  const proof = createProof(document.getElementById("proof"), () => (visible ? [hero] : []));

  function frame(now) {
    if (!running) return;
    const dt = last === null ? 0 : Math.max(0, now - last) / 1000;
    last = now;
    if (visible) hero.step(dt);
    proof.frame(now);
    requestAnimationFrame(frame);
  }

  if (reduceMotion) {
    hero.step(1);
    proof.frame(performance.now(), true);
  } else {
    requestAnimationFrame(frame);
  }

  return () => {
    running = false;
    off.abort();
    seen.disconnect();
    hero.free();
  };
}

// ---------- proof ----------

/** Seconds between updates of the proof line, so its numbers can be read. */
const PROOF_EVERY = 0.5;
/** A frame further apart than this from the last one came from a hidden or throttled page. */
const SLOW_FRAME_MS = 50;

/**
 * The line under the hero that measures this page as it runs: how many effects are playing on screen, the frame
 * rate the page holds while it plays them, and how big the engine's download was.
 */
function createProof(el, onScreen) {
  let since = null;
  let prev = null;
  let frames = 0;
  const download = engineDownload();
  const write = (fps) => {
    const playing = onScreen().reduce((n, v) => n + v.playing(), 0);
    el.innerHTML =
      `On your screen, <b>${playing} effects</b> are playing` +
      (fps ? ` at <b>${fps} frames a second</b>` : "") +
      (download ? `, all from a <b>${download} KB</b> engine.` : ".");
  };
  return {
    frame(now, still = false) {
      if (!el) return;
      if (still) return write(null);
      // a hidden or throttled page sends frames seconds apart: count only frames that come at a display's pace
      const steady = prev !== null && now - prev < SLOW_FRAME_MS;
      prev = now;
      if (!steady) since = null;
      if (since === null) {
        since = now;
        frames = 0;
        return;
      }
      frames++;
      if (now - since < PROOF_EVERY * 1000) return;
      write(Math.round((frames * 1000) / (now - since)));
      since = now;
      frames = 0;
    },
  };
}

/** The engine's wasm as it came over the network, in KB, or null when the browser does not say. */
function engineDownload() {
  const entry = performance.getEntriesByType("resource").find((e) => e.name.endsWith(".wasm"));
  const bytes = entry?.encodedBodySize || entry?.transferSize;
  return bytes ? Math.round(bytes / 1024) : null;
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
      if (step) select(tabs[(i + step + tabs.length) % tabs.length]);
      if (e.key === "Home") select(tabs[0]);
      if (e.key === "End") select(tabs.at(-1));
    });
  }
}
