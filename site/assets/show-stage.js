export const SHOW_BOX = { w: 248, h: 144 };

const NIGHT = {
  sky: "#121126", horizon: "#201c38", far: "#24213e", near: "#1b1930",
  moon: "#c2b7da", stars: "#665f82", floor: "#242133", rim: "#514961", joints: "#1a1826",
};
const OVERCAST = {
  sky: "#655e73", horizon: "#82768a", far: "#786d82", near: "#6a6277",
  moon: "#d8cdd8", stars: "#a096ae", floor: "#50485e", rim: "#9b8ca3", joints: "#443e50",
};
const DAWN = {
  ...NIGHT, sky: "#142330", horizon: "#233b48", far: "#283d50", near: "#1f3040", rim: "#50717d",
};

export function stageFloor(h) {
  return h - Math.round(h * 0.18);
}

// A moonlit stone platform, based on the Arena palette. Keep the centre clear for effects.
export function paintStage(ctx, w, h, scale, tone = "night", shake = [0, 0]) {
  const c = tone === "outline" ? OVERCAST : tone === "dawn" ? DAWN : NIGHT;
  const floor = stageFloor(h);
  ctx.fillStyle = c.sky;
  ctx.fillRect(0, 0, w * scale, h * scale);
  ctx.save();
  ctx.translate(Math.round(shake[0]) * scale, Math.round(shake[1]) * scale);
  ctx.scale(scale, scale);
  ctx.fillStyle = c.horizon;
  ctx.fillRect(0, Math.round(h * 0.42), w, h);
  ctx.fillStyle = c.stars;
  for (let i = 0; i < 20; i++) {
    ctx.fillRect(Math.floor(((i * 37 + 19) % 101) * w / 101), Math.floor(((i * 17 + 3) % 59) * h / 144), 1, 1);
  }
  ctx.fillStyle = c.moon;
  const moonX = Math.round(w * 0.79);
  const moonY = Math.round(h * 0.23);
  for (let y = -7; y <= 7; y++) {
    const half = Math.floor(Math.sqrt(49 - y * y));
    ctx.fillRect(moonX - half, moonY + y, half * 2 + 1, 1);
  }
  ridge(ctx, w, floor, h, c.far, [43, 27, 49, 23, 37, 25, 45, 31, 52]);
  ridge(ctx, w, floor, h, c.near, [18, 27, 14, 23, 12, 19, 14, 30, 21]);
  ctx.fillStyle = c.floor;
  ctx.fillRect(-8, floor, w + 16, h - floor + 8);
  ctx.fillStyle = c.rim;
  ctx.fillRect(-8, floor, w + 16, 1);
  ctx.fillStyle = c.joints;
  ctx.fillRect(0, floor + 8, w, 1);
  ctx.fillRect(0, floor + 20, w, 1);
  for (let x = 18; x < w; x += 38) {
    ctx.fillRect(x, floor + 1, 1, 7);
    ctx.fillRect(x + 13, floor + 9, 1, 11);
  }
  ctx.restore();
}

function ridge(ctx, w, floor, h, color, heights) {
  ctx.fillStyle = color;
  const last = heights.length - 1;
  for (let x = 0; x < w; x++) {
    const u = x / w * last;
    const i = Math.floor(u);
    const lift = heights[i] + (heights[i + 1] - heights[i]) * (u - i);
    const top = floor - Math.round(lift * h / 144);
    ctx.fillRect(x, top, 1, floor - top);
  }
}
