//! What every looping status ailment (burning, and the ones to come) reads of the body it plays on: the sprite's
//! [`Silhouette`] with the colour and brightness of each covered cell, read once per pose in `set_actor`; the
//! stand-in figure drawn to the ruler when the game hands no sprite; and the warm or cold light under the feet.
//! Nothing here belongs to one ailment.

use pixfx_core::{Actor, Area, Silhouette};
use pixfx_draw::Painter;
use pixfx_draw::color::{Rgba, luma};

/// The whole canvas of a status (64 x 64, origin (32, 55)): the box the body is read in.
pub(crate) const BOX: Area = Area {
    x: -32,
    y: -55,
    w: 64,
    h: 64,
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
    /// `rows[j]` is `(left, right)` of row `bounds.y + j`; `(0, -1)` for an empty row.
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
            rows,
        }
    }

    /// The covered cells whose neighbour at (`dx`, `dy`) is uncovered, row by row: pixelfx's `up` (0, -1), `side`
    /// (-1, 0) and (1, 0), and the cells something hangs from (0, 1).
    pub(crate) fn open(&self, dx: i32, dy: i32) -> impl Iterator<Item = (i32, i32)> + '_ {
        self.sil
            .cells()
            .filter(move |&(x, y)| !self.sil.covers(x + dx, y + dy))
    }
}

/// The stand-in figure to the ruler (`RULER`: 40 rows, head 7): a head disc `size(head, 1.0)` = 7 across, a trunk
/// `size(width, 0.5)` = 10 wide, arms `size(limb, 0.67)` = 2 thick, legs `size(limb, 1.0)` = 3 thick, standing on
/// the origin. It has up cells (the crown, the shoulders), hang cells (under the arms, the chin), two flanks and a
/// right edge at 60 % height, so every status finds its spots on it.
fn stand_in(area: Area) -> Silhouette {
    Silhouette::from_fn(area, |px, py| {
        let ax = libm::fabs(px);
        let head = libm::hypot(px, py + 34.5) < 3.5;
        let trunk = ax < 5.0 && (-31.0..-14.0).contains(&py);
        let arms = (5.0..7.0).contains(&ax) && (-29.0..-18.0).contains(&py);
        let legs = (1.0..4.0).contains(&ax) && (-14.0..0.0).contains(&py);
        head || trunk || arms || legs
    })
}

/// The ellipse of light under the feet, `rx` by `ry` with its middle half a pixel under the origin, colour `c` at
/// strength `k` in the middle falling to nothing at the rim (the floor flicker, cold floor light or sick glow the
/// sources draw with `shade`). Light only: it paints no colour.
pub(crate) fn floor_light(p: &mut Painter, rx: f64, ry: f64, c: Rgba, k: f64) {
    p.shade(-rx - 1.0, -3.0, rx + 1.0, 7.0, |x, y, out| {
        let d = libm::hypot(x / rx, (y - 0.5) / ry);
        if d < 1.0 {
            out.l = c;
            out.k = k * (1.0 - d) * (1.0 - d);
        }
        None
    });
}
