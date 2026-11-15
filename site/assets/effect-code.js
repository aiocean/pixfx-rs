// The code that plays one effect, with the options set on its page, for each way a game uses it: a JS shell
// (Canvas 2D, Phaser 4, Pixi 8), gpui in Rust, or files made by the export scripts. An effect that wraps the sprite
// it plays on (`meta.wrapsActor`) is handed one in every call that can hand it one.

/** The targets, in the order their tabs show. */
export const TARGETS = [
  { key: "canvas", label: "Canvas 2D", docs: "../docs/canvas.html" },
  { key: "phaser", label: "Phaser 4", docs: "../docs/phaser.html" },
  { key: "pixi", label: "Pixi 8", docs: "../docs/pixi.html" },
  { key: "gpui", label: "gpui", docs: "../docs/gpui.html" },
  { key: "files", label: "Files", docs: "../docs/scripts.html" },
];

/**
 * The code for target `key` that plays effect `id` (its `meta` `m`) with options `o`: only what differs from the
 * defaults, as the page's controls give them (`seed`, `light`, `palette` as a preset name, the effect's own numbers).
 */
export function codeFor(key, id, m, o) {
  return CODE[key](id, m, o);
}

const CODE = {
  canvas(id, m, o) {
    const play = `pixfx.play("${id}", x, y${jsOptions(o)})`;
    if (!m.wrapsActor) {
      return `import { createPixfx, loadPixfx } from "pixfx-canvas2d";

await loadPixfx();
const pixfx = createPixfx(ctx, { scale: 4 });
${play};
// each frame: pixfx.update(dt), then pixfx.draw()`;
    }
    return `import { createPixfx, imageActor, loadPixfx } from "pixfx-canvas2d";

await loadPixfx();
const pixfx = createPixfx(ctx, { scale: 4 });
// it plays on the sprite you draw: the part of your sheet shown now, and where
${play}.wrap(imageActor(() => ({
  image: sheet,
  cut: hero.frame, // { x, y, w, h } on the sheet
  x: hero.left,
  y: hero.top,
  flipX: hero.facingLeft,
})));
// each frame: pixfx.update(dt), then pixfx.draw()`;
  },

  phaser(id, m, o) {
    const play = `this.pixfx.play("${id}", ${m.wrapsActor ? "hero.x, hero.y" : "x, y"}${jsOptions(o)})`;
    const head = `import { PixfxPlugin, loadPixfx } from "pixfx-phaser";

await loadPixfx();
// plugins: { scene: [{ key: "pixfx", plugin: PixfxPlugin, mapping: "pixfx" }] }`;
    if (!m.wrapsActor) return `${head}\n${play};`;
    return `${head}
// it grips the sprite's outline and follows every frame it shows
${play}
  .follow(hero)
  .wrap(this.pixfx.actorOf(hero));`;
  },

  pixi(id, m, o) {
    const play = `pixfx.play("${id}", ${m.wrapsActor ? "hero.x, hero.y" : "x, y"}${jsOptions(o)})`;
    const head = `import { createPixfx, loadPixfx } from "pixfx-pixi";

await loadPixfx();
const pixfx = createPixfx(app, { container: world });`;
    if (!m.wrapsActor) return `${head}\n${play};`;
    return `${head}
// it grips the sprite's outline and follows every frame it shows
${play}
  .follow(hero)
  .wrap(pixfx.actorOf(hero));`;
  },

  gpui(id, m, o) {
    const opts = rustOptions(o);
    const uses = ["Layer", "Options", ...(m.wrapsActor ? ["Actor"] : []), ...(opts.extra ? ["OptValue"] : [])].sort();
    const lines = [
      `use pixfx_core::{${uses.join(", ")}};`,
      ...(o.palette ? ["use pixfx_draw::palette::preset;"] : []),
      "use pixfx_gpui::FxScene;",
      "",
      `let effect = pixfx_effects::effect("${id}").ok_or("missing")?;`,
      ...opts.lines,
      "let mut scene = FxScene::default();",
      `let fx = scene.play(effect, ${opts.value})?;`,
      "",
      "// each frame",
    ];
    if (m.wrapsActor) {
      lines.push(
        "// the sprite it plays on, as the view shows it now: straight-alpha RGBA, its top-left in effect pixels",
        "// (the effect's origin at 0, 0); the same sprite at the same place changes nothing",
        "let hero = Actor::from_rgba(left, top, w, h, &rgba)?;",
        "scene.set_actor(fx, Some(hero))?;",
      );
    }
    lines.push(
      "scene.tick(dt_seconds)?;",
      "",
      "// inside a canvas paint callback, the effect's origin at `origin`",
      "scene.paint(fx, Layer::Both, origin, 4, window)?;",
      "scene.end_frame(window)?;",
    );
    return lines.join("\n");
  },

  files(id, m, o) {
    const flags = [o.seed !== undefined ? `--seed ${o.seed}` : "", o.palette ? `--palette ${o.palette}` : ""].filter(Boolean).join(" ");
    const args = flags ? `${id} ${flags}` : id;
    const lines = [
      "# sprite sheets (composite, colour and light PNGs) and their Aseprite-style JSON",
      `cargo run -p pixfx-export --example sprite_sheet -- ${args} --out out`,
      "# a looping gif, each pixel 4 x 4",
      `cargo run -p pixfx-export --example gif -- ${args} --scale 4 --out out`,
      "# every frame baked, for a game to play without painting (loadBaked)",
      `cargo run -p pixfx-export --example bake -- ${args} --out out`,
    ];
    const unflagged = Object.keys(o).filter((k) => k !== "seed" && k !== "palette");
    if (unflagged.length > 0) {
      lines.push(`# the scripts take no flag for ${unflagged.join(", ")}: set ${unflagged.length > 1 ? "them" : "it"} on the Options in the script, or copy it`);
    }
    if (m.wrapsActor) {
      lines.push(
        "# it wraps a sprite: these draw it on none. For your sprite, render with",
        "# pixfx_core::render_frames_with_actor in a script of your own (copy one of these)",
      );
    }
    return lines.join("\n");
  },
};

/** The options as the last argument of a JS `play()`, or nothing when every option is its default. */
function jsOptions(o) {
  const keys = Object.keys(o);
  if (keys.length === 0) return "";
  return `, {\n${keys.map((k) => `  ${k}: ${JSON.stringify(o[k])},`).join("\n")}\n}`;
}

/**
 * The options as Rust: `value` to pass to `play`, and the `lines` that build it first when it takes more than a
 * literal (the effect's own numbers go in `extra`).
 */
function rustOptions(o) {
  const fields = [];
  if (o.seed !== undefined) fields.push(`seed: Some(${o.seed})`);
  if (o.light !== undefined) fields.push(`light: ${rustNumber(o.light)}`);
  if (o.palette) fields.push(`palette: preset("${o.palette}").map(<[_]>::to_vec)`);
  const own = Object.keys(o).filter((k) => !["seed", "light", "palette"].includes(k));
  if (fields.length === 0 && own.length === 0) return { value: "Options::default()", lines: [], extra: false };
  const literal = fields.length === 0
    ? "Options::default()"
    : `Options {\n${fields.map((f) => `    ${f},`).join("\n")}\n    ..Options::default()\n}`;
  if (own.length === 0) return { value: literal, lines: [], extra: false };
  return {
    value: "opts",
    lines: [
      `let mut opts = ${literal};`,
      ...own.map((k) => `opts.extra.insert("${k}".into(), OptValue::Num(${rustNumber(o[k])}));`),
    ],
    extra: true,
  };
}

/** A JS number as a Rust `f64` literal. */
function rustNumber(n) {
  return Number.isInteger(n) ? `${n}.0` : String(n);
}
