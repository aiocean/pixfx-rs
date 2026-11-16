//! The way a moving instance came: the points it passed, newest first, each with its age, in the space of the
//! effect that draws them (a trail, `pixfx_scene::Scene::trail`). An effect that draws a trace says how many
//! seconds of it it wants ([`crate::Effect::traces`]) and gets it through [`crate::EffectState::set_trace`]: it
//! bends where the mover turned, and is as long as the mover was fast.

/// Points a mover passed, newest first: (`x`, `y`) in the drawing effect's space, turned to `angle` (radians,
/// its own rotation, mirrored with x when the drawing effect is), `age` seconds ago.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Trace {
    pub points: Vec<TracePoint>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TracePoint {
    pub x: f64,
    pub y: f64,
    pub angle: f64,
    pub age: f64,
}

/// A point on a trace, a whole pixel of length from the last: where it is, the way the mover went there (a unit
/// vector), how far back from the newest point, and how long ago the mover passed it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Step {
    pub x: f64,
    pub y: f64,
    pub dx: f64,
    pub dy: f64,
    pub dist: f64,
    pub age: f64,
}

impl Trace {
    /// The trace walked back from its newest point a pixel at a time, up to `secs` old (the last piece cut where
    /// that age falls between two points).
    pub fn steps(&self, secs: f64) -> Vec<Step> {
        let mut out = Vec::new();
        let mut dist = 0.0;
        // how far into the current segment the next step falls
        let mut next = 0.0;
        for pair in self.points.windows(2) {
            let (a, b) = (pair[0], pair[1]);
            if a.age >= secs {
                break;
            }
            let (vx, vy) = (b.x - a.x, b.y - a.y);
            let len = libm::hypot(vx, vy);
            if len < 1e-9 {
                continue;
            }
            // the mover went from b to a: its way is the other way round
            let (dx, dy) = (-vx / len, -vy / len);
            while next <= len {
                let u = next / len;
                let age = a.age + (b.age - a.age) * u;
                if age > secs {
                    return out;
                }
                out.push(Step {
                    x: a.x + vx * u,
                    y: a.y + vy * u,
                    dx,
                    dy,
                    dist: dist + next,
                    age,
                });
                next += 1.0;
            }
            next -= len;
            dist += len;
        }
        out
    }
}
