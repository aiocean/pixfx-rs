//! Impacts: ports of pixelfx `src/fx/impacts`. [`lib`] is the shared `_lib.js`.

pub mod lib;

#[cfg(feature = "impacts-parry")]
mod parry;

#[cfg(feature = "impacts-parry")]
pub use parry::PARRY;

#[cfg(feature = "impacts-rune-flash")]
mod rune_flash;

#[cfg(feature = "impacts-rune-flash")]
pub use rune_flash::RUNE_FLASH;

#[cfg(feature = "impacts-thunderstrike")]
mod thunderstrike;

#[cfg(feature = "impacts-thunderstrike")]
pub use thunderstrike::THUNDERSTRIKE;

#[cfg(feature = "impacts-aftershock")]
mod aftershock;
#[cfg(feature = "impacts-aftershock")]
pub use aftershock::AFTERSHOCK;

#[cfg(feature = "impacts-charged-pulse")]
mod charged_pulse;
#[cfg(feature = "impacts-charged-pulse")]
pub use charged_pulse::CHARGED_PULSE;

#[cfg(feature = "impacts-contact-pulse")]
mod contact_pulse;
#[cfg(feature = "impacts-contact-pulse")]
pub use contact_pulse::CONTACT_PULSE;

#[cfg(feature = "impacts-directional-wave")]
mod directional_wave;
#[cfg(feature = "impacts-directional-wave")]
pub use directional_wave::DIRECTIONAL_WAVE;

#[cfg(feature = "impacts-resonance")]
mod resonance;
#[cfg(feature = "impacts-resonance")]
pub use resonance::RESONANCE;

#[cfg(any(
    feature = "impacts-aftershock",
    feature = "impacts-charged-pulse",
    feature = "impacts-directional-wave",
    feature = "impacts-resonance",
))]
mod wave;
