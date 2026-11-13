// Stress page: many effects at once, measured per frame. Open `stress.html?n=100&mix=mixed&mode=objects`, or
// `stress.html?sweep=10,25,50,100` to step through counts and print a table (also on `window.pixfxStress`).
// `workers=N` sets the effect workers (0: draw on the main thread; default: the package default) and
// `slowdown=K` stretches each worker batch K times (phone emulation: Chrome cannot CPU-throttle workers).
// `seeds=shared` gives every effect of one id the same seed, so identical effects share frames; the default gives
// each its own.
// Frame cost is main-thread time from Phaser's PRE_STEP to POST_RENDER (update, inline effect drawing, layer
// uploads and the render submission; uploads of frames arriving from workers happen between steps and are not
// in it); fps is game steps per second, which also drops when the GPU falls behind.
import Phaser from "phaser";
import type { Counts, PixfxEffect, PixfxLayer } from "../src";
import { PixfxPlugin, loadPixfx } from "../src";
import { defaultWorkers } from "../src/load";
import type { Seeds } from "./seeds";
import { seedsParam, stressSeed } from "./seeds";

const WIDTH = 640;
const HEIGHT = 360;
const IDS = ["fire/candle", "fire/campfire", "fire/brazier"] as const;

type Mix = "candle" | "campfire" | "brazier" | "mixed";
type Mode = "objects" | "layer";

interface Config {
  n: number;
  mix: Mix;
  mode: Mode;
  /** Share of effects (0..1) that follow a moving sprite. */
  follow: number;
  /** Effects stopped and replaced per second. */
  churn: number;
  /** One seed per effect, or one per id ([`stressSeed`]). */
  seeds: Seeds;
}

interface Result extends Config {
  workers: number;
  slowdown: number;
  fps: number;
  cpuMedian: number;
  cpuP95: number;
  frames: number;
  stats: Counts;
}

interface StressHandle {
  config: Config;
  results: Result[];
  done: boolean;
  game: Phaser.Game;
  run: (cfg: Config) => Promise<Result>;
}

declare global {
  interface Window {
    pixfxStress?: StressHandle;
  }
}

const params = new URLSearchParams(location.search);

function num(key: string, fallback: number): number {
  const v = Number(params.get(key) ?? fallback);
  return Number.isFinite(v) ? v : fallback;
}

function mixParam(): Mix {
  const v = params.get("mix");
  return v === "candle" || v === "campfire" || v === "brazier" ? v : "mixed";
}

const base: Config = {
  n: num("n", 50),
  mix: mixParam(),
  mode: params.get("mode") === "layer" ? "layer" : "objects",
  follow: Math.min(1, Math.max(0, num("follow", 0))),
  churn: Math.max(0, num("churn", 0)),
  seeds: seedsParam(params.get("seeds")),
};
const WORKERS = params.has("workers") ? Math.max(0, Math.floor(num("workers", 0))) : defaultWorkers(navigator.hardwareConcurrency);
const SLOWDOWN = Math.max(1, num("slowdown", 1));
const WARM_MS = num("warm", 1000);
const MEASURE_MS = num("measure", 3000);

/** mulberry32: the same positions and seeds on every run. */
function rng(seed: number): () => number {
  let a = seed >>> 0;
  return () => {
    a = (a + 0x6d2b79f5) >>> 0;
    let t = a;
    t = Math.imul(t ^ (t >>> 15), t | 1);
    t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

class StressScene extends Phaser.Scene {
  declare pixfx: PixfxPlugin;
  cfg: Config = base;
  private rand = rng(1);
  private live: PixfxEffect[] = [];
  private layer: PixfxLayer | null = null;
  private churnDebt = 0;
  private spawned = 0;

  constructor() {
    super("stress");
  }

  create(): void {
    this.rand = rng(1);
    this.live = [];
    this.churnDebt = 0;
    this.spawned = 0;
    this.add.rectangle(0, 0, WIDTH, HEIGHT, 0x1a1a24).setOrigin(0, 0).setDepth(-10);
    this.layer = this.cfg.mode === "layer" ? this.pixfx.createLayer({ bounds: { x: 0, y: 0, width: WIDTH, height: HEIGHT } }) : null;
    const followers = Math.round(this.cfg.n * this.cfg.follow);
    for (let i = 0; i < this.cfg.n; i++) {
      const effect = this.spawn();
      if (i < followers) {
        const sprite = this.add.rectangle(effect.x, effect.y, 4, 4, 0x40c0ff).setDepth(2);
        this.tweens.add({ targets: sprite, x: sprite.x + 40 + this.rand() * 60, duration: 1500 + this.rand() * 1500, yoyo: true, repeat: -1 });
        effect.fx.follow(sprite, 0, 0);
      }
    }
  }

  override update(_time: number, delta: number): void {
    if (this.cfg.churn <= 0) return;
    this.churnDebt += (this.cfg.churn * delta) / 1000;
    while (this.churnDebt >= 1 && this.live.length > 0) {
      this.churnDebt -= 1;
      const victim = this.live[Math.floor(this.rand() * this.live.length)];
      victim?.stop();
      this.spawn();
    }
  }

  private pickIndex(): number {
    const i = this.cfg.mix === "mixed" ? this.spawned % IDS.length : IDS.findIndex((id) => id === `fire/${this.cfg.mix}`);
    return i < 0 ? 0 : i;
  }

  private spawn(): { fx: PixfxEffect; x: number; y: number } {
    const x = Math.round(30 + this.rand() * (WIDTH - 60));
    const y = Math.round(90 + this.rand() * (HEIGHT - 100));
    const i = this.pickIndex();
    const id = IDS[i] ?? IDS[0];
    const opts = { seed: stressSeed(this.cfg.seeds, this.spawned, i) };
    this.spawned += 1;
    const fx = this.layer ? this.layer.play(id, x, y, opts) : this.pixfx.play(id, x, y, { ...opts, depth: { back: y, front: y + 0.5 } });
    this.live.push(fx);
    const forget = (): void => {
      this.live = this.live.filter((e) => e !== fx);
    };
    fx.on("complete", forget);
    fx.on("destroy", forget);
    return { fx, x, y };
  }
}

function sleep(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

function percentile(sorted: number[], p: number): number {
  if (sorted.length === 0) return 0;
  return sorted[Math.min(sorted.length - 1, Math.floor(p * sorted.length))] ?? 0;
}

const round2 = (v: number): number => Math.round(v * 100) / 100;

function rendererType(): number {
  return params.get("renderer") === "canvas" ? Phaser.CANVAS : Phaser.WEBGL;
}

function row(r: Result): string {
  return [r.mode, r.mix, r.n, r.follow, r.churn, r.seeds, r.workers, r.slowdown, r.fps, r.cpuMedian, r.cpuP95, r.stats.textures].map(String).join("\t");
}

async function main(): Promise<void> {
  await loadPixfx({ workers: WORKERS, slowdown: SLOWDOWN });
  const scene = new StressScene();
  const game = new Phaser.Game({
    type: rendererType(),
    width: WIDTH,
    height: HEIGHT,
    parent: "game",
    zoom: 2,
    pixelArt: true,
    backgroundColor: "#000000",
    scene,
    fps: { smoothStep: false },
    plugins: { scene: [{ key: "pixfx", plugin: PixfxPlugin, mapping: "pixfx" }] },
  });
  await new Promise<void>((resolve) => game.events.once(Phaser.Core.Events.READY, () => resolve()));

  let stepStart = 0;
  let recording = false;
  let samples: number[] = [];
  game.events.on(Phaser.Core.Events.PRE_STEP, () => {
    stepStart = performance.now();
  });
  game.events.on(Phaser.Core.Events.POST_RENDER, () => {
    if (recording) samples.push(performance.now() - stepStart);
  });

  const hud = document.getElementById("hud");
  const results: Result[] = [];
  const header = "mode\tmix\tn\tfollow\tchurn\tseeds\tworkers\tslowdown\tfps\tcpu-med-ms\tcpu-p95-ms\ttextures";

  async function run(cfg: Config): Promise<Result> {
    game.loop.wake();
    scene.cfg = cfg;
    scene.scene.restart();
    await sleep(WARM_MS);
    samples = [];
    recording = true;
    const t0 = performance.now();
    await sleep(MEASURE_MS);
    recording = false;
    const elapsed = performance.now() - t0;
    const sorted = [...samples].sort((a, b) => a - b);
    const r: Result = {
      ...cfg,
      workers: WORKERS,
      slowdown: SLOWDOWN,
      fps: round2((samples.length * 1000) / elapsed),
      cpuMedian: round2(percentile(sorted, 0.5)),
      cpuP95: round2(percentile(sorted, 0.95)),
      frames: samples.length,
      stats: scene.pixfx.stats(),
    };
    results.push(r);
    if (hud) hud.textContent = [header, ...results.map(row)].join("\n");
    return r;
  }

  const handle: StressHandle = { config: base, results, done: false, game, run };
  window.pixfxStress = handle;

  const sweep = params.get("sweep");
  const counts = sweep ? sweep.split(",").map(Number).filter((v) => Number.isInteger(v) && v > 0) : [base.n];
  for (const n of counts) await run({ ...base, n });
  handle.done = true;
  // A finished sweep stops drawing, so its heaviest scene does not hold up the page the harness opens next.
  if (sweep) game.loop.sleep();
}

main().catch((err: unknown) => {
  const hud = document.getElementById("hud");
  if (hud) hud.textContent = `stress page failed: ${err instanceof Error ? err.message : String(err)} (run \`bun run build:wasm\` first)`;
});
