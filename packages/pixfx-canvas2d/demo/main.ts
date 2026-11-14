// A campfire and braziers on dark ground, a candle following a walker drawn between the effects' back and front
// layers, and an explosion where you click. `window.pixfxCanvasDemo` exposes the loop and pixfx for the browser check.
import { createPixfx, loadPixfx } from "../src";
import type { Pixfx, WorldCue } from "../src";

const SCALE = 4;
const GAME = { w: 336, h: 180 };
const GROUND = 160;

declare global {
  interface Window {
    pixfxCanvasDemo?: { pixfx: Pixfx; events: string[]; running: boolean; frames: number };
  }
}

const params = new URLSearchParams(location.search);
const workers = Number(params.get("workers") ?? Number.NaN);
await loadPixfx(Number.isInteger(workers) ? { workers } : {});

const canvas = document.getElementById("scene");
if (!(canvas instanceof HTMLCanvasElement)) throw new Error("demo: no #scene canvas");
canvas.width = GAME.w * SCALE;
canvas.height = GAME.h * SCALE;
const ctx = canvas.getContext("2d");
if (!ctx) throw new Error("demo: no 2d context");

const pixfx = createPixfx(ctx, { scale: SCALE });
const events: string[] = [];
pixfx.play("fire/campfire", 168, GROUND, { seed: 1 });
pixfx.play("fire/brazier", 60, GROUND, { seed: 2 });
pixfx.play("fire/brazier", 280, GROUND, { seed: 3 });
const walker = { x: 114, y: GROUND - 18, active: true };
pixfx.play("fire/candle", 0, 0, { seed: 4 }).follow(walker, 0, 0).on("destroy", () => events.push("candle destroy"));
pixfx.feel.on("cue", (c: WorldCue) => events.push(`cue ${c.cue.type}`));

canvas.addEventListener("click", (e) => {
  const r = canvas.getBoundingClientRect();
  const x = ((e.clientX - r.left) / r.width) * GAME.w;
  const y = ((e.clientY - r.top) / r.height) * GAME.h;
  pixfx.play("explosions/small-explosion", x, y).on("complete", () => events.push("explosion complete"));
});

const demo = { pixfx, events, running: true, frames: 0 };
window.pixfxCanvasDemo = demo;

let last = performance.now();
let t = 0;
const frame = (now: number): void => {
  const dt = demo.running ? (now - last) / 1000 : 0;
  last = now;
  t += dt;
  walker.x = 114 + Math.sin(t / 0.9) * 22;
  pixfx.update(dt);
  ctx.setTransform(1, 0, 0, 1, 0, 0);
  ctx.fillStyle = "#17131c";
  ctx.fillRect(0, 0, canvas.width, canvas.height);
  const [sx, sy] = pixfx.feel.shake;
  ctx.translate(sx * SCALE, sy * SCALE);
  ctx.fillStyle = "#2a2233";
  ctx.fillRect(0, GROUND * SCALE, GAME.w * SCALE, (GAME.h - GROUND) * SCALE);
  pixfx.drawLayer("back");
  ctx.fillStyle = "#4fd1c5";
  ctx.fillRect((Math.round(walker.x) - 4) * SCALE, walker.y * SCALE, 8 * SCALE, 18 * SCALE);
  pixfx.drawLayer("front");
  demo.frames += 1;
  requestAnimationFrame(frame);
};
requestAnimationFrame(frame);
events.push("ready");
