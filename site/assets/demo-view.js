import { createPixfx, meta } from "../pkg/pixfx-canvas2d.js";
import { SMOULDER } from "./effect-demos.js";
import { paintStage } from "./show-stage.js";
import { createShow } from "./sprite-show.js";

export function createDemo(canvas, id, { sheets, told, still, scenario, pixelDensity = 2 }) {
  if (id.startsWith("transitions/")) return createTransition(canvas, id, pixelDensity);
  return createShow(canvas, {
    sheets, ids: [id], told, still, scenario: scenario ?? "", replay: false, pixelDensity,
  });
}

function createTransition(canvas, id, pixelDensity) {
  const { size: [w, h], origin: [x, y] } = meta(id);
  const scale = 2;
  canvas.width = w * scale;
  canvas.height = h * scale;
  const ctx = canvas.getContext("2d");
  const backgrounds = ["night", "dawn"].map((tone) => {
    const image = new OffscreenCanvas(canvas.width, canvas.height);
    paintStage(image.getContext("2d"), w, h, scale, tone);
    return image;
  });
  const stage = createPixfx(ctx, { scale, pixelDensity });
  let effect;
  let elapsed = 0;
  let ended = false;

  function draw(withEffects = true) {
    ctx.drawImage(backgrounds[elapsed >= SMOULDER ? 1 : 0], 0, 0);
    if (withEffects) stage.draw();
  }

  return {
    restart(opts) {
      effect?.destroy();
      elapsed = 0;
      ended = false;
      effect = stage.play(id, x, y, opts).on("complete", () => { ended = true; });
    },
    draw,
    finished: () => ended,
    step(dt) {
      elapsed += dt;
      stage.update(dt);
      draw();
    },
    free: () => stage.destroy(),
  };
}
