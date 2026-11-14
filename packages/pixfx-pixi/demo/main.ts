// A campfire and braziers on dark ground, a candle following a walker, and an explosion with shake where you click.
// `window.pixfxPixiDemo` exposes the app, pixfx and the events for the browser check.
import { Application, Container, Graphics } from "pixi.js";
import { createPixfx, loadPixfx } from "../src";
import type { Pixfx, WorldCue } from "../src";

const SCALE = 4;
const GAME = { w: 336, h: 180 };
const GROUND = 160;

declare global {
  interface Window {
    pixfxPixiDemo?: { app: Application; pixfx: Pixfx; world: Container; events: string[] };
  }
}

const params = new URLSearchParams(location.search);
const workers = Number(params.get("workers") ?? Number.NaN);
await loadPixfx(Number.isInteger(workers) ? { workers } : {});

const app = new Application();
await app.init({ width: GAME.w * SCALE, height: GAME.h * SCALE, background: "#17131c", antialias: false, roundPixels: true });
document.body.appendChild(app.canvas);

const world = app.stage.addChild(new Container());
world.scale.set(SCALE);
world.addChild(new Graphics().rect(0, GROUND, GAME.w, GAME.h - GROUND).fill(0x2a2233));

const fx = new Container();
const pixfx = createPixfx(app, { container: fx, camera: world });
world.addChild(fx);

const events: string[] = [];
pixfx.play("fire/campfire", 168, GROUND, { seed: 1 });
pixfx.play("fire/brazier", 60, GROUND, { seed: 2, depth: { back: 0, front: 2 } });
pixfx.play("fire/brazier", 280, GROUND, { seed: 3 });

// a walker the candle follows, between the back and front sprites
const walker = { x: 114, y: GROUND - 18, active: true };
const figure = new Graphics().rect(-4, 0, 8, 18).fill(0x4fd1c5);
figure.zIndex = 1;
fx.addChild(figure);
const candle = pixfx.play("fire/candle", 0, 0, { seed: 4 }).follow(walker, 0, 0);
candle.on("destroy", () => events.push("candle destroy"));
app.ticker.add(() => {
  walker.x = 114 + Math.sin(performance.now() / 900) * 22;
  figure.position.set(walker.x, walker.y);
});

pixfx.feel.on("cue", (c: WorldCue) => events.push(`cue ${c.cue.type}`));
app.canvas.addEventListener("click", (e) => {
  const r = app.canvas.getBoundingClientRect();
  const x = ((e.clientX - r.left) / r.width) * GAME.w;
  const y = ((e.clientY - r.top) / r.height) * GAME.h;
  pixfx.play("explosions/small-explosion", x, y).on("complete", () => events.push("explosion complete"));
});

window.pixfxPixiDemo = { app, pixfx, world, events };
events.push("ready");
