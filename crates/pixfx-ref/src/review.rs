//! Review pages: one per effect (every frame as original, the model's pixels, alpha, colour, light and rebuilt,
//! then the frame diffs) and a gallery of every effect's sheet linking to them. The pages are plain HTML over the PNGs the `extract`
//! script writes beside them, named by [`file`].

use serde_json::{Value, json};

use crate::extract::Extracted;
use crate::matrix::{Frame, Matrix, Px};
use crate::tween;

/// The columns of an effect page, left to right: the key each image file is named by, and its heading.
pub const COLUMNS: [(&str, &str); 7] = [
    ("orig", "original (over black)"),
    ("grid", "the model's pixels (busiest part, cuts in cyan)"),
    ("alpha", "alpha (transparency)"),
    ("color", "colour"),
    ("light", "light"),
    ("rebuilt", "rebuilt"),
    ("palette", "through the palette"),
];

/// The file an effect page shows for `cell` (`init-1`) in column `kind` (a [`COLUMNS`] key): `init-1.color.png`.
pub fn file(cell: &str, kind: &str) -> String {
    format!("{cell}.{kind}.png")
}

/// The file of a frame diff (`init-1-2`): `diff-init-1-2.png`.
pub fn diff_file(name: &str) -> String {
    format!("diff-{name}.png")
}

fn esc(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

const STYLE: &str = "body{background:#0f0f17;color:#ddd;font:14px system-ui;margin:20px}table{border-collapse:collapse}\
td,th{padding:4px;border:1px solid #2a2a3a;vertical-align:top}img{display:block;image-rendering:pixelated;width:192px;max-width:18vw}\
img.orig{image-rendering:auto}td.grid img{width:192px}td.color img{background:repeating-conic-gradient(#3c3c3c 0 25%,#282828 0 50%) 0 0/6px 6px}\
th{text-align:left;font-weight:600}small{color:#999;font-weight:400}\
.player{position:sticky;top:0;z-index:2;display:flex;gap:16px;align-items:flex-start;background:#0f0f17f2;padding:10px 0;border-bottom:1px solid #2a2a3a}\
.player canvas{image-rendering:pixelated;background:#000;border:1px solid #2a2a3a}.ctl{display:flex;flex-direction:column;gap:8px}\
.ctl button,.ctl select,.ctl input{background:#1b1b29;color:#ddd;border:1px solid #3a3a50;border-radius:4px;padding:3px 8px;font:inherit}\
.ctl input{width:60px}.sw{display:inline-flex;align-items:center;gap:3px;margin:0 8px 4px 0;font-family:ui-monospace,monospace}\
.sw input{width:22px;height:20px;padding:0;border:1px solid #444;background:none;cursor:pointer}#fr{font-family:ui-monospace,monospace}.d{display:flex;flex-wrap:wrap;gap:12px}figure{margin:0}a{color:#9cf}";

/// The player at the top of an effect page: the frames in order on a canvas, nearest-neighbour, over black; every
/// row in turn or one row looped, a layer to show, fps, play/pause (space) and a frame at a time (← →). It plays the
/// PNGs beside the page (named by [`file`]); `__ROWS__`, `__W__` and `__H__` are filled in by [`effect_page`]. With
/// in-betweens on, the frames made between each two drawn frames of a row ([`tween`], `__N__` of them, in `__MX__`
/// by [`tween::name`]) play between them, every one or every second or fourth, the drawn frames still at the fps.
const PLAYER: &str = r##"<section class="player"><canvas id="pv" width="__W__" height="__H__"></canvas><div class="ctl">
<button id="pp">pause</button>
<label>rows <select id="rw"></select></label>
<label>layer <select id="ly"><option value="rebuilt">rebuilt</option><option value="orig">original</option><option value="color">colour</option><option value="light">light</option><option value="recolour">recoloured (palette below)</option></select></label>
<label>in-betweens <select id="tw"></select></label>
<label>fps <input id="fp" type="number" min="1" max="30" value="6"></label>
<label>size <select id="zm"><option value="2">2x</option><option value="3" selected>3x</option><option value="4">4x</option></select></label>
<span id="fr"></span><small>space: play/pause · ← →: one frame</small></div></section>
<script>
const MX = __MX__;
const ROWS = __ROWS__, KINDS = ["rebuilt", "orig", "color", "light"], img = {};
for (const r of ROWS) for (const c of r.cells) for (const k of KINDS) { const i = new Image(); i.src = `${c}.${k}.png`; i.onload = () => draw(); img[`${c}.${k}`] = i; }
const $ = id => document.getElementById(id), cv = $("pv"), g = cv.getContext("2d");
$("rw").innerHTML = `<option value="-1">all, in order</option>` + ROWS.map((r, n) => `<option value="${n}">${r.row}</option>`).join("");
let at = 0, playing = true, last = 0;
const N = __N__;
$("tw").innerHTML = [0, 1, 3, 7, 15].filter(c => c <= N && (N + 1) % (c + 1) === 0).map(c => `<option${c === N ? " selected" : ""}>${c}</option>`).join("");
// a row's cells with the chosen in-betweens between each two (none across rows)
const between = cells => {
  const c = +$("tw").value, step = (N + 1) / (c + 1), out = [];
  cells.forEach((a, i) => { out.push(a); const b = cells[i + 1]; if (c && b && MX.f[`${a}>${b}@1`]) for (let k = 1; k <= c; k++) out.push(`${a}>${b}@${k * step}`); });
  return out;
};
const seq = () => { const n = +$("rw").value; return n < 0 ? ROWS.flatMap(r => between(r.cells)) : between(ROWS[n].cells); };
function size() { const z = +$("zm").value; cv.style.width = cv.width * z + "px"; cv.style.height = cv.height * z + "px"; }
function draw() {
  const s = seq(); at = ((at % s.length) + s.length) % s.length;
  const c = s[at];
  if ($("ly").value === "recolour" || c.includes(">")) { g.putImageData(recoloured(c), 0, 0); $("fr").textContent = `${c}  (${at + 1}/${s.length})`; return; }
  const im = img[`${c}.${$("ly").value}`];
  g.imageSmoothingEnabled = $("ly").value === "orig";
  g.fillStyle = "#000"; g.fillRect(0, 0, cv.width, cv.height);
  if (im && im.complete && im.naturalWidth) g.drawImage(im, 0, 0, cv.width, cv.height);
  $("fr").textContent = `${c}  (${at + 1}/${s.length})`;
}
function play(on) { playing = on; $("pp").textContent = on ? "pause" : "play"; }
function tick(t) { if (playing && t - last >= 1000 / Math.max(1, +$("fp").value || 6) / (1 + +$("tw").value)) { last = t; at++; draw(); } requestAnimationFrame(tick); }
$("pp").onclick = () => play(!playing);
$("rw").onchange = () => { at = 0; draw(); };
$("tw").onchange = () => { at = 0; draw(); };
$("ly").onchange = draw;
$("zm").onchange = size;
addEventListener("keydown", e => {
  if (e.target.tagName === "INPUT" || e.target.tagName === "SELECT") return;
  if (e.key === " ") { e.preventDefault(); play(!playing); }
  else if (e.key === "ArrowRight" || e.key === "ArrowLeft") { play(false); at += e.key === "ArrowRight" ? 1 : -1; draw(); }
});
// Recolouring: every pixel points at a palette entry (MX.k, a letter per pixel) and keeps its exact colour
// (MX.x, RGB bytes). An edited entry moves its pixels by the entry's change ("keep shading") or paints them the
// entry's colour ("flat"); the hue slider turns every entry round the colour wheel on top.
const NC = MX.c.length, AL = MX.letters, P = { c: MX.c.map(v => v.slice()), l: MX.l.map(v => v.slice()) };
const bytes = {}, at64 = s => Uint8Array.from(atob(s), ch => ch.charCodeAt(0));
function hueTurn([r, g, b], deg) {
  r /= 255; g /= 255; b /= 255;
  const mx = Math.max(r, g, b), mn = Math.min(r, g, b), l = (mx + mn) / 2, d = mx - mn;
  if (d === 0) return [r * 255, g * 255, b * 255];
  const sat = d / (1 - Math.abs(2 * l - 1));
  let h = mx === r ? ((g - b) / d) % 6 : mx === g ? (b - r) / d + 2 : (r - g) / d + 4;
  h = ((h * 60 + deg) % 360 + 360) % 360;
  const C = (1 - Math.abs(2 * l - 1)) * sat, X = C * (1 - Math.abs((h / 60) % 2 - 1)), m = l - C / 2;
  const [a, bb, cc] = h < 60 ? [C, X, 0] : h < 120 ? [X, C, 0] : h < 180 ? [0, C, X] : h < 240 ? [0, X, C] : h < 300 ? [X, 0, C] : [C, 0, X];
  return [(a + m) * 255, (bb + m) * 255, (cc + m) * 255];
}
function entry(kind, i) { const hue = +(document.getElementById("hue")?.value || 0); return hue ? hueTurn(P[kind][i], hue) : P[kind][i]; }
function recoloured(name) {
  const f = MX.f[name], out = g.createImageData(MX.w, MX.h), o = out.data;
  const x = bytes[name] || (bytes[name] = at64(f.x)), flat = document.getElementById("mode")?.value === "flat";
  const inks = { c: MX.c.map((_, i) => entry("c", i)), l: MX.l.map((_, i) => entry("l", i)) };
  for (let p = 0; p < f.k.length; p++) {
    const j = AL.indexOf(f.k[p]); o[p * 4 + 3] = 255;
    if (j < 0) continue;
    const kind = j < NC ? "c" : "l", i = j < NC ? j : j - NC, now = inks[kind][i], was = MX[kind][i];
    for (let ch = 0; ch < 3; ch++) o[p * 4 + ch] = flat ? now[ch] : x[p * 3 + ch] + now[ch] - was[ch];
  }
  return out;
}
window.recolour = { P, draw, show() { $("ly").value = "recolour"; draw(); } };
size(); draw(); requestAnimationFrame(tick);
</script>"##;

/// The rows of an extraction for the player, in order: `[{"row": "init", "cells": ["init-1", ...]}, ...]`.
fn player_rows(ex: &Extracted) -> Value {
    let mut rows: Vec<(String, Vec<String>)> = Vec::new();
    for c in &ex.cells {
        let row = tween::row(&c.name).to_string();
        match rows.last_mut() {
            Some((r, cells)) if *r == row => cells.push(c.name.clone()),
            _ => rows.push((row, vec![c.name.clone()])),
        }
    }
    Value::Array(
        rows.into_iter()
            .map(|(row, cells)| json!({"row": row, "cells": cells}))
            .collect(),
    )
}

/// The page of one effect: the player, then every frame's columns, then the frame diffs. `back` is the link to the
/// gallery (`../index.html#golden-bell`); `between` the in-betweens of the matrix's frames, `n` between each two
/// ([`tween::tweens`]).
pub fn effect_page(
    title: &str,
    back: &str,
    ex: &Extracted,
    m: &Matrix,
    between: &[Frame],
    n: usize,
) -> String {
    let heads: String = COLUMNS
        .iter()
        .map(|(_, h)| format!("<th>{h}</th>"))
        .collect();
    let rows: String = ex
        .cells
        .iter()
        .map(|c| {
            let tds: String = COLUMNS
                .iter()
                .map(|(k, _)| format!(r#"<td class="{k}"><img class="{k}" src="{}"></td>"#, file(&c.name, k)))
                .collect();
            format!(
                "<tr><th>{}<br><small>colour {} px<br>light {} px<br>err {:.2}<br>grid {:.2} × {:.2}<br>moved {:+},{:+}</small></th>{tds}</tr>",
                c.name, c.solid, c.lit, c.mean_err, c.strength.0, c.strength.1, c.shift.0, c.shift.1
            )
        })
        .collect();
    let diffs: String = ex
        .diffs
        .iter()
        .map(|d| {
            format!(
                r#"<figure><img src="{}"><figcaption>{}: {} px change</figcaption></figure>"#,
                diff_file(&d.name),
                d.name,
                d.changed
            )
        })
        .collect();
    let t = esc(title);
    format!(
        r#"<!doctype html><meta charset=utf-8><meta name=viewport content="width=device-width,initial-scale=1"><title>{t} extraction</title>
<style>{STYLE}</style>
<p><a href="{back}">← all effects</a></p>
<h1>{t}: effect pixels extracted</h1>
{player}
<p>Each row is one frame. Columns: <b>original</b> cell from the model, over black · <b>alpha</b> the model gave each pixel ·
<b>colour layer</b> (opaque pixels, checkerboard = empty) · <b>light layer</b> (additive glow, on black) ·
<b>rebuilt</b> (colour, else light, over black) · <b>through the palette</b> (each pixel's palette entry). The <b>model's pixels</b> column is the busiest part of the original blown up, with the
cuts between the model's pixels: each square should hold one flat colour. {grid}</p>
{palette}
<table><tr><th></th>{heads}</tr>{rows}</table>
<h2>Diff between neighbouring frames (green = brighter in the next frame, red = darker)</h2><div class=d>{diffs}</div>
"#,
        grid = grid_line(ex),
        palette = palette_section(m),
        player = PLAYER
            .replace("__MX__", &recolour_data(m, between).to_string())
            .replace("__N__", &n.to_string())
            .replace("__ROWS__", &player_rows(ex).to_string())
            .replace(
                "__W__",
                &ex.cells.first().map_or(0, |c| c.colour.w).to_string()
            )
            .replace(
                "__H__",
                &ex.cells.first().map_or(0, |c| c.colour.h).to_string()
            )
    )
}

/// The shared palettes as swatches, each with the letter `matrix.json` writes it as, and how far they miss.
fn palette_section(m: &Matrix) -> String {
    let letters: Vec<char> = pixfx_pixel::LEGEND.chars().collect();
    let swatches = |p: &[[u8; 3]], kind: &str| -> String {
        p.iter()
            .enumerate()
            .map(|(i, &[r, g, b])| {
                let l = letters.get(i).copied().unwrap_or('?');
                format!(
                    r##"<label class=sw title="{kind} {l}"><input type=color data-kind={kind} data-i={i} value="#{r:02x}{g:02x}{b:02x}">{l}</label>"##
                )
            })
            .collect()
    };
    format!(
        "<h2>Palette</h2><p>Every pixel keeps the exact colour the model drew (in <code>matrix.json</code>) and points at an \
entry of a palette the frames share: change an entry and every frame's pixels on it change. <b>through the palette</b> \
paints each frame from the palette. It misses the exact colours by {:.1}/255 (colour) and {:.1}/255 (light) per \
channel on average.</p><p><b>Try a palette:</b> click a swatch to change it, or turn the hue; the player switches to \
<b>recoloured</b> and every frame follows. <label>hue <input id=hue type=range min=-180 max=180 value=0></label> \
<label>mode <select id=mode><option value=shading>keep shading</option><option value=flat>flat palette</option></select></label> \
<button id=reset>reset</button></p><p>colour: {}</p><p>light: {}</p>{RECOLOUR_UI}",
        m.colour_err,
        m.light_err,
        swatches(&m.colours, "c"),
        swatches(&m.lights, "l")
    )
}

/// Wires the palette's colour inputs, hue, mode and reset to the player's recoloured layer.
const RECOLOUR_UI: &str = r#"<script>
for (const el of document.querySelectorAll("input[type=color]")) el.oninput = () => {
  const v = el.value, k = el.dataset.kind, i = +el.dataset.i;
  recolour.P[k][i] = [1, 3, 5].map(n => parseInt(v.slice(n, n + 2), 16)); recolour.show();
};
document.getElementById("hue").oninput = () => recolour.show();
document.getElementById("mode").onchange = () => recolour.show();
document.getElementById("reset").onclick = () => {
  for (const el of document.querySelectorAll("input[type=color]")) {
    el.value = el.defaultValue; recolour.P[el.dataset.kind][+el.dataset.i] = [1, 3, 5].map(n => parseInt(el.value.slice(n, n + 2), 16));
  }
  document.getElementById("hue").value = 0; recolour.show();
};
</script>"#;

/// The matrix for the page's recolouring: size, palettes, the letters, and per frame (the drawn ones, then
/// `between`) a letter per pixel (`.` empty, colour entries first, then light) and its exact RGB as base64.
fn recolour_data(m: &Matrix, between: &[Frame]) -> Value {
    let letters: Vec<char> = pixfx_pixel::LEGEND.chars().collect();
    let nc = m.colours.len();
    let frames: serde_json::Map<String, Value> = m
        .frames
        .iter()
        .chain(between)
        .map(|f| {
            let mut rgb = Vec::with_capacity(f.px.len() * 3);
            let k: String =
                f.px.iter()
                    .map(|p| {
                        let (v, j) = match *p {
                            Px::Empty => ([0; 3], None),
                            Px::Colour { rgb, ink } => (rgb, Some(usize::from(ink))),
                            Px::Light { rgb, ink } => (rgb, Some(nc + usize::from(ink))),
                        };
                        rgb.extend_from_slice(&v);
                        j.and_then(|j| letters.get(j).copied()).unwrap_or('.')
                    })
                    .collect();
            (
                f.name.clone(),
                json!({"k": k, "x": crate::base64::encode(&rgb)}),
            )
        })
        .collect();
    json!({"w": m.w, "h": m.h, "c": m.colours, "l": m.lights, "letters": pixfx_pixel::LEGEND, "f": frames})
}

/// The sentence about the sheet's grid: its size, pitch and fit, or that none was found.
fn grid_line(ex: &Extracted) -> String {
    let (w, h) = ex.size;
    if ex.found {
        format!(
            "Every frame is {w}×{h} model pixels, {:.2}×{:.2} sheet pixels each (gaps fit the pitch {:.2} × {:.2}); \
<b>grid</b> under each frame's name: how strongly its edges follow its cuts, 0 none, 1 perfect; <b>moved</b>: \
the model pixels the frame was moved (right, down) to line up with the others, its layers moved already.",
            ex.pitch.0, ex.pitch.1, ex.fit.0, ex.fit.1
        )
    } else {
        format!(
            "No grid of the model's showed: every frame is the template's {w}×{h} sprite pixels, even cells."
        )
    }
}

/// One effect in the gallery.
#[derive(Clone, Debug, PartialEq)]
pub struct Entry {
    /// The anchor and the effect id's last part: `golden-bell`.
    pub key: String,
    pub id: String,
    pub name: String,
    pub look: String,
    /// The whole sheet over black, relative to the gallery.
    pub sheet: String,
    /// The effect page, relative to the gallery.
    pub page: String,
    pub pitch: f64,
    pub mean_err: f64,
}

/// The gallery: a sticky bar of anchors, then a card per effect with its sheet (click to zoom) and a link to its
/// page.
pub fn gallery(title: &str, intro: &str, entries: &[Entry]) -> String {
    let nav: String = entries
        .iter()
        .map(|e| format!(r##"<a href="#{}">{}</a> "##, e.key, esc(&e.name)))
        .collect();
    let cards: String = entries
        .iter()
        .map(|e| {
            format!(
                r#"<figure id="{k}"><img src="{s}" alt="{n}" loading="lazy"><figcaption><b>{id}</b> · {n} · <a href="{p}">effect pixels, frame by frame →</a><br><span>{l}</span><br><span>pixel {pi:.2} · rebuild error {er:.2}/255</span></figcaption></figure>
"#,
                k = e.key,
                s = e.sheet,
                n = esc(&e.name),
                id = esc(&e.id),
                p = e.page,
                l = esc(&e.look),
                pi = e.pitch,
                er = e.mean_err
            )
        })
        .collect();
    let t = esc(title);
    format!(
        r#"<!doctype html><html lang="en"><meta charset="utf-8"><title>{t}</title>
<style>
body{{margin:0;background:#0f0f17;color:#ddd;font:15px/1.45 system-ui,sans-serif}}
header{{position:sticky;top:0;background:#0f0f17ee;padding:12px 20px;border-bottom:1px solid #2a2a3a;z-index:2}}
h1{{margin:0 0 6px;font-size:18px}} nav a{{color:#9cf;margin-right:10px;text-decoration:none;white-space:nowrap}}
main{{display:grid;grid-template-columns:repeat(auto-fill,minmax(420px,1fr));gap:20px;padding:20px}}
figure{{margin:0;background:#171724;border:1px solid #2a2a3a;border-radius:8px;overflow:hidden}}
img{{width:100%;display:block;cursor:zoom-in}} figcaption a{{color:#9cf}}
figcaption{{padding:10px 12px}} figcaption span{{color:#999}}
#zoom{{position:fixed;inset:0;background:#000d;display:none;align-items:center;justify-content:center;z-index:9;cursor:zoom-out}}
#zoom img{{max-width:96vw;max-height:96vh;width:auto;cursor:zoom-out}}
.intro{{padding:20px 20px 0;max-width:900px}}
</style>
<header><h1>{t}</h1><nav>{nav}</nav></header>
<section class="intro"><p>{intro}</p></section>
<main>{cards}</main>
<div id="zoom" onclick="this.style.display='none'"><img alt=""></div>
<script>
for (const im of document.querySelectorAll('main img')) im.onclick = () => {{
  const z = document.getElementById('zoom'); z.querySelector('img').src = im.src; z.style.display = 'flex'; }};
addEventListener('keydown', e => {{ if (e.key === 'Escape') document.getElementById('zoom').style.display = 'none'; }});
</script></html>
"#,
        intro = esc(intro)
    )
}

/// The numbers of an extraction as JSON (`report.json`): the sheet's grid (found or not, size, pitch, fit), and
/// per cell its rectangle on the sheet, pitch, grid strength and cuts, pixel counts and error, and per diff its changed
/// pixels.
pub fn report(ex: &Extracted) -> Value {
    json!({
        "grid": {"found": ex.found, "size": [ex.size.0, ex.size.1], "pitch": [ex.pitch.0, ex.pitch.1], "fit": [ex.fit.0, ex.fit.1]},
        "cells": ex.cells.iter().map(|c| json!({
            "cell": c.name, "rect": c.rect, "pitch": [c.pitch.0, c.pitch.1], "strength": [c.strength.0, c.strength.1], "shift": [c.shift.0, c.shift.1],
            "cuts": {"cols": c.grid.cols, "rows": c.grid.rows},
            "solid": c.solid, "light": c.lit, "mean_err": c.mean_err,
        })).collect::<Vec<_>>(),
        "diffs": ex.diffs.iter().map(|d| json!({"diff": d.name, "changed": d.changed})).collect::<Vec<_>>(),
    })
}
