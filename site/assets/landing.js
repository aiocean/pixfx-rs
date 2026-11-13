// The landing page: a live hero scene and one live tile per effect, all drawn by the pixfx-canvas Stage.
// The page owns one animation-frame loop: for each canvas it fills the ground, draws the effects' back layers,
// draws its own sprites, then the front layers.
import init, { Stage, effects, meta } from "../pkg/pixfx_canvas.js";

const COLORS = { sky: "#13101b", ground: "#1e1a24", edge: "#2f2a38", star: "#f0dcc0" };
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

/** Options each effect takes beyond the shared ones, for the tile controls. */
const OWN = { "fire/campfire": [{ key: "height", label: "Height", min: 8, max: 40, step: 1, value: 22 }] };

/** Canvas pixels per game pixel in a tile. */
const TILE_SCALE = 3;
/** Seconds a one-shot tile rests empty before it plays again. */
const REST = 0.6;

/** A tile's box in game pixels: at least 160x170 so most effects share one shape and one floor line, 24 pixels above
 *  the box's bottom (lower when an effect reaches further below its origin); the effect's origin stands on the floor,
 *  its canvas centred. */
function boxFor(m) {
  const pad = 8;
  const below = Math.max(24, m.size[1] - m.origin[1] + pad);
  const w = Math.max(160, m.size[0] + 2 * pad);
  const h = Math.max(170, m.origin[1] + pad + below);
  const at = { x: Math.round((w - m.size[0]) / 2) + m.origin[0], y: h - below };
  return { w, h, at, ground: at.y };
}

/** Mount the landing page: the tabs, the hero and one tile per effect. Returns how to unmount it. */
export async function mount() {
  const off = new AbortController();
  setupTabs(document.getElementById("targets"));

  await init();

  const hero = createHero(document.getElementById("scene"), off.signal);
  const views = [hero];
  const gallery = document.getElementById("gallery");
  for (const id of effects()) views.push(createTile(gallery, id));

  let last = null;
  let heroVisible = true;
  let running = true;
  const seen = new IntersectionObserver(([e]) => { heroVisible = e.isIntersecting; });
  seen.observe(hero.canvas);

  // one animation-frame loop for every canvas on the page
  function frame(now) {
    if (!running) return;
    const dt = last === null ? 0 : Math.max(0, now - last) / 1000;
    last = now;
    for (const v of views) if (v !== hero || heroVisible) v.step(dt);
    requestAnimationFrame(frame);
  }

  if (reduceMotion) {
    // one still frame a second into each effect; clicks still add candles
    for (const v of views) v.step(1);
    document.getElementById("hint").textContent = "Animation is off because your system asks for reduced motion.";
  } else {
    requestAnimationFrame(frame);
  }

  return () => {
    running = false;
    off.abort();
    seen.disconnect();
    for (const v of views) v.free();
  };
}

// ---------- hero ----------

function createHero(canvas, signal) {
  const view = { canvas, stage: null, extra: [], layout: null };
  const copy = document.querySelector(".hero-copy");

  function build() {
    view.stage?.free();
    const narrow = innerWidth < 760;
    const scale = narrow ? 2 : 3;
    const rect = canvas.getBoundingClientRect();
    const w = Math.ceil(rect.width / scale);
    const h = Math.ceil(rect.height / scale);
    canvas.width = w * scale;
    canvas.height = h * scale;
    const ground = h - (narrow ? 8 : 18);
    const textRight = narrow ? 0 : (copy.getBoundingClientRect().right - rect.left) / scale;
    const room = w - textRight;

    // fires to the right of the headline; fewer when there is less room
    let placed;
    if (room >= 250) {
      const at = (f) => Math.round(textRight + room * f);
      placed = [["fire/candle", at(0.2)], ["fire/campfire", at(0.5)], ["fire/brazier", at(0.82)]];
    } else if (narrow) {
      placed = [["fire/candle", Math.round(w * 0.22)], ["fire/campfire", Math.round(w * 0.62)]];
    } else {
      placed = [["fire/campfire", Math.round(w - 80)]];
    }

    view.stage = new Stage(canvas, scale);
    // fireflies drift over the whole scene once the effect is ported: one 320-wide field after another
    if (effects().includes("weather/fireflies")) {
      for (let x = 160; x - 160 < w; x += 320) view.stage.play("weather/fireflies", x, ground, { seed: x });
    }
    for (const [id, x] of placed) view.stage.play(id, x, ground, undefined);
    const campfire = placed.find(([id]) => id === "fire/campfire");
    view.layout = { scale, w, h, ground, traveller: campfire ? campfire[1] - 20 : null, stars: stars(w, ground) };
    view.extra = [];
    if (reduceMotion) view.step(1);
  }

  view.step = (dt) => {
    const { scale, w, h, ground, traveller, stars: dots } = view.layout;
    view.stage.update(dt);
    const ctx = canvas.getContext("2d");
    ctx.fillStyle = COLORS.sky;
    ctx.fillRect(0, 0, w * scale, h * scale);
    ctx.fillStyle = COLORS.star;
    for (const [x, y, a] of dots) {
      ctx.globalAlpha = a;
      ctx.fillRect(x * scale, y * scale, scale, scale);
    }
    ctx.globalAlpha = 1;
    ctx.fillStyle = COLORS.ground;
    ctx.fillRect(0, ground * scale, w * scale, (h - ground) * scale);
    ctx.fillStyle = COLORS.edge;
    ctx.fillRect(0, ground * scale, w * scale, scale);
    view.stage.drawLayer("back");
    if (traveller !== null) drawTraveller(ctx, traveller, ground, scale);
    view.stage.drawLayer("front");
  };

  canvas.addEventListener("click", (e) => {
    const { scale, ground } = view.layout;
    const x = (e.offsetX * canvas.width) / canvas.clientWidth / scale;
    view.extra.push(view.stage.play("fire/candle", x, ground, { seed: Math.floor(Math.random() * 1e6) }));
    // keep six placed candles at most: the oldest fades out and removes itself
    if (view.extra.length > 6) view.stage.stop(view.extra.shift());
    if (reduceMotion) view.step(1);
  });

  let pending = null;
  addEventListener("resize", () => {
    clearTimeout(pending);
    pending = setTimeout(build, 150);
  }, { signal });
  signal.addEventListener("abort", () => clearTimeout(pending));
  view.free = () => view.stage?.free();

  build();
  return view;
}

/** Sky dots from a fixed hash, so the sky is the same on every load. */
function stars(w, ground) {
  const out = [];
  let s = 7;
  const next = () => ((s = (s * 1103515245 + 12345) % 2147483648) / 2147483648);
  for (let i = 0; i < Math.floor((w * ground) / 260); i++) {
    out.push([Math.floor(next() * w), Math.floor(next() * (ground - 30)), 0.25 + next() * 0.5]);
  }
  return out;
}

/** A small traveller in a cloak, feet at (x, ground), drawn between the effects' back and front layers. */
function drawTraveller(ctx, x, ground, s) {
  const px = (dx, dy, w, h, c) => {
    ctx.fillStyle = c;
    ctx.fillRect((x + dx) * s, (ground + dy) * s, w * s, h * s);
  };
  px(-3, -10, 7, 9, "#3a3346"); // cloak
  px(-2, -10, 2, 8, "#4c4658"); // lit fold
  px(-2, -14, 5, 4, "#d8b896"); // face
  px(-3, -15, 7, 2, "#3a3346"); // hood
  px(1, -13, 1, 1, "#13101b"); // eye
  px(-2, -1, 2, 1, "#24130f"); // boots
  px(1, -1, 2, 1, "#24130f");
  px(5, -16, 1, 16, "#5a3520"); // staff
}

// ---------- effect tiles ----------

function createTile(parent, id) {
  const m = meta(id);
  const BOX = { ...boxFor(m), scale: TILE_SCALE };
  const tile = document.createElement("article");
  tile.className = "tile";
  tile.innerHTML = `
    <canvas width="${BOX.w * BOX.scale}" height="${BOX.h * BOX.scale}" aria-label="${m.name}, playing live"></canvas>
    <div class="tile-body">
      <h3>${m.name}</h3>
      <p class="facts">${m.size[0]}×${m.size[1]} px · ${m.duration} s ${m.loop ? "loop" : "one-shot"} · ${m.fps} fps</p>
      <p class="desc">${m.desc}</p>
      <div class="controls"></div>
      <pre><code></code></pre>
    </div>`;
  parent.append(tile);

  const canvas = tile.querySelector("canvas");
  const controls = tile.querySelector(".controls");
  const code = tile.querySelector("code");
  const stage = new Stage(canvas, BOX.scale);
  const opts = { seed: m.seed, light: 1, palette: "default" };
  const own = OWN[id] ?? [];
  for (const o of own) opts[o.key] = o.value;
  let handle = null;

  const range = (key, label, min, max, step) => {
    const input = Object.assign(document.createElement("input"), { type: "range", min, max, step, value: opts[key] });
    input.id = `${id.replace("/", "-")}-${key}`;
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
  select.id = `${id.replace("/", "-")}-palette`;
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
    handle = stage.play(id, BOX.at.x, BOX.at.y, o);
    resting = 0;
    code.textContent = callText(id, o);
    if (reduceMotion) view.step(1);
  }

  let resting = 0;
  const view = {
    step(dt) {
      stage.update(dt);
      // a one-shot is removed once it ends: rest a moment, then play it again
      if (!m.loop && !stage.handles().includes(handle)) {
        resting += dt;
        if (resting >= REST) replay();
      }
      const ctx = canvas.getContext("2d");
      const s = BOX.scale;
      ctx.fillStyle = COLORS.ground;
      ctx.fillRect(0, 0, BOX.w * s, BOX.h * s);
      ctx.fillStyle = COLORS.sky;
      ctx.fillRect(0, BOX.ground * s, BOX.w * s, (BOX.h - BOX.ground) * s);
      stage.drawLayer("back");
      stage.drawLayer("front");
    },
    free: () => stage.free(),
  };

  replay();
  return view;
}

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
