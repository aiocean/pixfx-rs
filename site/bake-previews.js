import { Canvas, ImageData, loadImage } from "@napi-rs/canvas";
import { mkdtemp, mkdir, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { basename, join, resolve } from "node:path";
import { pathToFileURL } from "node:url";

const FPS = 24;

async function bake(dist) {
  Object.assign(globalThis, { OffscreenCanvas: Canvas, ImageData });
  const fromDist = (file) => import(pathToFileURL(join(dist, file)).href);
  const [{ loadPixfx, effects, meta }, { createDemo }, { DEMOS }] = await Promise.all([
    fromDist("pkg/pixfx-canvas2d.js"),
    fromDist("assets/demo-view.js"),
    fromDist("assets/effect-demos.js"),
  ]);
  await loadPixfx({ workers: 0 });
  const sheets = {};
  for (const look of ["thunder", "frost"]) {
    sheets[look] = await loadImage(join(dist, "assets/sprites/dense", `${look}-poses.png`));
  }
  const output = join(dist, "assets/previews");
  await mkdir(output, { recursive: true });
  const work = await mkdtemp(join(tmpdir(), "pixfx-previews-"));
  try {
    for (const id of effects()) {
      const canvas = new Canvas(1, 1);
      const view = createDemo(canvas, id, { sheets, still: true, scenario: DEMOS[id]?.[0]?.[2] });
      try {
        const name = id.replaceAll("/", "_");
        const dissolve = /^(appear|disappear)\/(dither|fade)$/.test(id);
        const keyframe = dissolve ? Math.round(meta(id).duration * FPS / 2) : undefined;
        const frames = await capture(canvas, view, work, join(output, `${name}.png`), keyframe);
        const webpFrames = frames.flatMap((file, i) => [
          "-d", String(Math.round((i + 1) * 1000 / FPS) - Math.round(i * 1000 / FPS)), file,
        ]);
        const results = await Promise.allSettled([
          run(["img2webp", "-m", "2", ...webpFrames, "-o", join(output, `${name}.webp`)], work),
          run(["ffmpeg", "-hide_banner", "-loglevel", "error", "-y", "-framerate", String(FPS),
            "-i", "frame-%03d.png", "-frames:v", String(frames.length), "-filter_complex_threads", "1",
            "-filter_complex", "split[a][b];[a]palettegen=stats_mode=diff[p];[b][p]paletteuse=dither=none",
            "-loop", "0", join(output, `${name}.gif`)], work),
        ]);
        for (const result of results) {
          if (result.status === "rejected") throw result.reason;
        }
        await Promise.all(frames.map((file) => rm(join(work, file))));
        console.log(`preview: ${id} (${frames.length} frames)`);
      } finally {
        view.free();
      }
    }
  } finally {
    await rm(work, { recursive: true, force: true });
  }
}

async function capture(canvas, view, work, poster, keyframe) {
  const ctx = canvas.getContext("2d");
  const { width, height } = canvas;
  view.restart();
  const frames = [];
  let best = -Infinity;
  let still;
  for (let i = 0; i <= 10 * FPS && !view.finished(); i++) {
    view.step(i === 0 ? 0 : 1 / FPS);
    const png = canvas.toBuffer("image/png");
    const pixels = ctx.getImageData(0, 0, width, height).data;
    view.draw(false);
    const baseline = ctx.getImageData(0, 0, width, height).data;
    const score = keyframe === undefined ? changedPixels(baseline, pixels) : -Math.abs(i - keyframe);
    if (score > best) {
      still = png;
      best = score;
    }
    const file = `frame-${String(i).padStart(3, "0")}.png`;
    await Bun.write(join(work, file), png);
    frames.push(basename(file));
  }
  if (keyframe === undefined && best <= 0) throw new Error(`No visible effect in ${poster}`);
  await Bun.write(poster, still);
  return frames;
}

function changedPixels(before, after) {
  let count = 0;
  for (let i = 0; i < before.length; i += 4) {
    const delta = Math.abs(before[i] - after[i]) + Math.abs(before[i + 1] - after[i + 1]) + Math.abs(before[i + 2] - after[i + 2]);
    if (delta > 32) count++;
  }
  return count;
}

async function run(command, cwd) {
  const child = Bun.spawn(command, { cwd, stdout: "ignore", stderr: "pipe" });
  const error = await new Response(child.stderr).text();
  if (await child.exited !== 0) throw new Error(`${command[0]}: ${error.trim()}`);
}

const dist = process.argv[2];
if (!dist) throw new Error("usage: bun site/bake-previews.js <dist>");
await bake(resolve(dist));
