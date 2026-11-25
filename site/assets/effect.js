// Playable use cases, effect options, and code samples for one effect.
import { effects, loadPixfx, meta } from "../pkg/pixfx-canvas2d.js";
import { loadFighters } from "./sprite-show.js";
import { TARGETS, codeFor } from "./effect-code.js";
import { DEMOS } from "./effect-demos.js";
import { createDemo } from "./demo-view.js";

const reduceMotion = matchMedia("(prefers-reduced-motion: reduce)").matches;

/** The engine's palette presets (`palette: "ice"` recolours any effect), after the effect's own colours. */
/** Where the page keeps which target's code was picked last. */
const CODE_TARGET = "pixfx.code-target";

const PALETTES = ["default", "fire", "ice", "poison", "holy", "shadow", "arcane", "blood", "electric", "nature", "gold"];

/** Options each effect takes beyond the shared ones, for the controls. */
const OWN = {
  // how long, wide and wavy the trail is (projectiles/trail.rs SECS, WIDTH, WAVE)
  "projectiles/trail": [
    { key: "secs", label: "Seconds", min: 0.02, max: 0.5, step: 0.01, value: 0.12 },
    { key: "width", label: "Width", min: 1, max: 7, step: 1, value: 3 },
    { key: "wave", label: "Wave", min: 0, max: 4, step: 0.1, value: 1.2 },
    { key: "turn", label: "Turn with it", min: 0, max: 1, step: 1, value: 0 },
    // on a body: how narrow the tail ends, from where it turns translucent, from where the dither eats it
    // (projectiles/trail.rs TAPER, SOFT, BREAKUP)
    { key: "taper", label: "Tail width", min: 0, max: 1, step: 0.05, value: 1 },
    { key: "soft", label: "Fade from", min: 0, max: 1, step: 0.05, value: 1 },
    { key: "breakup", label: "Break up from", min: 0, max: 1, step: 0.05, value: 0.2 },
  ],
  "appear/teleport-beam": [
    { key: "bits", label: "Bits", min: 0, max: 24, step: 1, value: 12 },
    { key: "tear", label: "Tear", min: 0, max: 2, step: 0.25, value: 1 },
    { key: "flare", label: "Flare", min: 14, max: 23, step: 1, value: 21 },
  ],
  "disappear/blink-out": [
    { key: "motes", label: "Rune motes", min: 8, max: 48, step: 1, value: 30 },
    { key: "specks", label: "Specks", min: 0, max: 20, step: 1, value: 12 },
    { key: "glyph", label: "Floor glyph", min: 0.6, max: 1.4, step: 0.05, value: 1 },
  ],
  "appear/dither": [
    { key: "angle", label: "Sweep angle", min: 0, max: 360, step: 15, value: 90 },
    { key: "spread", label: "Spread", min: 0, max: 0.9, step: 0.05, value: 0.6 },
    { key: "whiten", label: "Whiten", min: 0, max: 1, step: 0.05, value: 0.3 },
  ],
  "appear/fade": [
    { key: "grow", label: "Grow", min: 0, max: 1, step: 0.05, value: 0 },
    { key: "whiten", label: "Whiten", min: 0, max: 1, step: 0.05, value: 0.5 },
  ],
  "disappear/dither": [
    { key: "angle", label: "Sweep angle", min: 0, max: 360, step: 15, value: 90 },
    { key: "spread", label: "Spread", min: 0, max: 0.9, step: 0.05, value: 0.6 },
    { key: "whiten", label: "Whiten", min: 0, max: 1, step: 0.05, value: 0.3 },
  ],
  "disappear/fade": [
    { key: "grow", label: "Grow", min: 0, max: 1, step: 0.05, value: 0 },
    { key: "whiten", label: "Whiten", min: 0, max: 1, step: 0.05, value: 0.5 },
  ],
  "disappear/incinerate": [
    { key: "flakes", label: "Loose ash", min: 8, max: 28, step: 1, value: 14 },
    { key: "breeze", label: "Breeze", min: 0, max: 1.2, step: 0.1, value: 1 },
  ],
  "feedback/electrified": [
    { key: "jag", label: "Jaggedness", min: 0.4, max: 1.8, step: 0.1, value: 1 },
    { key: "length", label: "Arc length", min: 0.6, max: 1.5, step: 0.05, value: 1 },
    { key: "sparks", label: "Sparks", min: 0, max: 2, step: 0.25, value: 1 },
    { key: "beat", label: "Snap arc", min: 0, max: 1, step: 1, value: 1 },
  ],
  "feedback/soaked": [
    { key: "pool", label: "Puddle", min: 12, max: 18, step: 1, value: 15 },
    { key: "drips", label: "Drips", min: 2, max: 8, step: 1, value: 8 },
    { key: "runnels", label: "Runnels", min: 0, max: 3, step: 1, value: 3 },
    { key: "splash", label: "Big splash", min: 0, max: 1, step: 1, value: 1 },
  ],
  "feedback/encase": [
    { key: "cracks", label: "Cracks", min: 1, max: 3, step: 1, value: 2 },
    { key: "dust", label: "Dust", min: 0.7, max: 1.25, step: 0.05, value: 1 },
  ],
  "feedback/cloak-shimmer": [
    { key: "tear", label: "Tear", min: 1, max: 3, step: 0.1, value: 2 },
    { key: "broken", label: "Broken edge", min: 0.15, max: 0.5, step: 0.01, value: 0.35 },
  ],
};

/** Labels and slider increments for options whose limits and defaults come from Rust. */
const SLIDERS = {
  "projectiles/violet-wake": {
    secs: ["Wake lifetime (s)", 0.01],
    width: ["Ribbon width", 0.1],
  },
  "feedback/outline": {
    width: ["Width (px)", 1],
    corners: ["Square corners", 1],
  },
  "feedback/volt-orbit": {
    gap: ["Orbit gap", 0.1],
    jag: ["Jaggedness", 0.1],
    sparks: ["Sparks per arc", 1],
    w: ["Effect canvas width (px)", 1],
    h: ["Effect canvas height (px)", 1],
  },
};

/** Mount the effect page now in the document. Returns how to unmount it. */
export async function mount() {
  const params = new URLSearchParams(location.search);
  const id = params.get("id");
  const pixelDensity = params.get("density") === "1" ? 1 : 2;
  const root = document.getElementById("effect");
  const screen = id?.startsWith("transitions/");
  const [, sheets] = await Promise.all([loadPixfx(), screen ? undefined : loadFighters(pixelDensity)]);
  const ids = effects();
  if (!id || !ids.includes(id)) {
    root.innerHTML = `<h1>No such effect</h1>
      <p class="lead">There is no effect called <code>${escape(id ?? "")}</code>. <a href="./">See every effect</a>.</p>`;
    return undefined;
  }
  const m = meta(id);
  const demos = DEMOS[id] ?? [[m.name, "Play", screen ? "screen" : undefined]];
  document.title = `${m.name} · pixfx effects`;
  const at = ids.indexOf(id);
  const prev = ids[(at - 1 + ids.length) % ids.length];
  const next = ids[(at + 1) % ids.length];
  const densityQuery = pixelDensity === 1 ? "&amp;density=1" : "";

  root.innerHTML = `
    <p class="crumbs"><a href="./">Effects</a></p>
    <h1>${m.name}</h1>
    <p class="lead"><code>${id}</code>. ${m.desc}</p>
    <div class="effect-layout">
      <div class="effect-demos">
        ${demos.map(([title, action], i) => `<figure class="effect-stage" aria-labelledby="demo-title-${i}">
          <div class="demo-head">
            <h2 id="demo-title-${i}">${escape(title)}</h2>
            <button type="button">${escape(action)}</button>
          </div>
          <canvas aria-label="${escape(title)} preview"></canvas>
          <figcaption class="effect-caption"></figcaption>
        </figure>`).join("")}
      </div>
      <div class="effect-side">
        <div class="controls"></div>
        <div class="tabs effect-code">
          <div role="tablist" aria-label="Use it in">
            ${TARGETS.map((t) => `<button role="tab" id="code-tab-${t.key}" aria-controls="code-panel" data-target="${t.key}">${t.label}</button>`).join("")}
          </div>
          <div role="tabpanel" id="code-panel">
            <pre><code></code></pre>
            <p class="effect-code-docs"><a></a></p>
          </div>
        </div>
        <div class="table-wrap"><table><tbody>
          <tr><th scope="row">Size</th><td>${m.size[0]}×${m.size[1]} px</td></tr>
          <tr><th scope="row">Origin</th><td>(${m.origin[0]}, ${m.origin[1]})</td></tr>
          <tr><th scope="row">Glow margin</th><td>${m.glowMargin} px</td></tr>
          <tr><th scope="row">Duration</th><td>${m.duration} s, ${m.loop ? "looping" : "once"}</td></tr>
          <tr><th scope="row">Frames per sheet</th><td>${Math.round(m.duration * m.fps)} at ${m.fps} fps</td></tr>
          <tr><th scope="row">Default seed</th><td>${m.seed}</td></tr>
          <tr><th scope="row">Render kind</th><td>${m.renderKind}</td></tr>
          <tr><th scope="row">Tags</th><td>${m.tags.join(", ")}</td></tr>
        </tbody></table></div>
      </div>
    </div>
    <nav class="pager" aria-label="Other effects">
      <a href="effect.html?id=${prev}${densityQuery}" rel="prev">${meta(prev).name}</a>
      <a href="effect.html?id=${next}${densityQuery}" rel="next">${meta(next).name}</a>
    </nav>`;

  const call = root.querySelector(".effect-code pre");
  const code = call.querySelector("code");
  const docs = root.querySelector(".effect-code-docs a");
  const copy = Object.assign(document.createElement("button"), { className: "copy", type: "button", textContent: "Copy" });
  copy.addEventListener("click", async () => {
    await navigator.clipboard.writeText(code.textContent);
    copy.textContent = "Copied";
    setTimeout(() => { copy.textContent = "Copy"; }, 1500);
  });
  call.append(copy);

  const views = [];
  // the code for the target picked (kept across effect pages), with the options as set
  let target = TARGETS.some((t) => t.key === localStorage.getItem(CODE_TARGET)) ? localStorage.getItem(CODE_TARGET) : TARGETS[0].key;
  const writeCode = () => {
    const t = TARGETS.find((x) => x.key === target) ?? TARGETS[0];
    const options = controls.options();
    if (pixelDensity === 1) options.pixelDensity = pixelDensity;
    code.textContent = codeFor(t.key, id, m, options);
    docs.href = t.docs;
    docs.textContent = `${t.label} reference`;
    root.querySelector(".effect-code [role=tabpanel]").setAttribute("aria-labelledby", `code-tab-${t.key}`);
  };
  const tabs = [...root.querySelectorAll(".effect-code [role=tab]")];
  const pick = (tab, focus) => {
    target = tab.dataset.target;
    localStorage.setItem(CODE_TARGET, target);
    for (const t of tabs) {
      t.setAttribute("aria-selected", String(t === tab));
      t.tabIndex = t === tab ? 0 : -1;
    }
    if (focus) tab.focus();
    writeCode();
  };
  for (const [i, tab] of tabs.entries()) {
    tab.addEventListener("click", () => pick(tab, true));
    tab.addEventListener("keydown", (e) => {
      const step = { ArrowRight: 1, ArrowLeft: -1 }[e.key];
      if (step) pick(tabs[(i + step + tabs.length) % tabs.length], true);
      if (e.key === "Home") pick(tabs[0], true);
      if (e.key === "End") pick(tabs.at(-1), true);
    });
  }
  const controls = createControls(root.querySelector(".controls"), id, m, () => {
    writeCode();
    for (const view of views) if (view.started) view.play();
  });
  pick(tabs.find((t) => t.dataset.target === target) ?? tabs[0], false);

  for (const [i, figure] of [...root.querySelectorAll(".effect-stage")].entries()) {
    const canvas = figure.querySelector("canvas");
    const caption = figure.querySelector("figcaption");
    const scenario = demos[i][2];
    const view = createDemo(canvas, id, {
      sheets, told: (text) => { caption.textContent = text; }, still: reduceMotion, scenario, pixelDensity,
    });
    if (scenario === "screen") caption.textContent = "The current screen burns closed and opens onto a new area.";
    view.play = () => {
      view.started = true;
      view.restart(controls.options());
    };
    figure.querySelector("button").addEventListener("click", view.play);
    view.draw();
    views.push(view);
  }

  let running = true;
  let last = null;
  const frame = (now) => {
    if (!running) return;
    const dt = last === null ? 0 : Math.min(0.05, Math.max(0, now - last) / 1000);
    for (const view of views) if (view.started && !view.finished()) view.step(dt);
    last = now;
    requestAnimationFrame(frame);
  };
  requestAnimationFrame(frame);

  return () => {
    running = false;
    for (const view of views) view.free();
  };
}

/** The controls in `root`: light, the effect's own options, palette and seed. Each change calls `changed`.
 *  `options()` is the engine options as set: only what differs from the defaults. */
function createControls(root, id, m, changed) {
  const opts = { seed: m.seed, light: 1, palette: "default" };
  const own = [...(OWN[id] ?? [])];
  const sliders = SLIDERS[id] ?? {};
  for (const spec of m.options) {
    const slider = sliders[spec.name];
    if (spec.kind !== "num" || !spec.range || !slider) continue;
    const [label, step] = slider;
    const [min, max] = spec.range;
    own.push({ key: spec.name, label, min, max, step, value: spec.default });
  }
  // an effect that fires impact cues takes `feel`: how hard its hit shakes, freezes, rings and flashes the screen
  if (m.options.some((o) => o.name === "feel")) own.push({ key: "feel", label: "Feel", min: 0, max: 2, step: 0.1, value: 1 });
  for (const o of own) opts[o.key] = o.value;

  const range = (key, label, min, max, step) => {
    const input = Object.assign(document.createElement("input"), { type: "range", min, max, step, value: opts[key] });
    input.id = `effect-${key}`;
    const lab = Object.assign(document.createElement("label"), { htmlFor: input.id, textContent: label });
    const out = document.createElement("output");
    out.value = opts[key];
    input.addEventListener("input", () => {
      opts[key] = Number(input.value);
      out.value = input.value;
      changed();
    });
    root.append(lab, input, out);
  };

  range("light", "Light", 0, 2, 0.1);
  for (const o of own) range(o.key, o.label, o.min, o.max, o.step);

  const choice = (key, label, values) => {
    const select = document.createElement("select");
    select.id = `effect-${key}`;
    for (const v of values) select.append(new Option(v === "default" ? "As designed" : v, v));
    select.addEventListener("change", () => {
      opts[key] = select.value;
      changed();
    });
    root.append(Object.assign(document.createElement("label"), { htmlFor: select.id, textContent: label }), select);
  };

  choice("palette", "Palette", PALETTES);

  const reseed = Object.assign(document.createElement("button"), { type: "button", textContent: "New seed" });
  reseed.addEventListener("click", () => {
    opts.seed = Math.floor(Math.random() * 10000);
    changed();
  });
  root.append(Object.assign(document.createElement("label"), { textContent: "Seed" }), reseed);

  return {
    options() {
      const out = {};
      if (opts.seed !== m.seed) out.seed = opts.seed;
      if (opts.light !== 1) out.light = opts.light;
      if (opts.palette !== "default") out.palette = opts.palette;
      for (const o of own) if (opts[o.key] !== o.value) out[o.key] = opts[o.key];
      return out;
    },
  };
}

function escape(text) {
  return text.replace(/[&<>"]/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" })[c]);
}
