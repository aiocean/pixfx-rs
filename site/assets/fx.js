// The site's own fx, every one a pixfx effect drawn by the engine: a wisp that follows the pointer, a burst where
// you click, a poof when a button is pressed, and a burn between pages (the old page burns open from where you
// clicked, showing the new page under it). Each switches on only when its effect is in the registry (`effects()`),
// so the site lights up as effects are ported, with no change here.
// Off with prefers-reduced-motion, or with the "Fx" switch in the top bar (remembered in localStorage), which shows
// only once at least one of these effects is in the registry.
import init, { Fx, Stage, effects } from "../pkg/pixfx_canvas.js";

const FX = {
  cursor: "critters/wisp",
  click: "impacts/parry",
  press: "spawn/poof",
  transition: "transitions/burn-away",
};
/** Game pixel to CSS pixels for the cursor, click and press fx. */
const SCALE = 2;
/** Where the wisp's origin goes relative to the pointer, in game pixels. The wisp hovers 36 pixels over its origin
 *  (its `hover` option), so this puts it just up and right of the pointer's tip, clear of the arrow. */
const WISP_OFFSET = { x: 4, y: 28 };
/** How far above its origin parry's blades clash, in game pixels (its `height` option). */
const CLASH_HEIGHT = 30;
/** burn-away's screen, and when its cover starts to smoulder and burn open from the hole (pixelfx
 *  src/fx/transitions/burn-away.js, SMOULDER). The page change plays only that part: the cover is the old page. */
const SCREEN = { w: 320, h: 180 };
const SMOULDER = 1.5;
/** Page changes play faster than the effect's own time so reading the docs never waits on them. */
const TRANSITION_SPEED = 1.5;
/** burn-away's cover colour: the site never shows it, it marks which pixels are still the old page. */
const COVER = "#13101b";
const SWITCH = "pixfx-fx";

const reduced = matchMedia("(prefers-reduced-motion: reduce)").matches;

/**
 * Start the site's fx. Returns whether they are on, the page transition when burn-away is in the registry, and
 * `decorate`, which puts the fx switch into the page now in the document (call it after every page mount).
 */
export async function startFx() {
  await init();
  const have = new Set(effects());
  const enabled = !reduced && localStorage.getItem(SWITCH) !== "off";
  if (enabled) {
    const overlay = createOverlay();
    if (have.has(FX.cursor) && matchMedia("(pointer: fine)").matches) followPointer(overlay);
    if (have.has(FX.click)) burstOnClick(overlay);
    if (have.has(FX.press)) poofOnPress(overlay);
  }
  return {
    enabled,
    transition: enabled && have.has(FX.transition) ? burnOpen : null,
    decorate: () => mountSwitch(have),
  };
}

// ---------- overlay: one transparent canvas over the page for pointer fx ----------

function createOverlay() {
  const canvas = Object.assign(document.createElement("canvas"), { className: "fx-overlay" });
  canvas.setAttribute("aria-hidden", "true");
  document.body.append(canvas);
  const stage = new Stage(canvas, SCALE);
  const fit = () => {
    canvas.width = Math.ceil(innerWidth / SCALE) * SCALE;
    canvas.height = Math.ceil(innerHeight / SCALE) * SCALE;
    // one canvas pixel per CSS pixel, so game pixels stay whole
    canvas.style.width = `${canvas.width}px`;
    canvas.style.height = `${canvas.height}px`;
  };
  fit();
  addEventListener("resize", fit);
  stage.start();
  // pointer positions in CSS pixels to the stage's game pixels
  const at = (x, y) => [x / SCALE, y / SCALE];
  return { stage, at };
}

function followPointer({ stage, at }) {
  let handle = null;
  let target = null;
  let pos = null;
  addEventListener("pointermove", (e) => {
    if (e.pointerType !== "mouse") return;
    target = at(e.clientX, e.clientY);
    if (handle === null) {
      pos = target;
      handle = stage.play(FX.cursor, pos[0] + WISP_OFFSET.x, pos[1] + WISP_OFFSET.y, undefined);
    }
  });
  document.addEventListener("mouseleave", () => {
    if (handle !== null) stage.stop(handle);
    handle = null;
  });
  // the wisp eases toward the pointer instead of sticking to it
  const step = () => {
    if (handle !== null && target) {
      pos = [pos[0] + (target[0] - pos[0]) * 0.18, pos[1] + (target[1] - pos[1]) * 0.18];
      stage.move(handle, pos[0] + WISP_OFFSET.x, pos[1] + WISP_OFFSET.y);
    }
    requestAnimationFrame(step);
  };
  requestAnimationFrame(step);
}

function burstOnClick({ stage, at }) {
  addEventListener("click", (e) => {
    // keyboard activation has no pointer position (detail 0): the press fx covers it on buttons
    if (e.detail === 0) return;
    const [x, y] = at(e.clientX, e.clientY);
    // parry's origin is where a fighter stands; the blades clash `height` above it. Play it that far below the
    // pointer so the clash is on the pointer.
    stage.play(FX.click, x, y + CLASH_HEIGHT, { seed: Math.floor(Math.random() * 1e6), height: CLASH_HEIGHT });
  });
}

function poofOnPress({ stage, at }) {
  const pressable = ".button, button, [role='tab'], .controls select";
  addEventListener("pointerdown", (e) => {
    const el = e.target.closest?.(pressable);
    if (!el) return;
    const r = el.getBoundingClientRect();
    const [x, y] = at(r.left + r.width / 2, r.bottom);
    stage.play(FX.press, x, y, { seed: Math.floor(Math.random() * 1e6) });
  });
}

// ---------- page transition: the old page burns open, the new one is under it ----------

/**
 * Burn `layer` (the old page, on top of the new one) open from `at` (CSS pixels; null: burn-away's own spot).
 * burn-away's cover is the old page: where the effect draws its cover colour the layer shows (a CSS mask), and
 * everything else it draws (the burning edge, char, embers, smoke) goes on a canvas above, its light blended
 * additively on a second one. The 320x180 screen is scaled by a whole number to cover the viewport, centred.
 */
async function burnOpen(layer, at) {
  const { w: W, h: H } = SCREEN;
  const scale = Math.max(1, Math.ceil(Math.max(innerWidth / W, innerHeight / H)));
  const left = Math.round((innerWidth - W * scale) / 2);
  const top = Math.round((innerHeight - H * scale) / 2);
  const hole = at ? { holeX: Math.round((at.x - left) / scale), holeY: Math.round((at.y - top) / scale) } : {};
  const fx = new Fx(FX.transition, { margin: 0, color: COVER, seed: Math.floor(Math.random() * 1e6), ...hole });
  for (let t = 0; t < SMOULDER; t += 0.25) fx.update(Math.min(0.25, SMOULDER - t));

  const screen = (extra) => {
    const c = Object.assign(document.createElement("canvas"), { className: `fx-screen ${extra}`, width: W * scale, height: H * scale });
    c.setAttribute("aria-hidden", "true");
    Object.assign(c.style, { left: `${left}px`, top: `${top}px`, width: `${W * scale}px`, height: `${H * scale}px` });
    document.body.append(c);
    const ctx = c.getContext("2d");
    ctx.imageSmoothingEnabled = false;
    return { c, ctx };
  };
  const fire = screen("");
  const light = screen("fx-light");
  // one game pixel per canvas pixel; ImageData words are RGBA bytes, the engine's ABGR words on little-endian
  const pixels = () => {
    const c = Object.assign(document.createElement("canvas"), { width: W, height: H });
    const img = new ImageData(W, H);
    return { c, ctx: c.getContext("2d"), img, words: new Uint32Array(img.data.buffer) };
  };
  const colour = pixels();
  const glow = pixels();
  const mask = pixels();
  const cover = coverWord(COVER);
  Object.assign(layer.style, { maskSize: `${W * scale}px ${H * scale}px`, maskPosition: `${left}px ${top}px` });

  const draw = () => {
    fx.draw("both");
    const col = fx.color();
    const lit = fx.light();
    for (let i = 0; i < col.length; i++) {
      const isCover = col[i] === cover;
      colour.words[i] = isCover ? 0 : col[i];
      mask.words[i] = isCover ? 0xff000000 : 0;
      const l = lit[i] & 0xffffff;
      glow.words[i] = l ? l | 0xff000000 : 0;
    }
    for (const [src, dst] of [[colour, fire], [glow, light]]) {
      src.ctx.putImageData(src.img, 0, 0);
      dst.ctx.clearRect(0, 0, dst.c.width, dst.c.height);
      dst.ctx.drawImage(src.c, 0, 0, dst.c.width, dst.c.height);
    }
    mask.ctx.putImageData(mask.img, 0, 0);
    layer.style.maskImage = `url(${mask.c.toDataURL()})`;
  };

  draw();
  await new Promise((resolve) => {
    let last = null;
    const frame = (now) => {
      const dt = last === null ? 0 : Math.max(0, now - last) / 1000;
      last = now;
      fx.update(Math.min(0.25, dt * TRANSITION_SPEED));
      draw();
      if (fx.done()) resolve();
      else requestAnimationFrame(frame);
    };
    requestAnimationFrame(frame);
  });
  fire.c.remove();
  light.c.remove();
  fx.free();
}

/** A "#rrggbb" colour as the engine's opaque ABGR word. */
function coverWord(hex) {
  const n = parseInt(hex.slice(1), 16);
  return (0xff000000 | ((n & 0xff) << 16) | (n & 0xff00) | (n >> 16)) >>> 0;
}

// ---------- the switch ----------

function mountSwitch(have) {
  const nav = document.querySelector(".topbar nav");
  // nothing to switch until at least one of the site's fx is in the registry
  if (!nav || reduced || nav.querySelector(".fx-switch") || !Object.values(FX).some((id) => have.has(id))) return;
  const button = Object.assign(document.createElement("button"), { type: "button", className: "fx-switch" });
  const on = localStorage.getItem(SWITCH) !== "off";
  button.textContent = on ? "Fx on" : "Fx off";
  button.setAttribute("aria-pressed", String(on));
  button.addEventListener("click", () => {
    localStorage.setItem(SWITCH, on ? "off" : "on");
    location.reload();
  });
  nav.append(button);
}
