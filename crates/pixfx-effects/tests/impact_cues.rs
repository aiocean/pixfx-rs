//! The effects that hit something ask the game for an impact's answer on the frame they hit, and their `feel`
//! option turns it down or off.

mod built;

use built::built;
use pixfx_core::{Cue, Effect, Instance, OptValue, Options, check_options};

/// Each impact effect and when it hits, in seconds.
const IMPACTS: [(&str, f64); 8] = [
    ("explosions/small-explosion", 0.0),
    ("explosions/dirt-explosion", 0.0),
    ("explosions/plasma-explosion", 0.0),
    ("explosions/implosion", 1.02),
    ("impacts/parry", 0.0),
    ("ice/icicle-fall", 0.6),
    ("spawn/poof", 0.0),
    ("holy/feather-burst", 0.0),
];

/// Seconds into playback of each cue an instance fires, played frame by frame at 60 a second for `secs`.
fn fired(fx: &'static dyn Effect, opts: Options, secs: f64) -> Vec<(f64, Cue)> {
    let mut inst = Instance::new(fx, opts);
    let mut out = Vec::new();
    let mut t = 0.0;
    while t < secs {
        inst.update(1.0 / 60.0);
        t += 1.0 / 60.0;
        out.extend(inst.take_cues().into_iter().map(|c| (c.t, c.cue)));
    }
    out
}

#[test]
fn each_impact_effect_shakes_the_game_when_it_hits() -> Result<(), String> {
    for (id, hit) in IMPACTS {
        let Some(fx) = built(id)? else { continue };
        let cues = fired(fx, Options::default(), 3.0);
        let shake = cues.iter().find(|(_, c)| matches!(c, Cue::Shake { .. }));
        let Some(&(t, _)) = shake else {
            return Err(format!("{id} fired no shake: {cues:?}"));
        };
        // the icicle hits when its fall ends, a little after it snaps
        let late = if id == "ice/icicle-fall" { 0.4 } else { 1e-9 };
        if t < hit - 1e-9 || t > hit + late {
            return Err(format!("{id} shook at {t}, expected {hit}"));
        }
    }
    Ok(())
}

#[test]
fn feel_zero_fires_nothing_and_every_impact_effect_takes_the_option() -> Result<(), String> {
    for (id, _) in IMPACTS {
        let Some(fx) = built(id)? else { continue };
        let mut opts = Options::default();
        opts.extra.insert("feel".into(), OptValue::Num(0.0));
        check_options(fx.meta(), &opts).map_err(|e| format!("{id}: {e}"))?;
        let cues = fired(fx, opts, 3.0);
        if !cues.is_empty() {
            return Err(format!("{id} fired {cues:?} at feel 0"));
        }
    }
    Ok(())
}
