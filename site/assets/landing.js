// The landing page: a live duel in the hero, drawn by the pixfx-canvas Stage. The page owns the animation-frame
// loop: each frame it draws the scene, the effects' back layers, the fighters, then the front layers.
import init, { Body, FightFeel, Lights, Sprites, Stage, effects, meta } from "../pkg/pixfx_canvas.js";
import { DECOR, createDuel, step } from "./duel.js";
import { BARS_TALL, drawBars, drawFighters, drawNumbers, drawSkills, drawStreaks } from "./duel-art.js";
import { createScene, drawScene } from "./duel-scene.js";
import { kindle, lampGlows } from "./duel-light.js";

/** Behind the hero's scene while the screen shakes. */
const BACKDROP = "#0b0a1c";
/** How tall the film bars an ultimate brings in are, in game pixels, at the sky's darkest. */
const LETTERBOX = 12;
const reduceMotion = matchMedia("(prefers-reduced-motion: reduce)").matches;

/** Mount the landing page: the tabs and the hero. Returns how to unmount it. */
export async function mount() {
  const off = new AbortController();
  setupTabs(document.getElementById("targets"));

  await init();

  const hero = createHero(document.getElementById("scene"), off.signal);
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

// ---------- hero ----------

function createHero(canvas, signal) {
  const view = { canvas, stage: null, duel: null, feel: null, sprites: null, lights: null, layout: null, handles: new Map() };
  const have = new Set(effects());
  let seed = 1 + Math.floor(Math.random() * 1e6);

  function build() {
    view.stage?.free();
    view.sprites?.free();
    view.feel?.free();
    view.lights?.free();
    view.handles.clear();
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

    view.stage = new Stage(canvas, scale);
    // the fight's own FightFeel answers every blow; the stage's effects just play the seconds it gives them
    view.stage.setFeel(false);
    // the night over the whole scene: one fireflies canvas its size, the meadow on the dais (the effect's ground
    // is mid-width, 10 pixels above its canvas's foot)
    if (have.has(DECOR.night)) view.stage.play(DECOR.night, Math.floor(w / 2), ground, { width: w, height: ground + 10 });
    // a brazier at each end of the arena lights it
    if (have.has(DECOR.lamp)) for (const x of [arena.left, arena.right]) view.stage.play(DECOR.lamp, x, ground, { seed: x });
    view.feel = new FightFeel();
    view.sprites = new Sprites(canvas, scale);
    view.lights = new Lights(w, h);
    // the bars go just under the page's top bar, which lies over the hero
    // (its links can wrap past its own box on a narrow screen, so measure what is in it)
    const topbar = Math.max(0, ...[...document.querySelectorAll(".topbar, .topbar *")].map((el) => el.getBoundingClientRect().bottom));
    const barTop = Math.max(6, Math.ceil((topbar - rect.top) / scale) + 6);
    // fighters fly no higher than keeps their heads (about 44 pixels tall) under the bars
    const sky = ground - (barTop + BARS_TALL + 13) - 46;
    view.duel = createDuel({ seed: seed++, left: arena.left + lampInset, right: arena.right - lampInset, ground, has: (id) => have.has(id), sky, Body });
    view.layout = { scale, w, h, ground, barTop, lamps: [arena.left, arena.right], scene: createScene(w, h, ground, barTop + BARS_TALL) };
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
      else if (c.type === "move" && view.handles.has(c.key)) view.stage.move(view.handles.get(c.key), c.x, c.y);
      else if (c.type === "stop" && view.handles.has(c.key)) {
        view.stage.stop(view.handles.get(c.key));
        view.handles.delete(c.key);
      }
    }
  }

  view.step = (dt) => {
    const { scale, w, h, ground, scene } = view.layout;
    // a blow stops the fight and its effects together for a few frames; a knockout plays out slowed
    const run = view.feel.advance(dt);
    const commands = step(view.duel, run);
    play(commands);
    view.feel.react(commands);
    kindle(view.lights, commands, ground, (fx) => meta(fx).loop);
    view.lights.step(run);
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
    drawFighters(view.sprites, view.duel, view.feel, ground, view.feel.jolts());
    // the magic lights the dais, the peaks and the fighters in its own colour
    view.lights.draw(ctx, scale, lampGlows(view.layout.lamps, ground, view.duel.time));
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
    drawBars(ctx, view.sprites, view.duel, scale, view.layout.barTop);
    // each fighter's skills, on the floor under its own side
    drawSkills(ctx, view.duel, scale, ground, h);
  };

  let pending = null;
  addEventListener("resize", () => {
    clearTimeout(pending);
    pending = setTimeout(build, 150);
  }, { signal });
  signal.addEventListener("abort", () => clearTimeout(pending));
  view.free = () => {
    view.stage?.free();
    view.sprites?.free();
    view.feel?.free();
    view.lights?.free();
  };
  view.playing = () => view.stage?.handles().length ?? 0;

  build();
  return view;
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
