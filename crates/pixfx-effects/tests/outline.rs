//! The `outline` option on the shipped effects: one that draws its own outline recolours or drops it, any other
//! gets the engine's ring, and unset renders as designed.

use pixfx_core::outline::Outline;
use pixfx_core::{Effect, Frame, Options, Rgba, render_frames};
use pixfx_effects::effect;

/// The effects that draw an outline of their own, and its colour as designed.
const OWN: [(&str, Rgba); 3] = [
    ("explosions/small-explosion", Rgba::hex24(0x1a1422)),
    ("explosions/dirt-explosion", Rgba::hex24(0x1a1422)),
    ("pickups/coin-spin", Rgba::hex24(0x4a2c08)),
];

/// A colour no effect here draws by itself.
const MARK: Rgba = Rgba::hex24(0x00ff00);

fn fx(id: &str) -> Result<&'static dyn Effect, String> {
    effect(id).ok_or(format!("{id} is not registered"))
}

fn frames(id: &str, outline: Option<Outline>) -> Result<Vec<Frame>, String> {
    let opts = Options {
        outline,
        ..Options::default()
    };
    Ok(render_frames(fx(id)?, &opts, 0).frames)
}

/// Opaque-enough pixels of exactly colour `c` over every frame.
fn count(frames: &[Frame], c: Rgba) -> usize {
    let rgb = c.pack() & 0x00ff_ffff;
    frames
        .iter()
        .flat_map(|f| &f.color)
        .filter(|&&w| w >> 24 != 0 && w & 0x00ff_ffff == rgb)
        .count()
}

fn mark() -> Option<Outline> {
    Outline::parse("#00ff00").ok()
}

#[test]
fn the_effects_that_draw_their_own_outline_say_so() -> Result<(), String> {
    for (id, _) in OWN {
        assert!(fx(id)?.draws_outline(), "{id}");
    }
    assert!(!fx("fire/candle")?.draws_outline());
    Ok(())
}

#[test]
fn an_own_outline_takes_the_callers_colour_or_goes() -> Result<(), String> {
    for (id, designed) in OWN {
        let base = frames(id, None)?;
        let had = count(&base, designed);
        assert!(had > 0, "{id} draws its outline in {designed:?}");
        let recoloured = frames(id, mark())?;
        assert!(count(&recoloured, MARK) > 0, "{id} recoloured");
        assert!(
            count(&recoloured, designed) < had,
            "{id}: the outline's pixels moved to the new colour"
        );
        let off = frames(id, Some(Outline::Off))?;
        assert!(count(&off, designed) < had, "{id}: off drops the outline");
        assert_eq!(count(&off, MARK), 0, "{id}: off adds no ring");
    }
    Ok(())
}

#[test]
fn any_other_effect_gets_the_engines_ring() -> Result<(), String> {
    for id in ["fire/candle", "spawn/poof", "holy/feather-burst"] {
        assert_eq!(count(&frames(id, None)?, MARK), 0, "{id} as designed");
        assert!(count(&frames(id, mark())?, MARK) > 0, "{id} ringed");
        assert_eq!(
            frames(id, Some(Outline::Off))?,
            frames(id, None)?,
            "{id}: off is as designed when it has no outline of its own"
        );
    }
    Ok(())
}
