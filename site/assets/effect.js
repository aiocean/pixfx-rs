// One effect's page (effects/effect.html?id=<id>): the effect playing live on a stage, the options it takes as
// controls, the call that plays it as set, and its facts. One-shots play, rest a moment, and play again.
import init, { Stage, effects, meta } from "../pkg/pixfx_canvas.js";

/** The stage's sky and floor. */
const COLORS = { sky: "#13101b", ground: "#1e1a24" };
const reduceMotion = matchMedia("(prefers-reduced-motion: reduce)").matches;

const PALETTES = {
  default: { label: "Default", ramp: null },
  spirit: {
    label: "Spirit (blue)",
    ramp: ["#0d1033", "#1b2a8a", "#2a46c0", "#3a62e0", "#5a8cf0", "#7fb4ff", "#a8d0ff", "#d6ecff", "#ffffff"],
  },
  witch: {
    label: "Witch fire (green)",
    ramp: ["#0f1f12", "#1b3a1a", "#2a5e22", "#3f8a2a", "#62b236", "#8fd64a", "#bdf06e", "#e4ffa8", "#ffffff"],
  },
};

/** Options each effect takes beyond the shared ones, for the controls. */
const OWN = { "fire/campfire": [{ key: "height", label: "Height", min: 8, max: 40, step: 1, value: 22 }] };

/** Seconds a one-shot rests empty before it plays again. */
const REST = 0.6;

/** The stage's box in game pixels: room round the effect for its glow, its origin standing on the floor 24 pixels
 *  above the box's bottom (lower when the effect reaches further below its origin), its canvas centred. */
function boxFor(m) {
  const pad = 24;
  const below = Math.max(24, m.size[1] - m.origin[1] + pad);
  const w = Math.max(200, m.size[0] + 2 * pad);
  const h = Math.max(160, m.origin[1] + pad + below);
  const at = { x: Math.round((w - m.size[0]) / 2) + m.origin[0], y: h - below };
  return { w, h, at, ground: at.y };
}

/** Canvas pixels per game pixel: as large as fits the column, whole numbers only so pixels stay square. */
function scaleFor(box, room) {
  return Math.max(1, Math.min(5, Math.floor(room / box.w)));
}

/** Mount the effect page now in the document. Returns how to unmount it. */
export async function mount() {
  await init();
  const id = new URLSearchParams(location.search).get("id");
  const root = document.getElementById("effect");
  const ids = effects();
  if (!id || !ids.includes(id)) {
    root.innerHTML = `<h1>No such effect</h1>
      <p class="lead">There is no effect called <code>${escape(id ?? "")}</code>. <a href="./">See every effect</a>.</p>`;
    return undefined;
  }
  const m = meta(id);
  document.title = `${m.name} · pixfx effects`;
  const at = ids.indexOf(id);
  const prev = ids[(at - 1 + ids.length) % ids.length];
  const next = ids[(at + 1) % ids.length];

  root.innerHTML = `
    <p class="crumbs"><a href="./">Effects</a></p>
    <h1>${m.name}</h1>
    <p class="lead"><code>${id}</code>. ${m.desc}</p>
    <div class="effect-layout">
      <div class="effect-stage"><canvas aria-label="${m.name}, playing live"></canvas></div>
      <div class="effect-side">
        <div class="controls"></div>
        <pre><code></code></pre>
        <div class="table-wrap"><table><tbody>
          <tr><th scope="row">Size</th><td>${m.size[0]}×${m.size[1]} px</td></tr>
          <tr><th scope="row">Origin</th><td>(${m.origin[0]}, ${m.origin[1]})</td></tr>
          <tr><th scope="row">Glow margin</th><td>${m.glowMargin} px</td></tr>
          <tr><th scope="row">Duration</th><td>${m.duration} s, ${m.loop ? "looping" : "once"}</td></tr>
          <tr><th scope="row">Frames per sheet</th><td>${Math.round(m.duration * m.fps)} at ${m.fps} fps</td></tr>
          <tr><th scope="row">Default seed</th><td>${m.seed}</td></tr>
          <tr><th scope="row">Tags</th><td>${m.tags.join(", ")}</td></tr>
        </tbody></table></div>
      </div>
    </div>
    <nav class="pager" aria-label="Other effects">
      <a href="effect.html?id=${prev}" rel="prev">${meta(prev).name}</a>
      <a href="effect.html?id=${next}" rel="next">${meta(next).name}</a>
    </nav>`;

  const canvas = root.querySelector("canvas");
  const box = boxFor(m);
  const scale = scaleFor(box, root.querySelector(".effect-stage").clientWidth);
  canvas.width = box.w * scale;
  canvas.height = box.h * scale;
  const view = createView(canvas, root.querySelector(".controls"), root.querySelector(".effect-side code"), id, m, box, scale);

  let running = true;
  let last = null;
  const frame = (now) => {
    if (!running) return;
    view.step(last === null ? 0 : Math.max(0, now - last) / 1000);
    last = now;
    requestAnimationFrame(frame);
  };
  if (reduceMotion) view.step(1);
  else requestAnimationFrame(frame);

  return () => {
    running = false;
    view.free();
  };
}

/** The live effect on `canvas`, driven by the controls it adds to `controls`, with its call shown in `code`. */
function createView(canvas, controls, code, id, m, box, scale) {
  const stage = new Stage(canvas, scale);
  const opts = { seed: m.seed, light: 1, palette: "default" };
  const own = OWN[id] ?? [];
  for (const o of own) opts[o.key] = o.value;
  let handle = null;
  let resting = 0;

  const range = (key, label, min, max, step) => {
    const input = Object.assign(document.createElement("input"), { type: "range", min, max, step, value: opts[key] });
    input.id = `effect-${key}`;
    const lab = Object.assign(document.createElement("label"), { htmlFor: input.id, textContent: label });
    const out = document.createElement("output");
    out.value = opts[key];
    input.addEventListener("input", () => {
      opts[key] = Number(input.value);
      out.value = input.value;
      replay();
    });
    controls.append(lab, input, out);
  };

  range("light", "Light", 0, 2, 0.1);
  for (const o of own) range(o.key, o.label, o.min, o.max, o.step);

  const select = document.createElement("select");
  select.id = "effect-palette";
  for (const [key, p] of Object.entries(PALETTES)) select.append(new Option(p.label, key));
  select.addEventListener("change", () => {
    opts.palette = select.value;
    replay();
  });
  controls.append(Object.assign(document.createElement("label"), { htmlFor: select.id, textContent: "Palette" }), select);

  const reseed = Object.assign(document.createElement("button"), { type: "button", textContent: "New seed" });
  reseed.addEventListener("click", () => {
    opts.seed = Math.floor(Math.random() * 10000);
    replay();
  });
  controls.append(Object.assign(document.createElement("label"), { textContent: "Seed" }), reseed);

  function engineOptions() {
    const out = {};
    if (opts.seed !== m.seed) out.seed = opts.seed;
    if (opts.light !== 1) out.light = opts.light;
    if (opts.palette !== "default") out.palette = PALETTES[opts.palette].ramp;
    for (const o of own) if (opts[o.key] !== o.value) out[o.key] = opts[o.key];
    return out;
  }

  function replay() {
    if (handle !== null) stage.remove(handle);
    const o = engineOptions();
    handle = stage.play(id, box.at.x, box.at.y, o);
    resting = 0;
    code.textContent = callText(id, o);
    if (reduceMotion) view.step(1);
  }

  const view = {
    step(dt) {
      stage.update(dt);
      // a one-shot is removed once it ends: rest a moment, then play it again
      if (!m.loop && !stage.handles().includes(handle)) {
        resting += dt;
        if (resting >= REST) replay();
      }
      const ctx = canvas.getContext("2d");
      // the effect's cues move the whole view (a hit's shake) and lift it toward white (its flash)
      const [sx, sy] = stage.shake(reduceMotion ? 0 : 1);
      ctx.save();
      ctx.translate(sx * scale, sy * scale);
      ctx.fillStyle = COLORS.sky;
      ctx.fillRect(-scale * 4, -scale * 4, (box.w + 8) * scale, (box.ground + 4) * scale);
      ctx.fillStyle = COLORS.ground;
      ctx.fillRect(-scale * 4, box.ground * scale, (box.w + 8) * scale, (box.h - box.ground + 4) * scale);
      stage.drawLayer("back");
      stage.drawLayer("front");
      ctx.restore();
      const flash = reduceMotion ? 0 : stage.flash();
      if (flash > 0) {
        ctx.fillStyle = `rgba(255, 255, 255, ${flash})`;
        ctx.fillRect(0, 0, box.w * scale, box.h * scale);
      }
    },
    free: () => stage.free(),
  };

  replay();
  return view;
}

/** The call that plays the effect with the options as set: only what differs from the defaults. */
function callText(id, o) {
  const keys = Object.keys(o);
  if (keys.length === 0) return `stage.play("${id}", x, y);`;
  const parts = keys.map((k) => {
    const v = o[k];
    if (Array.isArray(v)) return `  ${k}: [${v.map((c) => `"${c}"`).join(", ")}],`;
    return `  ${k}: ${v},`;
  });
  return `stage.play("${id}", x, y, {\n${parts.join("\n")}\n});`;
}

function escape(text) {
  return text.replace(/[&<>"]/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" })[c]);
}
