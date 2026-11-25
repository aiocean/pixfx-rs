use super::painting::{Slash, Swing};

pub static SLASH_1: Slash = Slash::new(
    "slashes/slash-1",
    "Ember Cleave",
    "A wide diagonal cut leaves a tapered ribbon of hot embers.",
    Swing {
        radii: (1.75, 0.79),
        start: 2.87,
        turn: -3.45,
        tilt: -0.54,
        width: 1.5,
    },
    false,
    [0x781b29, 0xdd4930, 0xffa046, 0xffeed0],
);
