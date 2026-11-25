//! What every looping status ailment (burning, and the ones to come) reads of the body it plays on: the sprite's
//! [`Silhouette`] with the colour and brightness of each covered cell, read once per pose in `set_actor`; the
//! stand-in figure drawn to the ruler when the game hands no sprite; and the warm or cold light under the feet.
//! Nothing here belongs to one ailment.

use pixfx_core::{Actor, Area, Silhouette};
use pixfx_draw::RULER;
use pixfx_draw::color::{Rgba, luma};

pub(crate) const ORIGIN: (i32, i32) = RULER.feet;
pub(crate) const SIZE: (u32, u32) = (RULER.cell, RULER.cell + 4);

/// The status canvas in game coordinates, with room above the crown and below the feet.
pub(crate) const BOX: Area = Area {
    x: -ORIGIN.0,
    y: -ORIGIN.1,
    w: SIZE.0,
    h: SIZE.1,
};

/// pixelfx's `colorAt` fallback: the mid grey the stand-in body is read as.
const STAND_GREY: Rgba = Rgba::hex24(0x6b6b6b);

/// One covered cell of the body: its colour and its perceived brightness (0..1).
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Cell {
    pub x: i32,
    pub y: i32,
    pub c: Rgba,
    pub l: f64,
}

/// The body a status plays on, read once per pose.
pub(crate) struct Body {
    pub sil: Silhouette,
    /// Every covered cell with its colour, row by row.
    pub cells: Vec<Cell>,
    /// The smallest box round the body.
    pub bounds: Area,
    /// `rows[j]` is `(left, right)` of row `bounds.y + j`; `(0, -1)` for an empty row. Petrified reads no row.
    #[cfg(any(
        feature = "feedback-burning",
        feature = "feedback-frozen",
        feature = "feedback-poisoned"
    ))]
    pub rows: Vec<(i32, i32)>,
}

impl Body {
    /// The sprite's body in `area`, or the stand-in figure when there is no sprite or it covers none of the area.
    pub(crate) fn read(actor: Option<&Actor>, area: Area) -> Body {
        let sprite = actor.map(|a| Silhouette::of(a, area));
        match (actor, sprite) {
            (Some(a), Some(sil)) if !sil.is_empty() => {
                let cells = a
                    .opaque()
                    .filter(|&(x, y, _)| area.contains(x, y))
                    .map(|(x, y, c)| Cell {
                        x,
                        y,
                        c,
                        l: luma(c),
                    })
                    .collect();
                Self::from(sil, cells)
            }
            _ => {
                let sil = stand_in(area);
                let cells = sil
                    .cells()
                    .map(|(x, y)| Cell {
                        x,
                        y,
                        c: STAND_GREY,
                        l: luma(STAND_GREY),
                    })
                    .collect();
                Self::from(sil, cells)
            }
        }
    }

    fn from(sil: Silhouette, cells: Vec<Cell>) -> Body {
        let bounds = sil.bounds().unwrap_or(Area {
            x: 0,
            y: 0,
            w: 0,
            h: 0,
        });
        #[cfg(any(
            feature = "feedback-burning",
            feature = "feedback-frozen",
            feature = "feedback-poisoned"
        ))]
        let rows = (0..bounds.h as i32)
            .map(|j| {
                let y = bounds.y + j;
                let mut row = (0, -1);
                for x in bounds.x..bounds.x + bounds.w as i32 {
                    if sil.covers(x, y) {
                        if row.1 < row.0 {
                            row.0 = x;
                        }
                        row.1 = x;
                    }
                }
                row
            })
            .collect();
        Body {
            sil,
            cells,
            bounds,
            #[cfg(any(
                feature = "feedback-burning",
                feature = "feedback-frozen",
                feature = "feedback-poisoned"
            ))]
            rows,
        }
    }
}

/// A stand-in fighter with a one-head crown, a half-width trunk and separated arms and legs.
fn stand_in(area: Area) -> Silhouette {
    let height = f64::from(RULER.height);
    let head_radius = f64::from(RULER.head) * 0.5;
    let half_width = f64::from(RULER.width) * 0.5;
    let trunk_edge = half_width * 0.5;
    let hip = -height * (14.0 / 38.0);
    let shoulder = -height + 2.0 * head_radius;
    let legs_edge = f64::from(RULER.limb) * (4.0 / 3.0);
    Silhouette::from_fn(area, |px, py| {
        let ax = libm::fabs(px);
        let head = libm::hypot(px, py + height - head_radius) < head_radius;
        let trunk = ax < trunk_edge && (shoulder..hip).contains(&py);
        let arms = (trunk_edge..half_width).contains(&ax)
            && (-height * (29.0 / 38.0)..-height * (18.0 / 38.0)).contains(&py);
        let legs = (legs_edge * 0.25..legs_edge).contains(&ax) && (hip..0.0).contains(&py);
        head || trunk || arms || legs
    })
}

/// The ellipse of light under the feet, `rx` by `ry` with its middle half a pixel under the origin, colour `c` at
/// strength `k` in the middle falling to nothing at the rim (the floor flicker, cold floor light or sick glow the
/// sources draw with `shade`). Light only: it paints no colour. Petrified's floor is dust, not light.
#[cfg(any(
    feature = "feedback-burning",
    feature = "feedback-frozen",
    feature = "feedback-poisoned"
))]
pub(crate) fn floor_light(p: &mut pixfx_draw::Painter, rx: f64, ry: f64, c: Rgba, k: f64) {
    p.shade(-rx - 1.0, -3.0, rx + 1.0, 7.0, |x, y, out| {
        let d = libm::hypot(x / rx, (y - 0.5) / ry);
        if d < 1.0 {
            out.l = c;
            out.k = k * (1.0 - d) * (1.0 - d);
        }
        None
    });
}
