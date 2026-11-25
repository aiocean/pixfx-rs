//! Heat drawn on connected sprite pixels, shared by the short contact pulse and the full ignition.

use std::collections::{BTreeMap, VecDeque};

use libm::hypot;
use pixfx_core::Actor;
#[cfg(any(feature = "feedback-mask-ignition", feature = "impacts-contact-pulse"))]
use pixfx_core::NumOpt;
#[cfg(any(feature = "feedback-mask-ignition", feature = "impacts-contact-pulse"))]
use pixfx_core::cue::{TimedCue, impact};
#[cfg(any(feature = "feedback-mask-ignition", feature = "impacts-contact-pulse"))]
use pixfx_core::{ColorOpt, Options};
use pixfx_draw::{
    Painter, Rgba,
    color::{WHITE, mix},
};

#[cfg(any(feature = "feedback-mask-ignition", feature = "impacts-contact-pulse"))]
pub(crate) const CONTACT_X: NumOpt = NumOpt::new("contactX", -16.0).within(-64.0, 64.0);
#[cfg(any(feature = "feedback-mask-ignition", feature = "impacts-contact-pulse"))]
pub(crate) const CONTACT_Y: NumOpt = NumOpt::new("contactY", 0.0).within(-64.0, 64.0);
#[cfg(any(feature = "feedback-mask-ignition", feature = "impacts-contact-pulse"))]
pub(crate) const TINT: ColorOpt = ColorOpt::new("color", Rgba::hex24(0xff702c));

#[cfg(any(feature = "feedback-mask-ignition", feature = "impacts-contact-pulse"))]
pub(crate) const IMPACT_AT: f64 = 7.0 / 60.0;

#[cfg(any(feature = "feedback-mask-ignition", feature = "impacts-contact-pulse"))]
pub(crate) fn contact_cues(opts: &Options) -> Vec<TimedCue> {
    impact(
        IMPACT_AT,
        0.58,
        opts.num(&CONTACT_X),
        opts.num(&CONTACT_Y),
        opts,
    )
}

pub(crate) struct MaskHeat {
    contact: (f64, f64),
    tint: Rgba,
    pixels: Vec<(i32, i32, Rgba, u32)>,
    reach: f64,
}

impl MaskHeat {
    #[cfg(any(feature = "feedback-mask-ignition", feature = "impacts-contact-pulse"))]
    pub(crate) fn new(opts: &Options) -> Self {
        let contact = (opts.num(&CONTACT_X), opts.num(&CONTACT_Y));
        let tint = opts
            .palette
            .as_deref()
            .and_then(|ramp| ramp.get(ramp.len() / 2))
            .copied()
            .unwrap_or_else(|| opts.color(&TINT));
        Self::new_at(contact, tint)
    }

    pub(crate) fn new_at(contact: (f64, f64), tint: Rgba) -> Self {
        Self {
            contact,
            tint,
            pixels: Vec::new(),
            reach: 1.0,
        }
    }

    pub(crate) fn read(&mut self, actor: Option<&Actor>, max_distance: Option<u32>) {
        self.pixels.clear();
        self.reach = 1.0;
        let Some(actor) = actor else {
            return;
        };
        let mut unburned: BTreeMap<_, _> = actor.opaque().map(|(x, y, c)| ((x, y), c)).collect();
        let (cx, cy) = self.contact;
        let seed = unburned
            .keys()
            .min_by(|a, b| {
                let distance = |(x, y): (i32, i32)| hypot(f64::from(x) - cx, f64::from(y) - cy);
                distance(**a).total_cmp(&distance(**b))
            })
            .copied();
        let Some((x, y)) = seed else {
            return;
        };
        let Some(color) = unburned.remove(&(x, y)) else {
            return;
        };
        let mut queue = VecDeque::from([(x, y, color, 0u32)]);
        while let Some((x, y, color, distance)) = queue.pop_front() {
            if max_distance.is_some_and(|limit| distance > limit) {
                break;
            }
            self.reach = self.reach.max(f64::from(distance));
            self.pixels.push((x, y, color, distance));
            for (dx, dy) in [(1, 0), (0, 1), (-1, 0), (0, -1)] {
                let (nx, ny) = (x.saturating_add(dx), y.saturating_add(dy));
                if let Some(color) = unburned.remove(&(nx, ny)) {
                    queue.push_back((nx, ny, color, distance + 1));
                }
            }
        }
    }

    #[cfg(feature = "feedback-mask-ignition")]
    pub(crate) fn reach(&self) -> f64 {
        self.reach
    }

    pub(crate) fn draw_front(&self, p: &mut Painter, front: f64, fade: f64) {
        for &(x, y, color, distance) in &self.pixels {
            let since = front - f64::from(distance);
            if since < 0.0 {
                continue;
            }
            let heat = (1.0 - since / 9.0).max(0.0);
            let (x, y) = (f64::from(x), f64::from(y));
            let color = mix(mix(color, self.tint, 0.88), WHITE, heat * 0.82);
            p.each_game_cell(x, y, |p, sx, sy| {
                p.put(sx, sy, color, fade * (0.35 + 0.65 * heat));
                p.add(sx, sy, self.tint, heat * fade);
            });
        }
    }
}
