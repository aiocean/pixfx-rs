// One effect's page (effects/effect.html?id=<id>): the effect playing live on a stage, the options it takes as
// controls, the call that plays it as set, and its facts. The stage shows the effect in a duel by default (Thunder
// and Frost on the mountain dais, the effect staged by its role in a short take that ends at its impact), or alone
// on a plain floor. One-shots and takes play, rest a moment, and play again.
import { Body, FightFeel, Lighting, Sprites, createPixfx, effects, lightColor, loadPixfx, meta, putOutLight, stepLights } from "../pkg/pixfx-canvas2d.js";
import loadRules, * as rules from "../arena/wasm/arena_wasm.js";
import { useRules } from "../arena/duel.js";
import { createBout, impact, step, take } from "./bout.js";
import { roleFor } from "./effect-roles.js";
import { TARGETS, codeFor } from "./effect-code.js";
import { actorOf, drawNumbers, drawStreaks, paintFighters } from "../arena/duel-art.js";
import { createScene, drawScene } from "../arena/duel-scene.js";
import { kindle } from "../arena/duel-light.js";

/** The plain stage's sky and floor. */
const COLORS = { sky: "#13101b", ground: "#1e1a24" };
/** Behind the duel's scene while the screen shakes. */
const BACKDROP = "#0b0a1c";
/** The colour the engine says effect `fx` with `opts` lights its surroundings in (or none), asked once for each. */
const lightColors = new Map();
function lightOf(fx, opts) {
  const key = `${fx} ${JSON.stringify(opts ?? {})}`;
  if (!lightColors.has(key)) lightColors.set(key, lightColor(fx, opts));
  return lightColors.get(key);
}
const reduceMotion = matchMedia("(prefers-reduced-motion: reduce)").matches;

/** The engine's palette presets (`palette: "ice"` recolours any effect), after the effect's own colours. */
/** Where the page keeps which target's code was picked last. */
const CODE_TARGET = "pixfx.code-target";

const PALETTES = ["default", "fire", "ice", "poison", "holy", "shadow", "arcane", "blood", "electric", "nature", "gold"];

/** Options each effect takes beyond the shared ones, for the controls. */
const OWN = {
  "fire/campfire": [{ key: "height", label: "Height", min: 8, max: 40, step: 1, value: 22 }],
  // where the blow lands on the seal, from the feet (arcane/restriction.rs HIT_X, HIT_Y)
  "arcane/restriction": [
    { key: "hitX", label: "Hit x", min: -30, max: 30, step: 1, value: 22 },
    { key: "hitY", label: "Hit y", min: -60, max: 0, step: 1, value: -29 },
  ],
  // how long, wide and wavy the trail is (projectiles/trail.rs SECS, WIDTH, WAVE)
  "projectiles/trail": [
    { key: "secs", label: "Seconds", min: 0.02, max: 0.5, step: 0.01, value: 0.12 },
    { key: "width", label: "Width", min: 1, max: 7, step: 1, value: 3 },
    { key: "wave", label: "Wave", min: 0, max: 4, step: 0.1, value: 1.2 },
    { key: "turn", label: "Turn with it", min: 0, max: 1, step: 1, value: 0 },
  ],
  // where the blow lands on the mirror, from the feet (arcane/bagua_mirror.rs HIT_X, HIT_Y)
  "arcane/sword-slash": [
    { key: "size", label: "Size", min: 1, max: 3, step: 0.5, value: 2 },
    { key: "hitX", label: "Hit x", min: -40, max: 40, step: 1, value: 0 },
    { key: "hitY", label: "Hit y", min: -60, max: 0, step: 1, value: -22 },
  ],
  "arcane/bagua-mirror": [
    { key: "hitX", label: "Hit x", min: -40, max: 40, step: 1, value: 26 },
    { key: "hitY", label: "Hit y", min: -50, max: -5, step: 1, value: -26 },
  ],
  // where the blow lands on the dome, from the feet (shadow/blood_shield.rs HIT_X, HIT_Y)
  "shadow/blood-shield": [
    { key: "hitX", label: "Hit x", min: -60, max: 60, step: 1, value: 38 },
    { key: "hitY", label: "Hit y", min: -90, max: 6, step: 1, value: -34 },
  ],
};

/** Seconds a one-shot or a take rests before it plays again. */
const REST = 0.6;
/** A still frame (reduced motion) looks this long for the take's impact before it settles for what it has, and shows
 *  the effect this long after its impact, once it has grown. */
const STILL_SEARCH = 6;
const STILL_AFTER = 0.3;

/** The stage's two views. */
const MODES = [{ value: "duel", label: "In a duel" }, { value: "alone", label: "Alone" }];

/** Mount the effect page now in the document. Returns how to unmount it. */
export async function mount() {
  await Promise.all([loadPixfx(), loadRules()]);
  useRules(rules);
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
      <figure class="effect-stage">
        <div class="effect-mode" role="radiogroup" aria-label="Show the effect">
          ${MODES.map((o, i) => `<label><input type="radio" name="effect-mode" value="${o.value}"${i === 0 ? " checked" : ""}> ${o.label}</label>`).join("")}
        </div>
        <canvas></canvas>
        <figcaption class="effect-caption"></figcaption>
      </figure>
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
          <tr><th scope="row">Tags</th><td>${m.tags.join(", ")}</td></tr>
        </tbody></table></div>
      </div>
    </div>
    <nav class="pager" aria-label="Other effects">
      <a href="effect.html?id=${prev}" rel="prev">${meta(prev).name}</a>
      <a href="effect.html?id=${next}" rel="next">${meta(next).name}</a>
    </nav>`;

  const canvas = root.querySelector("canvas");
  const caption = root.querySelector(".effect-caption");
  // the canvas pixels across the stage has room for, inside its border
  const room = () => canvas.clientWidth;
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

  const role = roleFor(id, m);
  let view = null;
  // the code for the target picked (kept across effect pages), with the options as set
  let target = TARGETS.some((t) => t.key === localStorage.getItem(CODE_TARGET)) ? localStorage.getItem(CODE_TARGET) : TARGETS[0].key;
  const writeCode = () => {
    const t = TARGETS.find((x) => x.key === target) ?? TARGETS[0];
    code.textContent = codeFor(t.key, id, m, controls.options());
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
    view.restart(controls.options());
  });
  pick(tabs.find((t) => t.dataset.target === target) ?? tabs[0], false);

  const show = (mode) => {
    view?.free();
    view = mode === "duel" ? createDuelView(canvas, id, m, role, room(), (text) => { caption.textContent = text; }) : createAloneView(canvas, id, m, room());
    canvas.setAttribute("aria-label", mode === "duel" ? `${m.name} in a duel, playing live` : `${m.name}, playing live`);
    if (mode === "alone") caption.textContent = "The effect on its own, on a plain floor.";
    view.restart(controls.options());
  };
  for (const input of root.querySelectorAll('input[name="effect-mode"]')) {
    input.addEventListener("change", () => input.checked && show(input.value));
  }
  show(MODES[0].value);

  let running = true;
  let last = null;
  const frame = (now) => {
    if (!running) return;
    if (!reduceMotion) view.step(last === null ? 0 : Math.max(0, now - last) / 1000);
    else view.step(0);
    last = now;
    requestAnimationFrame(frame);
  };
  requestAnimationFrame(frame);

  return () => {
    running = false;
    view.free();
  };
}

/** The controls in `root`: light, the effect's own options, palette and seed. Each change calls `changed`.
 *  `options()` is the engine options as set: only what differs from the defaults. */
function createControls(root, id, m, changed) {
  const opts = { seed: m.seed, light: 1, palette: "default" };
  const own = [...(OWN[id] ?? [])];
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

/** Canvas pixels per game pixel: as large as fits the column, whole numbers only so pixels stay square. */
function scaleFor(w, room, most) {
  return Math.max(1, Math.min(most, Math.floor(room / w)));
}

// ---------- alone ----------

/** The plain stage's box in game pixels: room round the effect for its glow, its origin standing on the floor 24
 *  pixels above the box's bottom (lower when the effect reaches further below its origin), its canvas centred. */
function aloneBox(m) {
  const pad = 24;
  const below = Math.max(24, m.size[1] - m.origin[1] + pad);
  const w = Math.max(200, m.size[0] + 2 * pad);
  const h = Math.max(160, m.origin[1] + pad + below);
  const at = { x: Math.round((w - m.size[0]) / 2) + m.origin[0], y: h - below };
  return { w, h, at, ground: at.y };
}

/** The plain box widened to fill `room` canvas pixels across at its whole-number scale, the effect still centred,
 *  so the canvas is shown at exactly its own size. */
function fitAlone(m, room) {
  const box = aloneBox(m);
  const scale = scaleFor(box.w, room, 5);
  const w = Math.max(box.w, Math.floor(room / scale));
  return { ...box, w, at: { ...box.at, x: box.at.x + Math.floor((w - box.w) / 2) }, scale };
}

/** The effect alone on `canvas`, on a flat sky and floor. */
function createAloneView(canvas, id, m, room) {
  const box = fitAlone(m, room);
  const { scale } = box;
  canvas.width = box.w * scale;
  canvas.height = box.h * scale;
  const ctx = canvas.getContext("2d");
  const stage = createPixfx(ctx, { scale });
  // reduced motion: nothing shakes, rings or flashes
  if (reduceMotion) stage.feel.setShakeStrength(0).setFlashStrength(0);
  let opts = {};
  let effect = null;
  let ended = false;
  let resting = 0;

  const view = {
    restart(o) {
      opts = o;
      effect?.destroy();
      ended = false;
      effect = stage.play(id, box.at.x, box.at.y, opts).on("complete", () => {
        ended = true;
      });
      resting = 0;
      // a still frame shows the effect a second in
      if (reduceMotion) stage.update(1);
    },
    step(dt) {
      stage.update(dt);
      // a one-shot is removed once it ends: rest a moment, then play it again
      if (!m.loop && ended && !reduceMotion) {
        resting += dt;
        if (resting >= REST) view.restart(opts);
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
  return view;
}

// ---------- in a duel ----------

/** The duel's box in game pixels and its scale, filling `room` canvas pixels across: 2 canvas pixels a game pixel,
 *  as the hero draws its fighters on a narrow screen, unless the column is too narrow to hold 160 game pixels. As
 *  tall as the effect over the floor, within reason and never taller than three quarters of its width, so a phone's
 *  stage is not mostly sky; the floor 20 pixels above the bottom. */
function duelBox(m, room) {
  const scale = room >= 320 ? 2 : 1;
  const w = Math.floor(room / scale);
  const h = Math.max(120, Math.min(200, m.size[1] + 40, Math.round(w * 0.75)));
  return { w, h, ground: h - 20, scale };
}

/** The effect staged by `role` in takes between Thunder and Frost on the mountain dais, on `canvas`; each take
 *  calls `told` with the bout's sentence for what it shows. */
function createDuelView(canvas, id, m, role, room, told) {
  const { w, h, ground, scale } = duelBox(m, room);
  canvas.width = w * scale;
  canvas.height = h * scale;
  const ctx = canvas.getContext("2d");
  const scene = createScene(w, h, ground);
  const stage = createPixfx(ctx, { scale });
  // the fight's own FightFeel answers every blow; the stage's effects just play the seconds it gives them
  stage.feel.setEnabled(false);
  const feel = new FightFeel();
  const sprites = new Sprites(w, h);
  const lighting = new Lighting(w, h);
  const bout = createBout({ left: 24, right: w - 24, ground, Body, seed: 1 + Math.floor(Math.random() * 1e6) });
  const isLoop = (fx) => meta(fx).loop;
  // a picture Rust painted (fighters, light), drawn and let go at once
  const show = (picture) => {
    stage.drawPicture(picture);
    picture?.free();
  };
  const handles = new Map();
  let lights = [];
  let opts = {};
  let resting = 0;
  // whether the take has struck: its effect's first impact cue, or the first blow it lands (a bolt that sends no cue)
  let struck = false;
  // the fighters as last painted, each fighter's also as the sprite an effect wrapped on it reads
  let painted = null;
  const bodies = [];

  /** The take's commands on the stage; each effect's impact cue tells the bout it landed. */
  function play(commands) {
    for (const c of commands) {
      if (c.type === "play") {
        const key = c.key;
        const handle = stage.play(c.fx, c.x, c.y, c.opts)
          .on("cue", () => {
            struck = true;
            impact(bout, key);
          })
          // each shot of a volley it was ordered to launch strikes where the bout aimed it
          .on("strike", () => {
            struck = true;
            impact(bout, key);
          })
          // a one-shot that ends on its own is let go
          .on("complete", () => handles.delete(key));
        // an effect that reads the sprite it plays on wraps the fighter that plays it
        if (meta(c.fx).wrapsActor) handle.wrap(() => bodies[c.by] ?? null);
        // a trail laid under it follows the way it moves
        if (c.trail) handle.trail(c.trail.id, c.trail.opts);
        handles.set(key, handle);
      } else if (c.type === "move" && handles.has(c.key)) handles.get(c.key).setPosition(c.x, c.y);
      else if (c.type === "order" && handles.has(c.key)) handles.get(c.key).attack(c.x, c.y, c.aim, c.caster);
      else if (c.type === "stop" && handles.has(c.key)) {
        handles.get(c.key).stop();
        handles.delete(c.key);
      }
    }
  }

  /** Each effect lights the dais in its own light's colour: `kindle` reads a ramp off a copy of each play. */
  const lit = (commands) =>
    commands.map((c) => {
      if (c.type !== "play") return c;
      const color = lightOf(c.fx, c.opts);
      return { ...c, opts: { ...c.opts, palette: color ? [color] : undefined } };
    });

  /** Let `dt` seconds of the take pass: the fight's feel freezes and slows it, the bout moves, the effects play. */
  function advance(dt) {
    const run = feel.advance(dt);
    const commands = step(bout, run);
    if (commands.some((c) => BLOWS.has(c.type))) struck = true;
    play(commands);
    feel.react(commands);
    lights = stepLights(kindle(lights, lit(commands), ground, isLoop, putOutLight), run);
    // the fighters are painted before the effects play, so an effect wrapped on one grips this frame's pose
    paint();
    declareBodies();
    stage.update(run);
  }

  /** Each standing fighter is a body the stage's volley pieces burst on (its index is its id), where it stands now. */
  function declareBodies() {
    for (const f of bout.fighters) {
      if (f.dead) {
        stage.clearBody(f.index);
        continue;
      }
      const tall = f.duck > 0 ? f.spec.duck : f.spec.body;
      stage.setBody(f.index, f.x - f.spec.halfWidth, ground - f.air - tall, f.spec.halfWidth * 2, tall);
    }
  }

  /** Paint the fighters as they stand now (the one a blow struck shudders through the hit-stop), and keep each
   *  one's sprite for the effects wrapped on it. */
  function paint() {
    if (painted) for (const picture of [...painted.fighters, painted.dust]) picture?.free();
    painted = paintFighters(sprites, bout, feel, ground, feel.jolts());
    for (const [i, picture] of painted.fighters.entries()) bodies[i] = actorOf(picture);
  }

  function draw() {
    ctx.fillStyle = BACKDROP;
    ctx.fillRect(0, 0, w * scale, h * scale);
    ctx.save();
    const [sx, sy] = feel.shake(reduceMotion ? 0 : 1);
    ctx.translate(sx * scale, sy * scale);
    drawScene(ctx, scene, bout.time, scale);
    stage.drawLayer("back");
    if (!painted) paint();
    // one struck down is gone: the effect that destroyed it draws what is left of it
    for (const [i, picture] of painted.fighters.entries()) if (!bout.fighters[i].dead) show(picture);
    show(painted.dust);
    painted = null;
    // the effect lights the dais and the fighters
    show(lighting.paint(lights, []));
    stage.drawLayer("front");
    // the qi a spell is thrown with, and what each blow did
    drawStreaks(ctx, feel.streaks(), bout, scale);
    drawNumbers(ctx, feel.numbers(), bout, scale);
    ctx.restore();
  }

  /** A new take with options `o`, one still running cut short. `caster` (a fighter's index) keeps the one who cast
   *  last; without it the fighters take turns. */
  function again(o, caster) {
    opts = o;
    take(bout, id, role, opts, { caster });
    told(`${bout.caption}.`);
    resting = 0;
    struck = false;
    // a still frame shows the take just after its impact
    if (reduceMotion) {
      for (let t = 0; t < STILL_SEARCH && !struck && !bout.done; t += 1 / 30) advance(1 / 30);
      for (let t = 0; t < STILL_AFTER && !bout.done; t += 1 / 30) advance(1 / 30);
    }
  }

  return {
    // the options changed: the same fighter casts again with them
    restart: (o) => again(o, bout.caster ?? undefined),
    step(dt) {
      // a still frame lets no time pass, but shows each frame the painters finish
      if (reduceMotion) advance(0);
      else {
        advance(dt);
        // a take ends once its effect has landed and both fighters stand still: rest a moment, then take again;
        // a fighter struck down is gone for good, so a death is not fought again until the options change
        if (bout.done && !bout.fighters.some((f) => f.dead)) {
          resting += dt;
          if (resting >= REST) again(opts);
        }
      }
      draw();
    },
    free() {
      stage.destroy();
      feel.free();
      sprites.free();
      lighting.free();
      if (painted) for (const picture of [...painted.fighters, painted.dust]) picture?.free();
    },
  };
}

/** The bout's events that are a blow landing. */
const BLOWS = new Set(["hit", "block", "clash"]);

function escape(text) {
  return text.replace(/[&<>"]/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" })[c]);
}
