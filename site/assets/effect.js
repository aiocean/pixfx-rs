// One effect's page (effects/effect.html?id=<id>): the effect playing live on a stage, the options it takes as
// controls, the call that plays it as set, and its facts. One-shots play, rest a moment, and play again.
import { createPixfx, effects, loadPixfx, meta } from "../pkg/pixfx-canvas2d.js";

/** The stage's sky and floor. */
const COLORS = { sky: "#13101b", ground: "#1e1a24" };
const reduceMotion = matchMedia("(prefers-reduced-motion: reduce)").matches;

/** The engine's palette presets (`palette: "ice"` recolours any effect), after the effect's own colours. */
const PALETTES = ["default", "fire", "ice", "poison", "holy", "shadow", "arcane", "blood", "electric", "nature", "gold"];

/** The outlines every effect takes: as designed, none, or a preset ring. */
const OUTLINES = ["default", "off", "dark", "black", "white", "thick"];

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
  await loadPixfx();
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
  const call = root.querySelector(".effect-side pre");
  const copy = Object.assign(document.createElement("button"), { className: "copy", type: "button", textContent: "Copy" });
  copy.addEventListener("click", async () => {
    await navigator.clipboard.writeText(call.querySelector("code").textContent);
    copy.textContent = "Copied";
    setTimeout(() => { copy.textContent = "Copy"; }, 1500);
  });
  call.append(copy);
  const view = createView(canvas, root.querySelector(".controls"), call.querySelector("code"), id, m, box, scale);

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
  const ctx = canvas.getContext("2d");
  const stage = createPixfx(ctx, { scale });
  // reduced motion: nothing shakes, rings or flashes
  if (reduceMotion) stage.feel.setShakeStrength(0).setFlashStrength(0);
  const opts = { seed: m.seed, light: 1, palette: "default", outline: "default" };
  const own = [...(OWN[id] ?? [])];
  // an effect that fires impact cues takes `feel`: how hard its hit shakes, freezes, rings and flashes the screen
  if (m.options.some((o) => o.name === "feel")) own.push({ key: "feel", label: "Feel", min: 0, max: 2, step: 0.1, value: 1 });
  for (const o of own) opts[o.key] = o.value;
  let effect = null;
  let ended = false;
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

  const choice = (key, label, values) => {
    const select = document.createElement("select");
    select.id = `effect-${key}`;
    for (const v of values) select.append(new Option(v === "default" ? "As designed" : v, v));
    select.addEventListener("change", () => {
      opts[key] = select.value;
      replay();
    });
    controls.append(Object.assign(document.createElement("label"), { htmlFor: select.id, textContent: label }), select);
  };

  choice("palette", "Palette", PALETTES);
  choice("outline", "Outline", OUTLINES);

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
    if (opts.palette !== "default") out.palette = opts.palette;
    if (opts.outline !== "default") out.outline = opts.outline;
    for (const o of own) if (opts[o.key] !== o.value) out[o.key] = opts[o.key];
    return out;
  }

  function replay() {
    effect?.destroy();
    const o = engineOptions();
    ended = false;
    effect = stage.play(id, box.at.x, box.at.y, o).on("complete", () => {
      ended = true;
    });
    resting = 0;
    code.textContent = callText(id, o);
    if (reduceMotion) view.step(1);
  }

  const view = {
    step(dt) {
      stage.update(dt);
      // a one-shot is removed once it ends: rest a moment, then play it again
      if (!m.loop && ended) {
        resting += dt;
        if (resting >= REST) replay();
      }
      // the effect's cues move the whole view (a hit's shake) and lift it toward white (its flash)
      const [sx, sy] = stage.feel.shake;
      ctx.save();
      ctx.translate(sx * scale, sy * scale);
      ctx.fillStyle = COLORS.sky;
      ctx.fillRect(-scale * 4, -scale * 4, (box.w + 8) * scale, (box.ground + 4) * scale);
      ctx.fillStyle = COLORS.ground;
      ctx.fillRect(-scale * 4, box.ground * scale, (box.w + 8) * scale, (box.h - box.ground + 4) * scale);
      stage.drawLayer("back");
      stage.drawLayer("front");
      ctx.restore();
      // a heavy hit's ring bends what is drawn
      stage.warp();
      const flash = stage.feel.flash;
      if (flash > 0) {
        ctx.fillStyle = `rgba(255, 255, 255, ${flash})`;
        ctx.fillRect(0, 0, box.w * scale, box.h * scale);
      }
    },
    free: () => stage.destroy(),
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
    return `  ${k}: ${JSON.stringify(v)},`;
  });
  return `stage.play("${id}", x, y, {\n${parts.join("\n")}\n});`;
}

function escape(text) {
  return text.replace(/[&<>"]/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" })[c]);
}
