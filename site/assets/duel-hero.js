// The live duel both sites play (the pixfx landing page's hero and the arena's pages): the rules through the engine's
// wasm (`duel.js`), drawn by pixfx-canvas2d (workers paint the effects), with the fighters, the scene's light, the HUD
// and its tooltips. The page owns the animation-frame loop and calls `step(dt)` each frame.
import { FightFeel, Lighting, Sprites, createPixfx, lookColors, putOutLight, stepLights } from "../pkg/pixfx-canvas2d.js";
import { DECOR, RAGE, createDuel, freeDuel, step } from "./duel.js";
import { BARS_TALL, actorOf, drawBars, drawNumbers, drawSkills, drawStreaks, hudAt, paintFighters } from "./duel-art.js";
import { buffInfo, skillInfo } from "./duel-info.js";
import { createScene, drawScene } from "./duel-scene.js";
import { kindle, lampGlows } from "./duel-light.js";

/** Behind the hero's scene while the screen shakes. */
const BACKDROP = "#0b0a1c";
/** How tall the film bars an ultimate brings in are, in game pixels, at the sky's darkest. */
const LETTERBOX = 12;
/** The effects that wrap a fighter's own sprite, while its ultimate is ready: a surge up its body when its rage fills,
 *  then a ring in its school's glow round every pose until it casts. */
const WRAP = { ring: "feedback/outline", surge: "feedback/power-up" };
/** Where a fighter's ring plays, above its feet: about the middle of its body, so the ring's box holds it in any pose. */
const RING_MID = 24;
const reduceMotion = matchMedia("(prefers-reduced-motion: reduce)").matches;

/** Fetch each named weights file in `pkg/` (beside the wasm) as bytes, `[left, right]`; a file named twice is fetched
 *  once. */
export async function loadPolicies(files) {
  const names = [...new Set(files)];
  const buffers = await Promise.all(
    names.map((name) =>
      fetch(new URL(`../pkg/${name}`, import.meta.url)).then((r) =>
        r.ok ? r.arrayBuffer() : Promise.reject(new Error(`${name}: HTTP ${r.status}`)),
      ),
    ),
  );
  const bytes = new Map(names.map((name, i) => [name, new Uint8Array(buffers[i])]));
  return files.map((name) => bytes.get(name));
}

// ---------- hero ----------

/**
 * The live duel on `canvas`: the arena sized to the canvas, the rules from `duel.js`, the fighters, effects, light, HUD
 * and tooltips. `seed` is the first duel's (a random one when left out); each later rebuild, on a resize, takes the
 * next, or the same one again with `replay` (a page that shows one seed's fight). Returns the view: `step(dt)` draws a frame, `restart(seed)` begins a fresh duel on that seed (with the
 * policies `usePolicies` seated last), `seed()` is the seed of the duel now playing, `free()` lets it all go.
 */
export function createHero(canvas, signal, icons, engine, { seed: first, replay = false } = {}) {
  const view = { canvas, icons, stage: null, duel: null, feel: null, sprites: null, lighting: null, lights: [], layout: null, handles: new Map(), wrapped: [] };
  const have = new Set(engine.effects());
  // a picture Rust painted (fighters, faces, light), drawn and let go at once
  const show = (picture) => {
    view.stage.drawPicture(picture);
    picture?.free();
  };
  let seed = first ?? 1 + Math.floor(Math.random() * 1e6);
  let playing = seed;

  function build() {
    view.free();
    view.handles.clear();
    view.wrapped = [];
    view.lights = [];
    const narrow = innerWidth < 760;
    // the fighters stay big enough to read their poses: bigger pixels on a wider screen
    const scale = narrow ? 2 : innerWidth < 1200 ? 3 : 4;
    const rect = canvas.getBoundingClientRect();
    const w = Math.ceil(rect.width / scale);
    const h = Math.ceil(rect.height / scale);
    canvas.width = w * scale;
    canvas.height = h * scale;
    const ground = h - (narrow ? 14 : 22);
    // the arena is the middle of the dais, at most 420 pixels wide, a brazier at each end
    const lampInset = 14;
    const span = Math.min(w - 16, 420);
    const arena = { left: Math.round((w - span) / 2), right: Math.round((w + span) / 2) };

    view.stage = createPixfx(canvas.getContext("2d"), { scale });
    // the fight's own FightFeel answers every blow; the stage's effects just play the seconds it gives them
    view.stage.feel.setEnabled(false);
    // the night over the whole scene: one fireflies canvas its size, the meadow on the dais (the effect's ground
    // is mid-width, 10 pixels above its canvas's foot)
    if (have.has(DECOR.night)) view.stage.play(DECOR.night, Math.floor(w / 2), ground, { width: w, height: ground + 10 });
    // a brazier at each end of the arena lights it
    if (have.has(DECOR.lamp)) for (const x of [arena.left, arena.right]) view.stage.play(DECOR.lamp, x, ground, { seed: x });
    view.feel = new FightFeel();
    view.sprites = new Sprites(w, h);
    view.lighting = new Lighting(w, h);
    // the bars go just under the page's top bar, which lies over the hero
    // (its links can wrap past its own box on a narrow screen, so measure what is in it)
    const topbar = Math.max(0, ...[...document.querySelectorAll(".topbar, .topbar *")].map((el) => el.getBoundingClientRect().bottom));
    const barTop = Math.max(6, Math.ceil((topbar - rect.top) / scale) + 6);
    // fighters fly no higher than keeps their heads (about 44 pixels tall) under the bars
    const sky = ground - (barTop + BARS_TALL + 13) - 46;
    if (view.duel) freeDuel(view.duel);
    playing = seed;
    view.duel = createDuel({ seed: replay ? seed : seed++, left: arena.left + lampInset, right: arena.right - lampInset, ground, has: (id) => have.has(id), sky });
    // each fighter's sprite as it was last painted, which the effects wrapped on it read
    view.wrapped = view.duel.fighters.map((f) => {
      return { body: null, rage: f.rage, surge: null, ring: null };
    });
    // each fighter's skills, on the floor under its own side: bigger icons where the floor has the room
    const icon = h - ground >= 20 ? 16 : 12;
    view.layout = { scale, w, h, ground, barTop, icon, lamps: [arena.left, arena.right], scene: createScene(w, h, ground, barTop + BARS_TALL) };
    // a still frame shows the fight under way
    if (reduceMotion) {
      for (let i = 0; i < 6 * 30; i++) play(step(view.duel, 1 / 30));
      view.step(0);
    }
  }

  /** The duel's effect commands, on the stage. */
  function play(commands) {
    for (const c of commands) {
      if (c.type === "play") view.handles.set(c.key, view.stage.play(c.fx, c.x, c.y, c.opts));
      else if (c.type === "move" && view.handles.has(c.key)) view.handles.get(c.key).setPosition(c.x, c.y);
      else if (c.type === "stop" && view.handles.has(c.key)) {
        view.handles.get(c.key).stop();
        view.handles.delete(c.key);
      }
    }
  }

  /** Hand each fighter's new pose to the effects wrapped on it and keep them on it. When its rage fills: a surge, and a
   *  ring that stays until the rage is spent or the fighter is down. */
  function wrapFighters(pictures) {
    const { ground } = view.layout;
    for (const f of view.duel.fighters) {
      const one = view.wrapped[f.index];
      if (!one) continue;
      one.body = actorOf(pictures[f.index]);
      const feet = ground - f.air;
      const ready = f.rage >= RAGE && f.hp > 0;
      if (ready && one.rage < RAGE && have.has(WRAP.surge)) {
        one.surge?.stop();
        const surge = view.stage.play(WRAP.surge, f.x, feet);
        surge.wrap(() => one.body).once("complete", () => {
          if (one.surge === surge) one.surge = null;
        });
        one.surge = surge;
      }
      if (ready && !one.ring && have.has(WRAP.ring)) {
        one.ring = view.stage.play(WRAP.ring, f.x, feet - RING_MID, { palette: [lookColors(f.school).glow] });
        one.ring.wrap(() => one.body);
      } else if (!ready && one.ring) {
        one.ring.stop();
        one.ring = null;
      }
      one.ring?.setPosition(f.x, feet - RING_MID);
      one.surge?.setPosition(f.x, feet);
      one.rage = f.rage;
    }
  }

  view.step = (dt) => {
    const { scale, w, h, ground, scene } = view.layout;
    // a blow stops the fight and its effects together for a few frames; a knockout plays out slowed
    const run = view.feel.advance(dt);
    const commands = step(view.duel, run);
    play(commands);
    view.feel.react(commands);
    view.lights = stepLights(kindle(view.lights, commands, ground, (fx) => engine.meta(fx).loop, putOutLight), run);
    // the fighters are painted before the effects play, so an effect wrapped on one grips this frame's pose
    const painted = paintFighters(view.sprites, view.duel, view.feel, ground, view.feel.jolts());
    wrapFighters(painted.fighters);
    view.stage.update(reduceMotion ? 0 : run);
    const ctx = canvas.getContext("2d");
    ctx.fillStyle = BACKDROP;
    ctx.fillRect(0, 0, w * scale, h * scale);
    ctx.save();
    const [sx, sy] = view.feel.shake(1);
    ctx.translate(sx * scale, sy * scale);
    drawScene(ctx, scene, view.duel.time, scale);
    view.stage.drawLayer("back");
    // an ultimate darkens the world behind the fighters and the magic
    const dark = view.feel.dusk();
    if (dark > 0) {
      ctx.fillStyle = `rgba(4, 2, 10, ${dark})`;
      ctx.fillRect(-scale * 4, -scale * 4, (w + 8) * scale, (h + 8) * scale);
    }
    // the one a blow struck shudders through the hit-stop
    for (const picture of painted.fighters) show(picture);
    show(painted.dust);
    // the magic lights the dais, the peaks and the fighters in its own colour
    show(view.lighting.paint(view.lights, lampGlows(view.layout.lamps, ground, view.duel.time)));
    view.stage.drawLayer("front");
    drawStreaks(ctx, view.feel.streaks(), view.duel, scale);
    drawNumbers(ctx, view.feel.numbers(), view.duel, scale);
    ctx.restore();
    // an ultimate frames the screen like a film for as long as the sky is dark
    if (dark > 0) {
      const band = Math.round((dark / FightFeel.duskDepth()) * LETTERBOX);
      ctx.fillStyle = "#000";
      ctx.fillRect(0, 0, w * scale, band * scale);
      ctx.fillRect(0, (h - band) * scale, w * scale, band * scale);
    }
    drawBars(ctx, view.sprites, show, view.duel, scale, view.layout.barTop);
    // each fighter's skills, from the atlas whose pixels land on whole screen pixels
    const { icon } = view.layout;
    drawSkills(ctx, view.duel, scale, h, icon, (icon * scale) % 32 === 0 ? view.icons[32] : view.icons[24]);
    // what the pointer holds changes with the fight: its tooltip follows
    view.tip.update();
  };

  let pending = null;
  addEventListener("resize", () => {
    clearTimeout(pending);
    pending = setTimeout(build, 150);
  }, { signal });
  signal.addEventListener("abort", () => clearTimeout(pending));
  view.free = () => {
    view.stage?.destroy();
    view.sprites?.free();
    view.feel?.free();
    view.lighting?.free();
    if (view.duel) freeDuel(view.duel);
    view.duel = null;
  };
  view.playing = () => view.stage?.stats().engines ?? 0;
  view.seed = () => playing;
  view.restart = (s) => {
    seed = s;
    build();
  };

  build();
  view.tip = createTip(view, signal);
  return view;
}

// ---------- the hero's tooltips ----------

/** A tooltip's nodes, laid out as in Path of Exile: a header with the name and the type line, then the properties,
 *  what it does and the mods, each part set off by a rule. */
function tipNodes(info) {
  const make = (tag, className, text) => Object.assign(document.createElement(tag), { className, textContent: text ?? "" });
  const head = make("header", "tip-head");
  head.append(make("h3", "", info.title));
  if (info.type) head.append(make("p", "tip-type", info.type));
  const parts = [];
  if (info.props.length > 0) {
    parts.push(info.props.map(([label, value]) => {
      const line = make("p", "tip-prop", `${label}: `);
      line.append(make("span", "", value));
      return line;
    }));
  }
  parts.push([make("p", "tip-text", info.text)]);
  if (info.mods.length > 0) parts.push(info.mods.map((mod) => make("p", "tip-mod", mod)));
  const body = make("div", "tip-body");
  parts.forEach((nodes, i) => body.append(...(i > 0 ? [make("hr", "")] : []), ...nodes));
  return [head, body];
}

/**
 * Hover a skill slot or a buff tile (or tap it) to see what it is: a panel by the pointer, kept up to date while the
 * pointer stays. A tap pins it; a tap anywhere else, or a scroll, lets it go.
 */
function createTip(view, signal) {
  const { canvas } = view;
  const el = document.createElement("div");
  el.className = "duel-tip";
  el.hidden = true;
  el.setAttribute("role", "tooltip");
  document.body.append(el);
  signal.addEventListener("abort", () => el.remove());
  // where the pointer is, in the page, and whether a tap pinned it there
  let at = null;
  let shown = "";
  const target = () => {
    if (!at || !view.duel || !view.layout) return null;
    const r = canvas.getBoundingClientRect();
    const { scale, barTop, h, icon } = view.layout;
    const gx = (((at.cx - r.left) / r.width) * canvas.width) / scale;
    const gy = (((at.cy - r.top) / r.height) * canvas.height) / scale;
    return hudAt(view.duel, gx, gy, barTop, h, icon);
  };
  const update = () => {
    const hit = target();
    canvas.style.cursor = hit ? "pointer" : "";
    const f = hit && view.duel.fighters[hit.fighter];
    const info = !hit ? null : hit.kind === "skill" ? skillInfo(f, hit.slot) : buffInfo(hit.buff);
    if (!info) {
      el.hidden = true;
      shown = "";
      return;
    }
    const key = JSON.stringify(info);
    if (key !== shown) {
      shown = key;
      el.replaceChildren(...tipNodes(info));
      el.dataset.tone = info.tone ?? "plain";
    }
    // drawn in the duel's own pixels: one of its pixels, in page pixels
    const r = canvas.getBoundingClientRect();
    el.style.setProperty("--px", `${Math.max(2, Math.round((view.layout.scale * r.width) / canvas.width))}px`);
    el.hidden = false;
    // beside the pointer, kept on the screen
    const pad = 14;
    const x = Math.max(4, Math.min(at.cx + pad, innerWidth - el.offsetWidth - 4));
    const y = at.cy + pad + el.offsetHeight > innerHeight - 4 ? at.cy - el.offsetHeight - pad : at.cy + pad;
    el.style.left = `${x}px`;
    el.style.top = `${Math.max(4, y)}px`;
  };
  canvas.addEventListener("pointermove", (e) => {
    if (e.pointerType !== "mouse" || at?.pinned) return;
    at = { cx: e.clientX, cy: e.clientY, pinned: false };
    update();
  }, { signal });
  canvas.addEventListener("pointerleave", () => {
    if (at?.pinned) return;
    at = null;
    update();
  }, { signal });
  canvas.addEventListener("click", (e) => {
    at = { cx: e.clientX, cy: e.clientY, pinned: true };
    if (!target()) at = null;
    update();
  }, { signal });
  document.addEventListener("click", (e) => {
    if (e.target === canvas || !at?.pinned) return;
    at = null;
    update();
  }, { signal });
  addEventListener("scroll", () => {
    if (!at) return;
    at = null;
    update();
  }, { signal, passive: true });
  return { update };
}

/** Each skill icon atlas, by the size of its icons: `{ [size]: { image, size } }`. */
export async function loadIcons(sizes) {
  const atlases = await Promise.all(sizes.map(async (size) => {
    const image = new Image();
    image.src = new URL(`skills-${size}.png`, import.meta.url).href;
    await image.decode();
    return [size, { image, size }];
  }));
  return Object.fromEntries(atlases);
}

