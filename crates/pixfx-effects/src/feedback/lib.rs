//! Fracture geometry and painting shared by the sprite shatter and shield effects.

use libm::{atan2, cos, hypot, sin};
use pixfx_core::cue::{Cue, FEEL, TimedCue, impact};
use pixfx_core::{Actor, NumOpt, Options};
use pixfx_draw::{
    Painter, Rgba,
    color::{WHITE, mix},
    ruler::{RULER, Ruler},
};

pub(super) const CONTACT_X: NumOpt = NumOpt::new("contactX", 0.0).within(-64.0, 64.0);
pub(super) const CONTACT_Y: NumOpt = NumOpt::new("contactY", 0.0).within(-64.0, 64.0);
pub(super) const FRACTURE_SECS: f64 = 0.65;
const BREAK_AT: f64 = 0.28;

pub(super) fn fracture_cues(opts: &Options, breaks: bool) -> Vec<TimedCue> {
    let x = opts.num(&CONTACT_X);
    let y = opts.num(&CONTACT_Y);
    // Frame 18 is the first frame where the fragments visibly separate.
    let (t, weight) = if breaks {
        (18.0 / 60.0, 0.76)
    } else {
        (1.0 / 60.0, 0.18)
    };
    let mut cues = impact(t, weight, x, y, opts);
    if !cues.is_empty() {
        cues.push(TimedCue {
            t,
            cue: Cue::Sound {
                name: if breaks {
                    "sprite-shatter"
                } else {
                    "shield-crack"
                },
                gain: (if breaks { 0.82 } else { 0.36 }) * opts.num(&FEEL),
                pitch: 1.0,
                len: None,
                x: Some(x),
            },
            pan: None,
        });
    }
    cues
}

struct Pixel {
    x: f64,
    y: f64,
    color: Rgba,
    distance: f64,
    crack: bool,
    velocity: (f64, f64),
}

pub(super) struct Fracture {
    requested: (f64, f64),
    phase: f64,
    pixels: Vec<Pixel>,
    reach: f64,
}

impl Fracture {
    pub(super) fn new(opts: &Options, seed: u64) -> Self {
        Self {
            requested: (opts.num(&CONTACT_X), opts.num(&CONTACT_Y)),
            phase: (seed % 360) as f64 * 0.017453292519943295,
            pixels: Vec::new(),
            reach: 1.0,
        }
    }

    pub(super) fn set_actor(&mut self, actor: Option<&Actor>) {
        self.pixels.clear();
        self.reach = 1.0;
        let Some(actor) = actor else {
            return;
        };
        let (rx, ry) = self.requested;
        let origin = actor.opaque().min_by(|a, b| {
            let distance = |x, y| hypot(f64::from(x) - rx, f64::from(y) - ry);
            distance(a.0, a.1).total_cmp(&distance(b.0, b.1))
        });
        let Some((ox, oy, _)) = origin else {
            return;
        };
        let (ox, oy) = (f64::from(ox), f64::from(oy));
        for (x, y, color) in actor.opaque() {
            let (x, y) = (f64::from(x), f64::from(y));
            let distance = hypot(x - ox, y - oy);
            let spokes = (atan2(y - oy, x - ox) + self.phase) * 1.1 + 0.22 * sin(distance * 0.46);
            let crack = sin(spokes * 3.0).abs() * distance < 2.0
                || (cos(spokes * 3.0).abs() > 0.85
                    && (distance - 10.0 - 2.0 * sin(spokes)).abs() < 0.65);
            let sector = libm::floor(spokes * 3.0 / std::f64::consts::PI);
            let direction = (sector + 0.5) * std::f64::consts::PI / 3.3 - self.phase;
            self.reach = self.reach.max(distance);
            self.pixels.push(Pixel {
                x,
                y,
                color,
                distance,
                crack,
                velocity: (cos(direction), sin(direction)),
            });
        }
    }

    pub(super) fn draw_fragments(&self, p: &mut Painter, t: f64) {
        let progress = (t / BREAK_AT).min(1.0);
        let age = fragment_age(t);
        let reach = f64::from(Ruler::size(RULER.blade, 1.15));
        for px in &self.pixels {
            let color = if px.crack && px.distance <= self.reach * progress {
                mix(px.color, WHITE, (0.86 - age).max(0.0))
            } else {
                px.color
            };
            p.paint_cell(
                px.x + px.velocity.0 * reach * age,
                px.y + px.velocity.1 * reach * age + 11.0 * age * age,
                color,
                1.0 - age,
            );
        }
    }

    #[cfg(feature = "feedback-shield-crack")]
    pub(super) fn draw_cracks(&self, p: &mut Painter, t: f64) {
        let progress = (t / BREAK_AT).min(1.0);
        let age = fragment_age(t);
        for px in &self.pixels {
            if px.crack && px.distance <= self.reach * progress {
                let flash = (1.0 - (self.reach * progress - px.distance) / 9.0).clamp(0.0, 1.0);
                let color = mix(Rgba::hex24(0x201927), WHITE, flash);
                p.each_game_cell(px.x, px.y, |p, sx, sy| {
                    p.put(sx, sy, color, 1.0 - age);
                    p.add(sx, sy, WHITE, flash * (1.0 - age) * 0.7);
                });
            }
        }
    }
}

fn fragment_age(t: f64) -> f64 {
    ((t - BREAK_AT) / (FRACTURE_SECS - BREAK_AT)).max(0.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use pixfx_core::OptValue;

    #[test]
    fn shatter_freezes_and_flashes_when_the_fragments_separate() {
        let mut opts = Options::default();
        opts.extra.insert("contactX".into(), OptValue::Num(12.0));
        let cues = fracture_cues(&opts, true);
        assert!(cues.iter().all(|cue| cue.t == 18.0 / 60.0));
        assert!(cues.iter().any(|cue| matches!(cue.cue, Cue::Shake { .. })));
        assert!(
            cues.iter()
                .any(|cue| matches!(cue.cue, Cue::Hitstop { .. }))
        );
        assert!(cues.iter().any(|cue| matches!(cue.cue, Cue::Flash { .. })));
        assert!(
            cues.iter()
                .any(|cue| matches!(cue.cue, Cue::Ripple { x: 12.0, .. }))
        );
        assert!(cues.iter().any(|cue| matches!(
            cue.cue,
            Cue::Sound {
                name: "sprite-shatter",
                ..
            }
        )));
        assert!(
            fracture_cues(&opts, false)
                .iter()
                .all(|cue| !matches!(cue.cue, Cue::Hitstop { .. } | Cue::Flash { .. }))
        );
        opts.extra.insert("feel".into(), OptValue::Num(0.0));
        assert!(fracture_cues(&opts, true).is_empty());
    }
}
