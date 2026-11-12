// Candle, campfire and brazier side by side on a dark ground, drawn by the pixfx-canvas Stage. The page owns the
// loop: ground, the effects' back layers, a pixel figure standing in the campfire, then the front layers. Click: play a one-shot effect there if the registry has one, else restart the nearest effect.
// Build first: wasm-pack build crates/pixfx-canvas --target web --out-dir demo/pkg
import init, { Stage, effects, meta } from "./pkg/pixfx_canvas.js";

const SCALE = 4;
const GAME = { w: 336, h: 180 }; // game pixels
const GROUND = 160;

await init();
const canvas = document.getElementById("stage");
const status = document.getElementById("status");
canvas.width = GAME.w * SCALE;
canvas.height = GAME.h * SCALE;

const stage = new Stage(canvas, SCALE);
const ctx = canvas.getContext("2d");
const placed = [
  { id: "fire/candle", x: 60, y: GROUND },
  { id: "fire/campfire", x: 168, y: GROUND },
  { id: "fire/brazier", x: 280, y: GROUND },
].map((p) => ({ ...p, handle: stage.play(p.id, p.x, p.y, { seed: 1 }) }));

// A pixel figure (game pixels, feet at its x, y), drawn by the page between the layers.
const FIGURE = { x: 150, y: GROUND, color: "#4fd1c5", w: 8, h: 18 };
function drawFigure() {
  ctx.fillStyle = FIGURE.color;
  ctx.fillRect((FIGURE.x - FIGURE.w / 2) * SCALE, (FIGURE.y - FIGURE.h) * SCALE, FIGURE.w * SCALE, FIGURE.h * SCALE);
}

let last = null;
function frame(now) {
  const dt = last === null ? 0 : Math.max(0, now - last) / 1000;
  last = now;
  stage.update(dt);
  ctx.fillStyle = "#17131c";
  ctx.fillRect(0, 0, canvas.width, canvas.height);
  stage.drawLayer("back");
  drawFigure();
  stage.drawLayer("front");
  requestAnimationFrame(frame);
}
requestAnimationFrame(frame);

const oneShot = effects().find((id) => !meta(id).loop);

// window.pixfxDemo lets a test harness read what the page is doing.
const stats = { stage, figure: FIGURE, placed, oneShot: oneShot ?? null, clicks: [], fps: 0 };
window.pixfxDemo = stats;

canvas.addEventListener("click", (e) => {
  const x = (e.offsetX * canvas.width) / canvas.clientWidth / SCALE;
  const y = (e.offsetY * canvas.height) / canvas.clientHeight / SCALE;
  if (oneShot) {
    stats.clicks.push({ x, y, spawned: oneShot, handle: stage.play(oneShot, x, y, undefined) });
    return;
  }
  const near = placed.reduce((a, b) => (Math.hypot(b.x - x, b.y - y) < Math.hypot(a.x - x, a.y - y) ? b : a));
  const was = near.handle;
  stage.remove(was);
  near.handle = stage.play(near.id, near.x, near.y, { seed: 1 });
  stats.clicks.push({ x, y, restarted: near.id, was, handle: near.handle });
});

// Display rate, measured in the page; the page draws once per animation frame.
let windowStart = performance.now();
let frames = 0;
function count(now) {
  frames += 1;
  if (now - windowStart >= 1000) {
    stats.fps = Math.round((frames * 1000) / (now - windowStart));
    windowStart = now;
    frames = 0;
    const last = stats.clicks.at(-1);
    const click = last ? ` · click: ${last.spawned ? `spawned ${last.spawned}` : `restarted ${last.restarted}`}` : "";
    status.textContent = `${placed.length} effects · scale ${SCALE} · ${stats.fps} fps${click}`;
  }
  requestAnimationFrame(count);
}
requestAnimationFrame(count);
