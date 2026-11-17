// A walker on dark ground ringed by an outline and smeared by a trail along the way it walks, a hero powering up (the
// aura grips its sprite), and a body bursting with shake where you click.
// `window.pixfxPixiDemo` exposes the app, pixfx and the events for the browser check.
import { Application, Container, Graphics, Sprite, Texture } from "pixi.js";
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

/** A sprite of one `w` x `h` block of `color`, standing on its bottom middle. */
function block(w: number, h: number, color: string): Sprite {
  const canvas = document.createElement("canvas");
  canvas.width = w;
  canvas.height = h;
  const c = canvas.getContext("2d");
  if (!c) throw new Error("demo: no 2d context");
  c.fillStyle = color;
  c.fillRect(0, 0, w, h);
  const texture = Texture.from(canvas);
  texture.source.scaleMode = "nearest";
  const sprite = new Sprite(texture);
  sprite.anchor.set(0.5, 1);
  return sprite;
}

// the game's own figures, between the effects' back and front sprites
const figures = new Container();
figures.zIndex = 0.5;
fx.addChild(figures);

// a walker the outline rings and the trail smears
const walker = { x: 114, y: GROUND, active: true };
const figure = figures.addChild(block(8, 18, "#4fd1c5"));
pixfx.play("projectiles/trail", 0, 0, { seed: 3 }).follow(walker, 0, -9).wrap(pixfx.actorOf(figure));
const outline = pixfx.play("feedback/outline", 0, 0, { seed: 4 }).follow(walker, 0, -9).wrap(pixfx.actorOf(figure));
outline.on("destroy", () => events.push("outline destroy"));
app.ticker.add(() => {
  walker.x = 114 + Math.sin(performance.now() / 900) * 22;
  figure.position.set(Math.round(walker.x), walker.y);
});

// a hero powering up, again and again
const hero = figures.addChild(block(12, 22, "#d94f5c"));
hero.position.set(236, GROUND);
const powerUp = (): void => {
  pixfx.play("feedback/power-up", 236, GROUND, { seed: 5 }).wrap(pixfx.actorOf(hero)).on("complete", powerUp);
};
powerUp();

pixfx.feel.on("cue", (c: WorldCue) => events.push(`cue ${c.cue.type}`));
app.canvas.addEventListener("click", (e) => {
  const r = app.canvas.getBoundingClientRect();
  const x = ((e.clientX - r.left) / r.width) * GAME.w;
  const y = ((e.clientY - r.top) / r.height) * GAME.h;
  // no character given: it blows apart a stand-in body standing at the click
  pixfx.play("explosions/body-burst", x, y).on("complete", () => events.push("burst complete"));
});

window.pixfxPixiDemo = { app, pixfx, world, events };
events.push("ready");
