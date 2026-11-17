// Plays projectiles/trail through the pixfx-wasm engine: JS steps the effect and blits its pixels each frame.
// Build first: wasm-pack build crates/pixfx-wasm --target web --out-dir demo/pkg
import init, { Fx, effects, meta } from "./pkg/pixfx_wasm.js";

const ID = "projectiles/trail";

await init();
const info = meta(ID);
const fx = new Fx(ID, { seed: 1 });
const canvas = document.getElementById("fx");
const status = document.getElementById("status");
canvas.width = fx.width();
canvas.height = fx.height();
canvas.style.width = `${fx.width() * 4}px`;
const ctx = canvas.getContext("2d");

// window.pixfxDemo lets a test harness read what the page is doing.
const stats = { id: ID, effects: effects(), meta: info, frames: 0, fps: 0 };
window.pixfxDemo = stats;

let last = performance.now();
let windowStart = last;
let windowFrames = 0;
function frame(now) {
  // the first rAF timestamp can precede the performance.now() taken at start: never step backwards
  fx.update(Math.max(0, now - last) / 1000);
  last = now;
  fx.draw("both");
  ctx.putImageData(new ImageData(fx.rgba(), fx.width(), fx.height()), 0, 0);
  stats.frames += 1;
  windowFrames += 1;
  if (now - windowStart >= 1000) {
    stats.fps = Math.round((windowFrames * 1000) / (now - windowStart));
    windowStart = now;
    windowFrames = 0;
  }
  status.textContent = `${info.name} · ${fx.width()}×${fx.height()} · t ${fx.t().toFixed(1)} s · ${stats.fps} fps`;
  requestAnimationFrame(frame);
}
requestAnimationFrame(frame);
