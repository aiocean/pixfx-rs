//! Scale: a sprite measured against the game's ruler ([`Ruler`]), so everything in a scene keeps one pixel size.
//! A standing character must be the ruler's height; a thing drawn beside it (glasses, a hat, a sword) must be as many
//! cells as the body part it is measured against, times the ratio, rounded and never under one
//! ([`Ruler::size`]). Cells are counted painted at alpha `OPAQUE` and above.

use pixfx_draw::Ruler;

use crate::critique::{Issue, Severity};
use crate::{OPAQUE, RgbaImage};

/// The painted cells' bounds, `(x, y, w, h)`; `None` for a picture with none.
fn bounds(img: &RgbaImage) -> Option<(u32, u32, u32, u32)> {
    let (mut x0, mut y0, mut x1, mut y1) = (u32::MAX, u32::MAX, 0, 0);
    for y in 0..img.h {
        for x in 0..img.w {
            if img.get(x, y).is_some_and(|p| p[3] >= OPAQUE) {
                (x0, y0, x1, y1) = (x0.min(x), y0.min(y), x1.max(x), y1.max(y));
            }
        }
    }
    (x0 != u32::MAX).then(|| (x0, y0, x1 + 1 - x0, y1 + 1 - y0))
}

/// The length of the ruler's part `name` (`height`, `width`, `head`, `eye` (its width), `hand`, `limb`,
/// `blade`, `outline`); `None` for another name.
pub fn part(ruler: &Ruler, name: &str) -> Option<u32> {
    Some(match name {
        "height" => ruler.height,
        "width" => ruler.width,
        "head" => ruler.head,
        "eye" => ruler.eye.0,
        "hand" => ruler.hand,
        "limb" => ruler.limb,
        "blade" => ruler.blade,
        "outline" => ruler.outline,
        _ => return None,
    })
}

/// A sprite of a character standing, measured against `ruler`: an error when its painted rows are not the ruler's
/// height, a warning when its width is off by more than a quarter of the ruler's.
pub fn standing(img: &RgbaImage, ruler: &Ruler) -> Vec<Issue> {
    let Some((x, y, w, h)) = bounds(img) else {
        return Vec::new();
    };
    let mut issues = Vec::new();
    if h != ruler.height {
        issues.push(Issue {
            check: "scale",
            severity: Severity::Error,
            cells: vec![(x, y), (x, y + h - 1)],
            message: format!(
                "a standing character {h} rows tall where the ruler says {} (head {}, eye {}x{}): beside the game's other sprites it is {:.2} times their size",
                ruler.height,
                ruler.head,
                ruler.eye.0,
                ruler.eye.1,
                f64::from(h) / f64::from(ruler.height.max(1))
            ),
            fix: format!(
                "draw it again at {} rows: `clean --height {}` resamples a model's drawing; painter code takes the ruler's parts",
                ruler.height, ruler.height
            ),
        });
    }
    if w.abs_diff(ruler.width) * 4 > ruler.width {
        issues.push(Issue {
            check: "scale",
            severity: Severity::Warning,
            cells: vec![(x, y), (x + w - 1, y)],
            message: format!("a standing character {w} columns wide where the ruler says {}", ruler.width),
            fix: "check the build against the ruler's body: a wider robe or weapon is fine, a wider body is not"
                .to_string(),
        });
    }
    issues
}

/// A thing measured against a body part: its longer painted side should be `Ruler::size(part, ratio)` cells (a lens
/// as wide as the eye: `part` the eye's width, `ratio` 1); a warning when it is off by more than one cell.
pub fn against(img: &RgbaImage, what: &str, part: u32, ratio: f64) -> Vec<Issue> {
    let Some((x, y, w, h)) = bounds(img) else {
        return Vec::new();
    };
    let want = Ruler::size(part, ratio);
    let side = w.max(h);
    if side.abs_diff(want) <= 1 {
        return Vec::new();
    }
    vec![Issue {
        check: "scale",
        severity: Severity::Warning,
        cells: vec![(x, y), (x + w - 1, y + h - 1)],
        message: format!("{side} cells long where {what} gives {want} ({part} x {ratio})"),
        fix: format!("draw it {want} cells long, or say why it is not as big as {what}"),
    }]
}
