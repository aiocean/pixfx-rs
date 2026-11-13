//! Point lists for lightning (pixelfx `shapes.js`).

use crate::math::Rng;

/// A jagged lightning path from `(x0, y0)` to `(x1, y1)` by midpoint displacement. `jag` is how far a midpoint may
/// move sideways, as a share of the segment's length.
pub fn bolt(
    r: &mut Rng,
    x0: f64,
    y0: f64,
    x1: f64,
    y1: f64,
    jag: f64,
    depth: u32,
) -> Vec<(f64, f64)> {
    let mut pts = vec![(x0, y0), (x1, y1)];
    for _ in 0..depth {
        let mut next = Vec::with_capacity(pts.len() * 2 - 1);
        next.push(pts[0]);
        for pair in pts.windows(2) {
            let ((ax, ay), (bx, by)) = (pair[0], pair[1]);
            let (dx, dy) = (bx - ax, by - ay);
            let l = libm::hypot(dx, dy);
            let off = (r.next() * 2.0 - 1.0) * jag * l;
            let l = if l == 0.0 || l.is_nan() { 1.0 } else { l };
            next.push((
                (ax + bx) / 2.0 + (-dy / l) * off,
                (ay + by) / 2.0 + (dx / l) * off,
            ));
            next.push(pair[1]);
        }
        pts = next;
    }
    pts
}

/// Forks off a bolt path: `count` branches, each up to `len` px long. Returns a list of paths.
pub fn forks(
    r: &mut Rng,
    path: &[(f64, f64)],
    count: usize,
    len: f64,
    jag: f64,
    spread: f64,
) -> Vec<Vec<(f64, f64)>> {
    let mut out = Vec::with_capacity(count);
    for _ in 0..count {
        let i = 1 + libm::floor(r.next() * (path.len() as f64 - 2.0)) as usize;
        let (Some(&(x, y)), Some(&(px, py))) = (path.get(i), path.get(i - 1)) else {
            break;
        };
        let a = libm::atan2(y - py, x - px)
            + (if r.next() < 0.5 { -1.0 } else { 1.0 }) * spread * (0.4 + 0.6 * r.next());
        let l = len * (0.4 + 0.6 * r.next());
        out.push(bolt(
            r,
            x,
            y,
            x + libm::cos(a) * l,
            y + libm::sin(a) * l,
            jag,
            3,
        ));
    }
    out
}
