// The arena's duel pages load this script (docs pages load only `menu.js`). It mounts the page's own part, picked by
// the body's data-page: the duel full-screen on the home page, two bots chosen from files on the watch page. The duel itself
// is the pixfx landing's (`duel-hero.js`, copied beside this file by `build.sh`); this file decides nothing about it.
import { loadPixfx } from "../pkg/pixfx-canvas2d.js";
import * as pixfx from "../pkg/pixfx-canvas2d.js";
import { useEngine, usePolicies } from "./duel.js";
import { createHero, loadIcons, loadPolicies } from "./duel-hero.js";

/** The bot both corners play when the page is not told otherwise: the trained policy shipped with the rules. */
const HOUSE = "policy.bin";

const PAGES = { home: mountHome, watch: mountWatch };
await PAGES[document.body.dataset.page]?.();

/** The engine and the skill icons, then the house bot in both corners. */
async function start() {
  const [engine, icons] = await Promise.all([loadPixfx(), loadIcons([32, 24])]);
  useEngine(pixfx);
  const [house] = await loadPolicies([HOUSE]);
  return { engine, icons, house };
}

/** Run `hero` each animation frame while its canvas is on screen and the tab is visible. */
function run(hero, onFrame) {
  let last = null;
  let visible = true;
  new IntersectionObserver(([e]) => {
    visible = e.isIntersecting;
  }).observe(hero.canvas);
  function frame(now) {
    const dt = last === null ? 0 : Math.max(0, now - last) / 1000;
    last = now;
    if (visible && !document.hidden) {
      hero.step(dt);
      onFrame?.();
    }
    requestAnimationFrame(frame);
  }
  requestAnimationFrame(frame);
}

async function mountHome() {
  const { engine, icons, house } = await start();
  usePolicies([house, house]);
  const hero = createHero(document.getElementById("scene"), new AbortController().signal, icons, engine);
  run(hero);
}

/** The watch page: a bot file for each corner (the house bot where none is chosen) and a seed. The same two files and
 *  the same seed play the same fight on every machine. */
async function mountWatch() {
  const form = document.getElementById("match");
  const status = document.getElementById("status");
  const seedInput = form.elements.seed;
  const params = new URLSearchParams(location.search);
  const asked = Number.parseInt(params.get("seed") ?? "", 10);
  const { engine, icons, house } = await start();
  usePolicies([house, house]);
  const hero = createHero(document.getElementById("scene"), new AbortController().signal, icons, engine, {
    seed: Number.isSafeInteger(asked) && asked > 0 ? asked : undefined,
    replay: true,
  });
  seedInput.value = String(hero.seed());
  const names = ["house bot", "house bot"];
  const say = (text, error = false) => {
    status.textContent = text;
    status.classList.toggle("error", error);
  };

  form.addEventListener("submit", async (e) => {
    e.preventDefault();
    const seed = Number.parseInt(seedInput.value, 10);
    if (!Number.isSafeInteger(seed) || seed < 1) return say("The seed is a whole number from 1 up.", true);
    try {
      const files = [form.elements.left.files[0], form.elements.right.files[0]];
      const bytes = await Promise.all(files.map(async (f) => (f ? new Uint8Array(await f.arrayBuffer()) : house)));
      usePolicies(bytes);
      files.forEach((f, i) => (names[i] = f ? f.name : "house bot"));
    } catch (err) {
      return say(`That file is not a bot for this season: ${err?.message ?? err}`, true);
    }
    hero.restart(seed);
    history.replaceState(null, "", `?seed=${seed}`);
    say("");
  });
  form.querySelector("[data-random]").addEventListener("click", () => {
    seedInput.value = String(1 + Math.floor(Math.random() * 1e6));
  });

  run(hero, () => {
    const d = hero.duel;
    if (!d || status.classList.contains("error")) return;
    const text = `Seed ${hero.seed()} · round ${d.round} · Thunder (${names[0]}) ${d.wins[0]} – ${d.wins[1]} Frost (${names[1]})`;
    if (status.textContent !== text) status.textContent = text;
  });
}

