// The effects page (effects/): a card for every effect, each a still of the effect at its brightest moment,
// linking to the effect's own page where it plays live. A still is drawn once, when its card first comes near
// the screen, so the page runs no animation and costs little however many effects there are.
import { Fx, effects, loadPixfx, meta } from "../pkg/pixfx-canvas2d.js";

/** A card's picture, in game pixels; the effect is centred in it, scaled up by whole numbers when it is small. */
const CARD = { w: 240, h: 180 };
/** The card's sky and floor. */
const COLORS = { sky: "#13101b", ground: "#1e1a24" };
/** A still is the brightest of this many moments, spread over the effect's run (a loop's first two seconds). */
const MOMENTS = 8;

/** Mount the effects page now in the document. Returns how to unmount it. */
export async function mount() {
  await loadPixfx();
  const grid = document.getElementById("effects-grid");
  const ids = effects();
  document.getElementById("effects-count").textContent = String(ids.length);
  grid.innerHTML = ids
    .map((id) => {
      const m = meta(id);
      return `<li><a class="card" href="effect.html?id=${id}">
        <canvas width="${CARD.w}" height="${CARD.h}" data-id="${id}" aria-hidden="true"></canvas>
        <span class="card-name">${m.name}</span>
        <code class="card-id">${id}</code>
      </a></li>`;
    })
    .join("");

  // draw each still once, as its card nears the screen
  const near = new IntersectionObserver(
    (entries) => {
      for (const e of entries) {
        if (!e.isIntersecting) continue;
        near.unobserve(e.target);
        drawStill(e.target, e.target.dataset.id);
      }
    },
    { rootMargin: "300px" },
  );
  for (const c of grid.querySelectorAll("canvas")) near.observe(c);
  return () => near.disconnect();
}

/** The effect `id` at its brightest moment, centred on `canvas` over the card's sky and floor. */
function drawStill(canvas, id) {
  const m = meta(id);
  const fx = new Fx(id, undefined);
  const w = fx.width();
  const h = fx.height();
  const origin = [fx.originX(), fx.originY()];
  const span = m.loop ? Math.min(2, m.duration) : m.duration;
  let best = null;
  let bestLight = -1;
  for (let k = 1; k <= MOMENTS; k++) {
    // the engine takes at most a quarter second a step
    for (let left = span / (MOMENTS + 1); left > 0; left -= 0.25) fx.update(Math.min(0.25, left));
    fx.draw("both");
    const px = fx.rgba();
    const light = brightness(px);
    if (light > bestLight) {
      bestLight = light;
      best = px;
    }
  }
  fx.free();
  const frame = Object.assign(document.createElement("canvas"), { width: w, height: h });
  frame.getContext("2d").putImageData(new ImageData(best, w, h), 0, 0);

  // whole-number scale when it fits, shrunk to fit when it does not
  const fit = Math.min(CARD.w / w, CARD.h / h);
  const s = fit >= 1 ? Math.min(3, Math.floor(fit)) : fit;
  const x = Math.round((CARD.w - w * s) / 2);
  const y = Math.round((CARD.h - h * s) / 2);
  const ctx = canvas.getContext("2d");
  const floor = Math.min(CARD.h, Math.round(y + origin[1] * s));
  ctx.fillStyle = COLORS.sky;
  ctx.fillRect(0, 0, CARD.w, floor);
  ctx.fillStyle = COLORS.ground;
  ctx.fillRect(0, floor, CARD.w, CARD.h - floor);
  ctx.imageSmoothingEnabled = false;
  ctx.drawImage(frame, x, y, w * s, h * s);
}

/** How much an RGBA frame shows: each pixel's brightest channel, weighted by how opaque it is. */
function brightness(px) {
  let sum = 0;
  for (let i = 0; i < px.length; i += 4) sum += Math.max(px[i], px[i + 1], px[i + 2]) * px[i + 3];
  return sum;
}
