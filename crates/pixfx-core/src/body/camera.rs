//! Where the viewer stands: turned round the figure (yaw) and looking down at her (pitch). Orthographic, so a
//! figure seen from above is shorter on screen through foreshortening, never rescaled.

use serde::{Deserialize, Serialize};

/// A camera in degrees. `yaw` is the figure's facing as [`super::facing`] names it: 0 faces screen right (her right
/// side toward the viewer), 90 faces the viewer, 180 screen left, 270 her back. `pitch` 0 is a side view, 90 looks
/// straight down.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Camera {
    pub yaw: f64,
    pub pitch: f64,
}

/// Pitch of an isometric view.
pub const ISO_PITCH: f64 = 30.0;
/// Pitch of a top-down view: the three-quarter look of a top-down RPG's sprites.
pub const TOPDOWN_PITCH: f64 = 45.0;

impl Camera {
    /// A side view (a platformer's) facing one of four ways.
    pub fn side(dir: Dir4) -> Camera {
        Camera {
            yaw: dir.yaw(),
            pitch: 0.0,
        }
    }
    /// An isometric view facing one of eight ways.
    pub fn iso(dir: Dir8) -> Camera {
        Camera {
            yaw: dir.yaw(),
            pitch: ISO_PITCH,
        }
    }
    /// A top-down view (an RPG's) facing one of eight ways.
    pub fn topdown(dir: Dir8) -> Camera {
        Camera {
            yaw: dir.yaw(),
            pitch: TOPDOWN_PITCH,
        }
    }
}

/// Four facings: east is screen right, south toward the viewer.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Dir4 {
    E,
    S,
    W,
    N,
}

impl Dir4 {
    pub fn yaw(self) -> f64 {
        match self {
            Dir4::E => 0.0,
            Dir4::S => 90.0,
            Dir4::W => 180.0,
            Dir4::N => 270.0,
        }
    }
    pub fn all() -> [Dir4; 4] {
        [Dir4::E, Dir4::S, Dir4::W, Dir4::N]
    }
    pub fn name(self) -> &'static str {
        match self {
            Dir4::E => "E",
            Dir4::S => "S",
            Dir4::W => "W",
            Dir4::N => "N",
        }
    }
}

/// Eight facings, clockwise on screen from east.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Dir8 {
    E,
    SE,
    S,
    SW,
    W,
    NW,
    N,
    NE,
}

impl Dir8 {
    pub fn yaw(self) -> f64 {
        match self {
            Dir8::E => 0.0,
            Dir8::SE => 45.0,
            Dir8::S => 90.0,
            Dir8::SW => 135.0,
            Dir8::W => 180.0,
            Dir8::NW => 225.0,
            Dir8::N => 270.0,
            Dir8::NE => 315.0,
        }
    }
    pub fn all() -> [Dir8; 8] {
        [
            Dir8::E,
            Dir8::SE,
            Dir8::S,
            Dir8::SW,
            Dir8::W,
            Dir8::NW,
            Dir8::N,
            Dir8::NE,
        ]
    }
    pub fn name(self) -> &'static str {
        match self {
            Dir8::E => "E",
            Dir8::SE => "SE",
            Dir8::S => "S",
            Dir8::SW => "SW",
            Dir8::W => "W",
            Dir8::NW => "NW",
            Dir8::N => "N",
            Dir8::NE => "NE",
        }
    }
}
