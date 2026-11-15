// A campfire and braziers on dark ground, a candle following a walker drawn between the effects' back and front
// layers, a hero powering up (the aura grips each pose of its sprite), and an explosion where you click. `window.pixfxCanvasDemo` exposes the loop and pixfx for the browser check.
import { createPixfx, imageActor, loadPixfx } from "../src";
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

// the hero: two poses of a 12 x 22 sprite, arms down then raised, drawn into one sheet
const HERO = { x: 236, w: 12, h: 22 };
const sheet = document.createElement("canvas");
sheet.width = HERO.w * 2;
sheet.height = HERO.h;
const sctx = sheet.getContext("2d");
if (!sctx) throw new Error("demo: no 2d context for the hero");
for (const [pose, raised] of [[0, false], [1, true]] as const) {
  const o = pose * HERO.w;
  const px = (x: number, y: number, w: number, h: number, c: string): void => {
    sctx.fillStyle = c;
    sctx.fillRect(o + x, y, w, h);
  };
  px(4, 0, 4, 4, "#f2c79b"); // head
  px(3, 4, 6, 8, "#d94f5c"); // body
  px(3, 12, 2, 10, "#3b4a8c"); // legs
  px(7, 12, 2, 10, "#3b4a8c");
  if (raised) {
    px(1, 0, 2, 6, "#f2c79b");
    px(9, 0, 2, 6, "#f2c79b");
  } else {
    px(1, 5, 2, 7, "#f2c79b");
    px(9, 5, 2, 7, "#f2c79b");
  }
}
let heroPose = 0;
const hero = (): { image: HTMLCanvasElement; cut: { x: number; y: number; w: number; h: number }; x: number; y: number } => ({
  image: sheet,
  cut: { x: heroPose * HERO.w, y: 0, w: HERO.w, h: HERO.h },
  x: HERO.x - HERO.w / 2,
  y: GROUND - HERO.h,
});
const powerUp = (): void => {
  pixfx.play("feedback/power-up", HERO.x, GROUND, { seed: 5 }).wrap(imageActor(hero)).on("complete", powerUp);
};
powerUp();

const demo = { pixfx, events, running: true, frames: 0 };
window.pixfxCanvasDemo = demo;

let last = performance.now();
let t = 0;
const frame = (now: number): void => {
  const dt = demo.running ? (now - last) / 1000 : 0;
  last = now;
  t += dt;
  walker.x = 114 + Math.sin(t / 0.9) * 22;
  heroPose = Math.floor(t / 0.4) % 2;
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
  const h = hero();
  ctx.imageSmoothingEnabled = false;
  ctx.drawImage(h.image, h.cut.x, h.cut.y, h.cut.w, h.cut.h, h.x * SCALE, h.y * SCALE, h.cut.w * SCALE, h.cut.h * SCALE);
  pixfx.drawLayer("front");
  demo.frames += 1;
  requestAnimationFrame(frame);
};
requestAnimationFrame(frame);
events.push("ready");
