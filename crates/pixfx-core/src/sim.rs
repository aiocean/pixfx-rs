//! Port of the parts of pixelfx `src/core/sim.js` that the ported effects use.
//!
//! Simulated particles, debris and lights, for effects that want real physics: sparks that streak and bounce
//! off the floor, shards that tumble and settle, vapour and smoke that swell and thin, lights that flare and die.
//! Adapted from Sprite Fusion's open-source pixel art VFX (https://www.spritefusion.com/pixel-art-vfx),
//! MIT License, Copyright (c) 2026 Hugo Duprez. Units here are art pixels and seconds; draw through a Painter,
//! so effects stay in local space and mirror or rotate like any other.

use crate::color::Rgba;

/// Smoke from fresh to thin: the ramp smoke puffs walk down
/// (pixelfx `["#7b6f72", "#5d5660", "#433d48", "#2b2630", "#1a171e"].map(hex)`).
pub const SMOKE_RAMP: [Rgba; 5] = [
    Rgba(0x7b, 0x6f, 0x72, 255),
    Rgba(0x5d, 0x56, 0x60, 255),
    Rgba(0x43, 0x3d, 0x48, 255),
    Rgba(0x2b, 0x26, 0x30, 255),
    Rgba(0x1a, 0x17, 0x1e, 255),
];
