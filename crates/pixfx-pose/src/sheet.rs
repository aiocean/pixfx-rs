//! Skeletons drawn into a sheet an image model draws a character into: one posed skeleton a cell, each bone in its
//! own colour (OpenPose style, so left and right and the sword cannot be confused), and a caption above every cell
//! naming the pose. The cells keep the geometry of `pixfx_pixel::sheet`, so its `cut` takes the drawing apart.

use pixfx_draw::{Rgba, RgbaImage};
use pixfx_pixel::sheet::{Layout, blank, cell_rect, check};
use serde::{Deserialize, Serialize};

use crate::font;
use pixfx_body::Body;
use pixfx_body::Pose;
use pixfx_body::{Bone, Point, Skeleton, facing, skeleton};

/// The colour of the head circle and the nose dot.
pub const HEAD: Rgba = Rgba(160, 0, 255, 255);
/// Caption ink.
const INK: [u8; 4] = [20, 20, 20, 255];

/// The colour a bone is drawn in.
pub const fn colour(bone: Bone) -> Rgba {
    match bone {
        Bone::Neck | Bone::Spine | Bone::Shoulders | Bone::Hips => Rgba(220, 0, 0, 255),
        Bone::RightUpperArm => Rgba(255, 140, 0, 255),
        Bone::RightForearm => Rgba(230, 200, 0, 255),
        Bone::LeftUpperArm => Rgba(120, 220, 0, 255),
        Bone::LeftForearm => Rgba(0, 150, 0, 255),
        Bone::RightThigh => Rgba(0, 170, 120, 255),
        Bone::RightShin => Rgba(0, 105, 85, 255),
        Bone::LeftThigh => Rgba(0, 110, 255, 255),
        Bone::LeftShin => Rgba(0, 0, 190, 255),
        Bone::Sword => Rgba(80, 80, 80, 255),
    }
}

/// The colour key as prompt lines, to paste into the image model's prompt so its words match the sheet's colours.
pub fn legend() -> String {
    [
        "- purple circle = head, purple dot = nose (the way the face points; no dot = seen from behind)",
        "- red = neck, spine, shoulder line and hip line",
        "- orange then yellow = her RIGHT arm (upper arm, forearm)",
        "- light green then dark green = her LEFT arm",
        "- teal then dark teal = her RIGHT leg (thigh, shin)",
        "- blue then dark blue = her LEFT leg",
        "- dark grey = the sword blade, from the hand holding it to the tip",
        "- round dots = elbows, wrists, knees and ankles",
    ]
    .join("\n")
}

/// One cell: a pose seen from a facing, and the caption the model reads above it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Cell {
    pub caption: String,
    pub pose: Pose,
    /// Degrees, see [`facing`].
    pub facing: f64,
}

/// A whole sheet: the figure, the cell grid, the cells in reading order.
#[derive(Clone, Debug, PartialEq)]
pub struct PoseSheet {
    pub body: Body,
    /// Cell size in sprite pixels.
    pub cell: (u32, u32),
    pub cols: u32,
    /// Sprite pixels between cells and round the edge; the caption is drawn in the gap above its cell, so a gap
    /// under 2 leaves no room for one.
    pub gap: u32,
    /// Sheet pixels a sprite pixel.
    pub scale: u32,
    /// Where the feet stand, in sprite pixels up from the bottom of the cell.
    pub ground: u32,
    pub cells: Vec<Cell>,
}

impl Default for PoseSheet {
    /// The duel's ruler (`pixfx_draw::RULER`): its 64x64 cell, its figure ([`Body::default`]) with the
    /// soles on its feet row; four across, a gap of 8 for two caption lines, drawn 8x.
    fn default() -> Self {
        let r = pixfx_draw::RULER;
        let feet_up = r
            .cell
            .saturating_sub(u32::try_from(r.feet.1 + 1).unwrap_or(0));
        PoseSheet {
            body: Body::default(),
            cell: (r.cell, r.cell),
            cols: 4,
            gap: 8,
            scale: 8,
            ground: feet_up,
            cells: Vec::new(),
        }
    }
}

/// A rendered sheet: the picture, the grid `pixfx_pixel::sheet::cut` takes back, and each cell's skeleton in sprite
/// pixels from its cell's top-left.
#[derive(Clone, Debug, PartialEq)]
pub struct Drawn {
    pub image: RgbaImage,
    pub layout: Layout,
    pub skeletons: Vec<Skeleton>,
}

/// One pose in every facing, eight cells captioned `<label>: <facing>`.
pub fn turnaround(label: &str, pose: &Pose) -> Vec<Cell> {
    facing::ALL
        .iter()
        .map(|&f| Cell {
            caption: format!("{label}: {}", facing::name(f)),
            pose: *pose,
            facing: f,
        })
        .collect()
}

impl PoseSheet {
    /// The grid these cells fill: as many rows as the cells need.
    pub fn layout(&self) -> Layout {
        let cols = self.cols.max(1);
        let n = u32::try_from(self.cells.len()).unwrap_or(u32::MAX);
        Layout {
            cell_w: self.cell.0,
            cell_h: self.cell.1,
            cols,
            rows: n.div_ceil(cols).max(1),
            gap: self.gap,
            scale: self.scale.max(1),
        }
    }

    /// Draw every cell. Fails on a grid too big to draw (see `pixfx_pixel::sheet::check`) or a caption too long
    /// for the gap above its cell, naming the cell.
    pub fn render(&self) -> Result<Drawn, String> {
        let layout = self.layout();
        check(&layout)?;
        let mut image = blank(&layout, false);
        let s = f64::from(layout.scale);
        let (cw, ch) = (f64::from(layout.cell_w), f64::from(layout.cell_h));
        let ground = ch - f64::from(self.ground);
        let texts: Vec<&str> = self.cells.iter().map(|c| c.caption.as_str()).collect();
        let k = caption_size(&layout, &texts)?;
        let mut skeletons = Vec::with_capacity(self.cells.len());
        for (i, cell) in self.cells.iter().enumerate() {
            let i = u32::try_from(i).unwrap_or(u32::MAX);
            let (col, row) = (i % layout.cols, i / layout.cols);
            let rect = cell_rect(&layout, col, row);
            let sk =
                skeleton(&self.body, &cell.pose, cell.facing).placed(cw, ground, cell.pose.lift);
            draw_skeleton(&mut image, rect, s, &sk);
            if !cell.caption.trim().is_empty() {
                caption(&mut image, &layout, rect, &cell.caption, k);
            }
            skeletons.push(sk);
        }
        Ok(Drawn {
            image,
            layout,
            skeletons,
        })
    }
}

/// Write each cell's caption into the gap above it, cells in reading order (`texts[i]` over cell `i`, left to right,
/// top to bottom; an empty text draws nothing), all at one font size: the biggest that fits the longest. Fails
/// naming the first caption too long for the gap.
pub fn captions(img: &mut RgbaImage, layout: &Layout, texts: &[&str]) -> Result<(), String> {
    let k = caption_size(layout, texts)?;
    for (i, text) in texts.iter().enumerate() {
        let i = u32::try_from(i).unwrap_or(u32::MAX);
        let (col, row) = (i % layout.cols.max(1), i / layout.cols.max(1));
        if !text.trim().is_empty() && row < layout.rows {
            caption(img, layout, cell_rect(layout, col, row), text, k);
        }
    }
    Ok(())
}

/// The one font size every caption of a sheet is drawn at: the biggest that fits the longest.
fn caption_size(layout: &Layout, texts: &[&str]) -> Result<u32, String> {
    let band = layout.gap * layout.scale;
    let mut k = 4;
    for (i, text) in texts.iter().enumerate() {
        if text.trim().is_empty() {
            continue;
        }
        let fits = fit(text, layout.cell_w * layout.scale, band).ok_or_else(|| {
            let i = u32::try_from(i).unwrap_or(u32::MAX);
            format!(
                "cell {} ({},{}): caption \"{}\" does not fit the gap above the cell: shorten it, or raise the gap or the scale",
                i + 1,
                i % layout.cols.max(1),
                i / layout.cols.max(1),
                text
            )
        })?;
        k = k.min(fits);
    }
    Ok(k)
}

/// What is drawn for a skeleton, back to front by depth.
enum Mark {
    Line(Point, Point, f64),
    Dot(Point, f64),
    Ring(Point, f64, f64),
}

fn draw_skeleton(img: &mut RgbaImage, rect: [u32; 4], s: f64, sk: &Skeleton) {
    let (line, dot) = (s * 1.5, s * 1.1);
    let mut marks: Vec<(f64, Rgba, Mark)> = Vec::new();
    for (bone, a, b) in sk.bones() {
        let c = colour(bone);
        marks.push(((a.depth + b.depth) / 2.0, c, Mark::Line(a, b, line)));
        let jointed = !matches!(
            bone,
            Bone::Neck | Bone::Spine | Bone::Shoulders | Bone::Hips
        );
        if jointed {
            marks.push((b.depth + 0.01, c, Mark::Dot(b, dot)));
        }
    }
    marks.push((
        sk.head.depth,
        HEAD,
        Mark::Ring(sk.head, sk.head_radius, s * 1.25),
    ));
    if sk.nose_visible {
        marks.push((sk.nose.depth + 0.01, HEAD, Mark::Dot(sk.nose, s * 1.3)));
    }
    marks.sort_by(|a, b| a.0.total_cmp(&b.0));
    let [x0, y0, _, _] = rect;
    let to = |p: Point| (f64::from(x0) + p.x * s, f64::from(y0) + p.y * s);
    for (_, c, m) in marks {
        let px = [c.0, c.1, c.2, c.3];
        match m {
            Mark::Line(a, b, t) => {
                let (a, b) = (to(a), to(b));
                paint(img, rect, bbox(&[a, b], t), px, |p| {
                    seg_dist(p, a, b) <= t / 2.0
                });
            }
            Mark::Dot(p, r) => {
                let c = to(p);
                paint(img, rect, bbox(&[c], r), px, |q| dist(q, c) <= r);
            }
            Mark::Ring(p, r, t) => {
                let (c, r) = (to(p), r * s);
                paint(img, rect, bbox(&[c], r + t), px, |q| {
                    (dist(q, c) - r).abs() <= t / 2.0
                });
            }
        }
    }
}

fn bbox(pts: &[(f64, f64)], pad: f64) -> [f64; 4] {
    pts.iter().fold(
        [f64::MAX, f64::MAX, f64::MIN, f64::MIN],
        |[l, t, r, b], &(x, y)| {
            [
                l.min(x - pad),
                t.min(y - pad),
                r.max(x + pad),
                b.max(y + pad),
            ]
        },
    )
}

/// Set every pixel of `area` (sheet pixels, clipped to the cell `rect`) whose centre passes `inside`.
fn paint(
    img: &mut RgbaImage,
    rect: [u32; 4],
    area: [f64; 4],
    px: [u8; 4],
    inside: impl Fn((f64, f64)) -> bool,
) {
    let [x0, y0, w, h] = rect;
    let clip = |v: f64, lo: u32, len: u32| {
        let v = libm::floor(v).max(f64::from(lo));
        (v.min(f64::from(lo + len))) as u32
    };
    let (l, t) = (clip(area[0], x0, w), clip(area[1], y0, h));
    let (r, b) = (clip(area[2] + 1.0, x0, w), clip(area[3] + 1.0, y0, h));
    for y in t..b {
        for x in l..r {
            if inside((f64::from(x) + 0.5, f64::from(y) + 0.5)) {
                img.set(x, y, px);
            }
        }
    }
}

fn dist(a: (f64, f64), b: (f64, f64)) -> f64 {
    libm::hypot(a.0 - b.0, a.1 - b.1)
}

fn seg_dist(p: (f64, f64), a: (f64, f64), b: (f64, f64)) -> f64 {
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let len2 = dx * dx + dy * dy;
    let t = if len2 > 0.0 {
        (((p.0 - a.0) * dx + (p.1 - a.1) * dy) / len2).clamp(0.0, 1.0)
    } else {
        0.0
    };
    dist(p, (a.0 + t * dx, a.1 + t * dy))
}

/// Words wrapped to lines of at most `width` characters; a word longer than a line is split.
fn wrap(text: &str, width: usize) -> Vec<String> {
    let width = width.max(1);
    let mut lines: Vec<String> = Vec::new();
    let mut line = String::new();
    for word in text.split_whitespace() {
        let mut word: Vec<char> = word.chars().collect();
        while word.len() > width {
            if !line.is_empty() {
                lines.push(std::mem::take(&mut line));
            }
            let rest = word.split_off(width);
            lines.push(word.into_iter().collect());
            word = rest;
        }
        let n = line.chars().count();
        if n > 0 && n + 1 + word.len() > width {
            lines.push(std::mem::take(&mut line));
        }
        if !line.is_empty() {
            line.push(' ');
        }
        line.extend(word);
    }
    if !line.is_empty() {
        lines.push(line);
    }
    lines
}

/// Font pixels a caption line takes: 7 of glyph, 3 of space.
const LINE: u32 = 10;

/// The biggest font size (1 to 4) at which `text` wraps into a band `band` pixels high over a cell `w` wide.
fn fit(text: &str, w: u32, band: u32) -> Option<u32> {
    (1..=4u32).rev().find(|&k| {
        let per_line = (w / (6 * k)) as usize;
        let n = u32::try_from(wrap(text, per_line).len()).unwrap_or(u32::MAX);
        per_line > 0 && n.saturating_mul(LINE * k).saturating_add(2) <= band
    })
}

/// Write `text` at font size `k` into the gap above the cell at `rect`.
fn caption(img: &mut RgbaImage, layout: &Layout, rect: [u32; 4], text: &str, k: u32) {
    let band = layout.gap * layout.scale;
    let [x0, y0, w, _] = rect;
    let lines = wrap(text, (w / (6 * k)) as usize);
    let need = u32::try_from(lines.len()).unwrap_or(0) * LINE * k;
    let top = y0 - band + band.saturating_sub(need) / 2 + k;
    for (li, line) in lines.iter().enumerate() {
        let ly = top + u32::try_from(li).unwrap_or(0) * LINE * k;
        for (ci, c) in line.chars().enumerate() {
            let cx = x0 + u32::try_from(ci).unwrap_or(0) * 6 * k;
            for gy in 0..font::H {
                for gx in 0..font::W {
                    if font::ink(c, gx, gy) {
                        for dy in 0..k {
                            for dx in 0..k {
                                img.set(cx + gx * k + dx, ly + gy * k + dy, INK);
                            }
                        }
                    }
                }
            }
        }
    }
}
