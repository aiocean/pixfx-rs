// The site's own fx, drawn by the engine: a trail behind the pointer, drawn along the way it moved
// (`projectiles/trail` played on its own and moved, so it reads that motion). It switches on only when the trail is in
// the registry (`effects()`).
// Off with prefers-reduced-motion, or with the "Fx" switch in the top bar (remembered in localStorage), which shows
// only when the trail is in the registry.
import { createPixfx, effects, loadPixfx } from "../pkg/pixfx-canvas2d.js";

const FX = { cursor: "projectiles/trail" };
/** Game pixel to CSS pixels. */
const SCALE = 2;
const SWITCH = "pixfx-fx";

const reduced = matchMedia("(prefers-reduced-motion: reduce)").matches;

/**
 * Start the site's fx. Returns whether they are on, the page transition (none), and `decorate`, which puts the fx
 * switch into the page now in the document (call it after every page mount).
 */
export async function startFx() {
  await loadPixfx();
  const have = new Set(effects());
  const enabled = !reduced && localStorage.getItem(SWITCH) !== "off";
  if (enabled && have.has(FX.cursor) && matchMedia("(pointer: fine)").matches) followPointer(createOverlay());
  return { enabled, transition: null, decorate: () => mountSwitch(have) };
}

// ---------- overlay: one transparent canvas over the page for pointer fx ----------

function createOverlay() {
  const canvas = Object.assign(document.createElement("canvas"), { className: "fx-overlay" });
  canvas.setAttribute("aria-hidden", "true");
  document.body.append(canvas);
  const ctx = canvas.getContext("2d");
  const stage = createPixfx(ctx, { scale: SCALE });
  // pointer fx decorate the page: they never shake or flash it
  stage.feel.setEnabled(false);
  const fit = () => {
    canvas.width = Math.ceil(innerWidth / SCALE) * SCALE;
    canvas.height = Math.ceil(innerHeight / SCALE) * SCALE;
    // one canvas pixel per CSS pixel, so game pixels stay whole
    canvas.style.width = `${canvas.width}px`;
    canvas.style.height = `${canvas.height}px`;
  };
  fit();
  addEventListener("resize", fit);
  // the overlay's own loop: the effects play, and the canvas shows only them
  let last = null;
  const frame = (now) => {
    stage.update(last === null ? 0 : Math.max(0, now - last) / 1000);
    last = now;
    ctx.clearRect(0, 0, canvas.width, canvas.height);
    stage.draw();
    requestAnimationFrame(frame);
  };
  requestAnimationFrame(frame);
  // pointer positions in CSS pixels to the stage's game pixels
  const at = (x, y) => [x / SCALE, y / SCALE];
  return { stage, at };
}

/** The trail starts where the pointer enters and is moved with it; it ends when the pointer leaves the page. */
function followPointer({ stage, at }) {
  let handle = null;
  addEventListener("pointermove", (e) => {
    if (e.pointerType !== "mouse") return;
    const [x, y] = at(e.clientX, e.clientY);
    if (handle === null) handle = stage.play(FX.cursor, x, y);
    else handle.setPosition(x, y);
  });
  document.addEventListener("mouseleave", () => {
    handle?.stop();
    handle = null;
  });
}

// ---------- the switch ----------

function mountSwitch(have) {
  const nav = document.querySelector(".topbar nav");
  // nothing to switch until the site's fx is in the registry
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
