//! A path in pixels, made of pieces joined end to end: straight lines, quadratic curves and turns round a centre
//! (an arc of a circle, or a spiral when the radius changes on the way). Anything that goes from one place to
//! another along a way it is given reads it here: a thrown piece flying a loop before it strikes
//! (`pixfx_core::volley`), growth laid along a circle (`crate::growth::Tree::path` with [`Path::points`]).
//!
//! A path is read by distance along it: [`Path::at`] and [`Path::heading`] take pixels from its start, so a thing
//! moving on it at a steady speed moves the same pixels a second on a tight turn as on a straight. Each piece's
//! length is measured once, when it is added. A line and an arc of a circle are measured exactly; a curve and a
//! spiral by straight pieces (32 for a curve, 32 a whole turn for a spiral).
//!
//! It is built from its start, each piece beginning where the last ended, so a path is never broken:
//!
//! ```
//! use core::f64::consts::TAU;
//! use pixfx_draw::path::Path;
//! // out to the right, once round (10, 0), then curving down onto (40, 30)
//! let p = Path::begin((0.0, 0.0)).line_to((10.0, -10.0)).turn((10.0, 0.0), TAU).quad_to((30.0, -10.0), (40.0, 30.0));
//! assert!(p.length() > 60.0);
//! ```

use core::f64::consts::TAU;

use crate::math::clamp;

/// One piece of a [`Path`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Piece {
    /// Straight from `from` to `to`.
    Line { from: (f64, f64), to: (f64, f64) },
    /// A quadratic curve from `from`, bent toward `via`, to `to`.
    Quad {
        from: (f64, f64),
        via: (f64, f64),
        to: (f64, f64),
    },
    /// Round `centre` from the angle `start` (radians, 0 along +x, y down) through `sweep` radians (positive turns
    /// clockwise on screen; several whole turns go round several times), its radius going from `r0` to `r1`: an arc
    /// of a circle when they are equal, else a spiral.
    Turn {
        centre: (f64, f64),
        start: f64,
        sweep: f64,
        r0: f64,
        r1: f64,
    },
}

/// The straight pieces a curve is measured by.
const QUAD_PIECES: u32 = 32;

/// The straight pieces a whole turn of a spiral is measured by.
const SPIRAL_PIECES_PER_TURN: f64 = 32.0;

impl Piece {
    /// The point `u` (0 to 1) along it by its own parameter.
    fn point(&self, u: f64) -> (f64, f64) {
        let u = clamp(u, 0.0, 1.0);
        match *self {
            Piece::Line { from, to } => {
                (from.0 + (to.0 - from.0) * u, from.1 + (to.1 - from.1) * u)
            }
            Piece::Quad { from, via, to } => {
                let v = 1.0 - u;
                (
                    v * v * from.0 + 2.0 * v * u * via.0 + u * u * to.0,
                    v * v * from.1 + 2.0 * v * u * via.1 + u * u * to.1,
                )
            }
            Piece::Turn {
                centre,
                start,
                sweep,
                r0,
                r1,
            } => {
                let a = start + sweep * u;
                let r = r0 + (r1 - r0) * u;
                (centre.0 + r * libm::cos(a), centre.1 + r * libm::sin(a))
            }
        }
    }

    /// The way it heads at `u`, radians; its overall way where it has none there.
    fn heading(&self, u: f64) -> f64 {
        let u = clamp(u, 0.0, 1.0);
        match *self {
            Piece::Line { from, to } => libm::atan2(to.1 - from.1, to.0 - from.0),
            Piece::Quad { from, via, to } => {
                let dx = 2.0 * (1.0 - u) * (via.0 - from.0) + 2.0 * u * (to.0 - via.0);
                let dy = 2.0 * (1.0 - u) * (via.1 - from.1) + 2.0 * u * (to.1 - via.1);
                if dx * dx + dy * dy > 1e-12 {
                    libm::atan2(dy, dx)
                } else {
                    libm::atan2(to.1 - from.1, to.0 - from.0)
                }
            }
            Piece::Turn {
                start,
                sweep,
                r0,
                r1,
                ..
            } => {
                let a = start + sweep * u;
                let r = r0 + (r1 - r0) * u;
                let (c, s) = (libm::cos(a), libm::sin(a));
                let dx = (r1 - r0) * c - r * sweep * s;
                let dy = (r1 - r0) * s + r * sweep * c;
                if dx * dx + dy * dy > 1e-12 {
                    libm::atan2(dy, dx)
                } else {
                    // at a spiral's still centre: the way round
                    a + if sweep < 0.0 { -TAU / 4.0 } else { TAU / 4.0 }
                }
            }
        }
    }

    /// The straight pieces it is measured by, or none when it is measured exactly.
    fn sampled(&self) -> Option<u32> {
        match *self {
            Piece::Line { .. } => None,
            Piece::Quad { .. } => Some(QUAD_PIECES),
            Piece::Turn { sweep, r0, r1, .. } => {
                if r0 == r1 {
                    None
                } else {
                    let n = libm::ceil(libm::fabs(sweep) / TAU * SPIRAL_PIECES_PER_TURN);
                    Some(clamp(n, 8.0, 4096.0) as u32)
                }
            }
        }
    }

    /// The `u` `dist` pixels along it, and its whole length.
    fn u_at(&self, dist: f64) -> (f64, f64) {
        let Some(pieces) = self.sampled() else {
            let len = match *self {
                Piece::Line { from, to } => libm::hypot(to.0 - from.0, to.1 - from.1),
                Piece::Turn { sweep, r0, .. } => libm::fabs(sweep * r0),
                Piece::Quad { .. } => 0.0,
            };
            let u = if len > 1e-12 {
                clamp(dist / len, 0.0, 1.0)
            } else {
                1.0
            };
            return (u, len);
        };
        let mut len = 0.0;
        let mut last = self.point(0.0);
        let mut found = None;
        for i in 1..=pieces {
            let u = f64::from(i) / f64::from(pieces);
            let p = self.point(u);
            let step = libm::hypot(p.0 - last.0, p.1 - last.1);
            if found.is_none() && len + step >= dist {
                let k = if step > 1e-12 {
                    (dist - len) / step
                } else {
                    0.0
                };
                found = Some((f64::from(i - 1) + clamp(k, 0.0, 1.0)) / f64::from(pieces));
            }
            len += step;
            last = p;
        }
        (found.unwrap_or(1.0), len)
    }

    fn end(&self) -> (f64, f64) {
        match *self {
            Piece::Line { to, .. } | Piece::Quad { to, .. } => to,
            Piece::Turn { .. } => self.point(1.0),
        }
    }
}

/// A path: where it starts and its pieces, each starting where the one before ended, with the distance along the
/// path at the end of each.
#[derive(Clone, Debug, PartialEq)]
pub struct Path {
    start: (f64, f64),
    pieces: Vec<(Piece, f64)>,
}

impl Path {
    /// A path at `start` that goes nowhere yet.
    pub fn begin(start: (f64, f64)) -> Path {
        Path {
            start,
            pieces: Vec::new(),
        }
    }

    /// A straight path.
    pub fn line(from: (f64, f64), to: (f64, f64)) -> Path {
        Path::begin(from).line_to(to)
    }

    /// A quadratic curve from `from`, bent toward `via`, to `to`.
    pub fn quad(from: (f64, f64), via: (f64, f64), to: (f64, f64)) -> Path {
        Path::begin(from).quad_to(via, to)
    }

    /// Straight from point to point; nowhere for no points.
    pub fn through(points: &[(f64, f64)]) -> Path {
        let mut it = points.iter().copied();
        let Some(first) = it.next() else {
            return Path::begin((0.0, 0.0));
        };
        it.fold(Path::begin(first), Path::line_to)
    }

    fn push(mut self, piece: Piece) -> Path {
        let before = self.length();
        let (_, len) = piece.u_at(f64::INFINITY);
        self.pieces.push((piece, before + len));
        self
    }

    /// On straight to `to`.
    pub fn line_to(self, to: (f64, f64)) -> Path {
        let from = self.end();
        self.push(Piece::Line { from, to })
    }

    /// On along a quadratic curve bent toward `via` to `to`.
    pub fn quad_to(self, via: (f64, f64), to: (f64, f64)) -> Path {
        let from = self.end();
        self.push(Piece::Quad { from, via, to })
    }

    /// On round `centre` through `sweep` radians (positive clockwise on screen) at the distance it is from it now.
    pub fn turn(self, centre: (f64, f64), sweep: f64) -> Path {
        let from = self.end();
        let r = libm::hypot(from.0 - centre.0, from.1 - centre.1);
        self.spiral(centre, sweep, r)
    }

    /// On round `centre` through `sweep` radians, its distance from it going from what it is now to `radius`.
    pub fn spiral(self, centre: (f64, f64), sweep: f64, radius: f64) -> Path {
        let from = self.end();
        let r0 = libm::hypot(from.0 - centre.0, from.1 - centre.1);
        let start = libm::atan2(from.1 - centre.1, from.0 - centre.0);
        self.push(Piece::Turn {
            centre,
            start,
            sweep,
            r0,
            r1: radius.max(0.0),
        })
    }

    /// Where it starts.
    pub fn start(&self) -> (f64, f64) {
        self.start
    }

    /// Where it ends.
    pub fn end(&self) -> (f64, f64) {
        self.pieces.last().map_or(self.start, |(p, _)| p.end())
    }

    /// Its pieces, in order.
    pub fn pieces(&self) -> impl Iterator<Item = &Piece> {
        self.pieces.iter().map(|(p, _)| p)
    }

    /// Its length in pixels.
    pub fn length(&self) -> f64 {
        self.pieces.last().map_or(0.0, |&(_, end)| end)
    }

    /// The piece `dist` pixels along it, and how far along that piece.
    fn find(&self, dist: f64) -> Option<(&Piece, f64)> {
        let mut before = 0.0;
        let mut last = None;
        for (piece, end) in &self.pieces {
            if dist <= *end {
                return Some((piece, dist - before));
            }
            // past its end: the last piece, which holds at its end
            last = Some((piece, dist - before));
            before = *end;
        }
        last
    }

    /// The point `dist` pixels along it, held at its ends.
    pub fn at(&self, dist: f64) -> (f64, f64) {
        match self.find(dist.max(0.0)) {
            Some((piece, d)) => piece.point(piece.u_at(d).0),
            None => self.start,
        }
    }

    /// The way it heads `dist` pixels along it, radians (0 along +x, y down); 0 for a path that goes nowhere.
    pub fn heading(&self, dist: f64) -> f64 {
        match self.find(dist.max(0.0)) {
            Some((piece, d)) => piece.heading(piece.u_at(d).0),
            None => 0.0,
        }
    }

    /// The way it heads at its end; 0 for a path that goes nowhere.
    pub fn end_heading(&self) -> f64 {
        self.pieces.last().map_or(0.0, |(p, _)| p.heading(1.0))
    }

    /// A point every `step` pixels along it (at least half a pixel), from its start to its end, both included:
    /// a polyline for anything that takes one (growth laid along it, `crate::growth::Tree::path`).
    pub fn points(&self, step: f64) -> Vec<(f64, f64)> {
        let step = step.max(0.5);
        let len = self.length();
        let n = libm::ceil(len / step).max(1.0) as usize;
        (0..=n)
            .map(|i| self.at(len * i as f64 / n as f64))
            .collect()
    }
}
