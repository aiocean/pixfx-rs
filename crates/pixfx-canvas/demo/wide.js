// 100 fires over a world four canvases wide, seen through a camera that pans across it. The page owns the loop
// and draws through its camera; with ?view=on it tells the stage what the camera sees (setView), so fires outside
// it are not drawn, with ?view=off every fire is drawn. window.pixfxWide.cost() gives the median and p95 time of
// update + drawing per frame, in ms, over the last 120 frames.
// Build first: wasm-pack build crates/pixfx-canvas --target web --out-dir demo/pkg
import init, { Stage } from "./pkg/pixfx_canvas.js";

const SCALE = 2;
const GAME = { w: 320, h: 180 }; // game pixels the canvas shows
const WORLD_W = GAME.w * 4;
const FIRES = 100;
const PAN_SPEED = 80; // game pixels a second
const viewOn = new URLSearchParams(location.search).get("view") !== "off";

await init();
const canvas = document.getElementById("stage");
canvas.width = GAME.w * SCALE;
canvas.height = GAME.h * SCALE;
const ctx = canvas.getContext("2d");
const status = document.getElementById("status");
const stage = new Stage(canvas, SCALE);
const ids = ["fire/candle", "fire/campfire", "fire/brazier"];
for (let i = 0; i < FIRES; i += 1) {
  const x = ((i + 0.5) * WORLD_W) / FIRES;
  const y = 60 + ((i * 37) % 100);
  stage.play(ids[i % ids.length], x, y, { seed: 1 + i });
}

const costs = [];
let camX = 0;
let last = null;
function frame(now) {
  const dt = last === null ? 0 : Math.max(0, now - last) / 1000;
  last = now;
  const start = performance.now();
  // the camera pans to the world's right edge and back
  const span = WORLD_W - GAME.w;
  const t = (now / 1000) * PAN_SPEED;
  camX = Math.round(span - Math.abs((t % (2 * span)) - span));
  if (viewOn) stage.setView(camX, 0, GAME.w, GAME.h);
  stage.update(dt);
  ctx.fillStyle = "#17131c";
  ctx.fillRect(0, 0, canvas.width, canvas.height);
  ctx.save();
  ctx.translate(-camX * SCALE, 0);
  stage.drawLayer("back");
  stage.drawLayer("front");
  ctx.restore();
  costs.push(performance.now() - start);
  if (costs.length > 120) costs.shift();
  if (costs.length % 30 === 0) {
    const c = cost();
    status.textContent = `${FIRES} fires · view ${viewOn ? "on" : "off"} · ${c.medianMs.toFixed(1)} ms a frame (median)`;
  }
  requestAnimationFrame(frame);
}
requestAnimationFrame(frame);

function cost() {
  const s = [...costs].sort((a, b) => a - b);
  const at = (q) => s[Math.min(s.length - 1, Math.floor(q * s.length))];
  return { view: viewOn, frames: s.length, medianMs: at(0.5), p95Ms: at(0.95) };
}
window.pixfxWide = { stage, cost };
