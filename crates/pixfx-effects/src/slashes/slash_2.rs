use super::painting::{Slash, Swing};

pub static SLASH_2: Slash = Slash::new(
    "slashes/slash-2",
    "Frost Hook",
    "A tight upward sweep leaves a tall blue crescent and a pale cutting edge.",
    Swing {
        radii: (0.85, 1.72),
        start: 1.72,
        turn: -3.31,
        tilt: 0.25,
        width: 1.1,
    },
    false,
    [0x193976, 0x2b84cb, 0x69def2, 0xe3fbff],
);
