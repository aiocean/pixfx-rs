// Preview files are captured from the same show as the live demos.
import { effects, loadPixfx, meta } from "../pkg/pixfx-canvas2d.js";

export async function mount() {
  await loadPixfx();
  const grid = document.getElementById("effects-grid");
  const ids = effects();
  document.getElementById("effects-count").textContent = String(ids.length);
  grid.innerHTML = ids.map((id) => {
    const { name, renderKind } = meta(id);
    const file = `../assets/previews/${id.replaceAll("/", "_")}`;
    return `<li>
      <a class="card" href="effect.html?id=${id}">
        <picture>
          <source media="(prefers-reduced-motion: reduce)" srcset="${file}.png">
          <source type="image/webp" srcset="${file}.webp">
          <img src="${file}.gif" alt="" loading="lazy" decoding="async" width="496" height="288">
        </picture>
        <span class="card-name">${name}</span>
        <code class="card-id">${id} · ${renderKind}</code>
      </a>
    </li>`;
  }).join("");
}
