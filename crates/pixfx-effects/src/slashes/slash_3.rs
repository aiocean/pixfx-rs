use super::painting::{Slash, Swing};

pub static SLASH_3: Slash = Slash::new(
    "slashes/slash-3",
    "Violet Blade Trail",
    "An overhead violet sweep that follows the blade tip when given a movement trace.",
    Swing {
        radii: (1.65, 1.26),
        start: -3.02,
        turn: 3.0,
        tilt: -0.12,
        width: 1.4,
    },
    true,
    [0x441947, 0x943cab, 0xe68bff, 0xffe6f9],
);
