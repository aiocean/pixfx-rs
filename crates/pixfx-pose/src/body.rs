//! How long each bone is.

use serde::{Deserialize, Serialize};

/// Bone lengths of a figure, in sprite pixels. [`Body::new`] gives a figure of a height and a head count; change a
/// field afterwards for a figure built otherwise (long legs, a short sword).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Body {
    /// Radius of the head circle.
    pub head: f64,
    /// From the top of the spine to the bottom of the head.
    pub neck: f64,
    /// From the pelvis to the top of the spine.
    pub torso: f64,
    /// From the top of the spine to each shoulder.
    pub shoulder: f64,
    /// From the pelvis to each hip joint.
    pub hip: f64,
    pub upper_arm: f64,
    pub forearm: f64,
    pub thigh: f64,
    /// Knee to the sole of the foot.
    pub shin: f64,
    /// Hand to the tip of a held sword.
    pub sword: f64,
}

impl Body {
    /// A figure `height` pixels tall standing upright, `heads` head-heights tall (about 7.5 for a real adult, 6 for
    /// a game hero, 3 to 4 for a chibi). A head count under 2 counts as 2.
    pub fn new(height: f64, heads: f64) -> Body {
        let heads = if heads.is_finite() {
            heads.max(2.0)
        } else {
            6.0
        };
        let height = if height.is_finite() {
            height.max(1.0)
        } else {
            56.0
        };
        let unit = height / heads;
        // Head and neck take 1.2 heads; the rest splits 4 to 6 between torso and legs.
        let rest = (heads - 1.2) * unit;
        Body {
            head: unit / 2.0,
            neck: unit * 0.2,
            torso: rest * 0.4,
            shoulder: unit * 0.75,
            hip: unit * 0.4,
            upper_arm: rest * 0.25,
            forearm: rest * 0.23,
            thigh: rest * 0.3,
            shin: rest * 0.3,
            sword: height * 0.4,
        }
    }

    /// Height standing upright, head top to soles.
    pub fn height(&self) -> f64 {
        2.0 * self.head + self.neck + self.torso + self.thigh + self.shin
    }
}

impl Default for Body {
    /// A 56-pixel game hero, six heads tall: fills a 64x64 cell.
    fn default() -> Self {
        Body::new(56.0, 6.0)
    }
}
